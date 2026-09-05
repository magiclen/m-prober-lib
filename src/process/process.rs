use std::{
    collections::BTreeMap,
    fs,
    hash::{Hash, Hasher},
    mem::take,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use chrono::prelude::*;

use crate::{
    Error,
    btime::get_btime,
    cpu::get_average_cpu_stat,
    process::{
        ProcessFilter, ProcessStat, ProcessState, ProcessTimeStat, get_process_stat,
        get_process_status, get_process_time_stat, process_stat::get_process_ppid,
    },
    utils::clock_ticks_to_duration,
};

/// One running process. Two instances are equal when their PIDs are equal.
#[derive(Debug, Clone, Eq)]
pub struct Process {
    /// The ID of this process.
    pub pid:                u32,
    /// The effective user ID.
    pub effective_uid:      u32,
    /// The effective group ID.
    pub effective_gid:      u32,
    /// The state of the process.
    pub state:              ProcessState,
    /// The ID of the parent process.
    pub ppid:               u32,
    /// The program name, which is the `comm` field of the `/proc/PID/stat` file.
    pub program:            String,
    /// The command line with its NUL separators replaced by spaces. It is empty for a kernel thread.
    pub cmdline:            String,
    /// The path of the executable. It is `None` for kernel threads or when the permission is denied.
    /// The kernel appends ` (deleted)` to the path when the executable file has been removed or replaced.
    pub exe:                Option<PathBuf>,
    /// The name of the controlling terminal, e.g. `pts/0`. It is `None` when there is none.
    pub tty:                Option<String>,
    /// The scheduling priority.
    pub priority:           i8,
    /// The real-time scheduling priority. It is `None` for a process not running under a real-time policy.
    pub real_time_priority: Option<u8>,
    /// The nice value, from `-20` (high priority) to `19` (low priority).
    pub nice:               i8,
    /// The number of threads in this process.
    pub threads:            usize,
    /// Virtual Set Size (VIRT)
    pub vsz:                usize,
    /// Resident Set Size (RES)
    pub rss:                usize,
    /// Resident Shared Size (SHR)
    pub rss_shared:         usize,
    /// Resident Anonymous Memory
    pub rss_anon:           usize,
    /// The swapped-out memory size in bytes (`VmSwap`).
    pub swap:               usize,
    /// The time this process started, computed from the boot time.
    pub start_time:         DateTime<Utc>,
}

impl Hash for Process {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pid.hash(state)
    }
}

impl PartialEq for Process {
    #[inline]
    fn eq(&self, other: &Process) -> bool {
        self.pid.eq(&other.pid)
    }
}

// This enum is short-lived, so boxing the matched process would only add an allocation per process.
#[allow(clippy::large_enum_variant)]
enum ProcessProbe {
    Matched(Process, ProcessStat),
    /// The parent PID is only looked up when a PID filter needs it for finding descendants.
    Filtered(Option<u32>),
}

/// Convert the content of `/proc/PID/cmdline` to a string with the arguments separated by spaces.
fn cmdline_to_string(mut data: Vec<u8>) -> String {
    // Every argument ends with a NUL, so the last one is dropped instead of becoming a trailing space.
    if data.last() == Some(&0) {
        data.pop();
    }

    for e in data.iter_mut() {
        if *e == 0 {
            *e = b' ';
        }
    }

    // Command line arguments are arbitrary bytes, so they may not be valid UTF-8.
    String::from_utf8_lossy(&data).into_owned()
}

/// Check whether an error means that the process exited during the scan.
/// The kernel returns `ENOENT` when the `/proc/PID` folder is gone and `ESRCH` when a file in it is read after the process was reaped.
#[inline]
fn is_process_gone(err: &Error) -> bool {
    if let Error::IOError(err) = err {
        matches!(err.raw_os_error(), Some(libc::ENOENT | libc::ESRCH))
    } else {
        false
    }
}

fn get_process_with_stat_inner<P: AsRef<Path>>(
    pid: u32,
    process_path: P,
    process_filter: &ProcessFilter,
    btime: DateTime<Utc>,
) -> Result<ProcessProbe, Error> {
    let process_path = process_path.as_ref();

    let status = get_process_status(pid)?;

    let uid_filtered = process_filter.uid_filter.is_some_and(|uid_filter| {
        status.real_uid != uid_filter
            && status.effective_uid != uid_filter
            && status.saved_set_uid != uid_filter
            && status.fs_uid != uid_filter
    });

    let gid_filtered = process_filter.gid_filter.is_some_and(|gid_filter| {
        status.real_gid != gid_filter
            && status.effective_gid != gid_filter
            && status.saved_set_gid != gid_filter
            && status.fs_gid != gid_filter
    });

    if uid_filtered || gid_filtered {
        let ppid =
            if process_filter.pid_filter.is_some() { Some(get_process_ppid(pid)?) } else { None };

        return Ok(ProcessProbe::Filtered(ppid));
    }

    let cmdline = cmdline_to_string(fs::read(process_path.join("cmdline"))?);

    let program_filter_match = process_filter
        .program_filter
        .as_ref()
        .is_none_or(|program_filter| program_filter.is_match(&cmdline));

    let mut stat = get_process_stat(pid)?;

    if !program_filter_match
        && let Some(program_filter) = process_filter.program_filter.as_ref()
        && !program_filter.is_match(&stat.comm)
    {
        return Ok(ProcessProbe::Filtered(Some(stat.ppid)));
    }

    let effective_uid = status.effective_uid;
    let effective_gid = status.effective_gid;
    let state = stat.state;
    let ppid = stat.ppid;
    let program = take(&mut stat.comm);

    let tty = {
        match stat.tty_nr_major {
            4 => {
                if stat.tty_nr_minor < 64 {
                    Some(format!("tty{}", stat.tty_nr_minor))
                } else {
                    Some(format!("ttyS{}", stat.tty_nr_minor - 64))
                }
            },
            136..=143 => Some(format!("pts/{}", stat.tty_nr_minor)),
            _ => None,
        }
    };

    if let Some(tty_filter) = process_filter.tty_filter.as_ref() {
        match tty.as_ref() {
            Some(tty) => {
                if !tty_filter.is_match(tty) {
                    return Ok(ProcessProbe::Filtered(Some(ppid)));
                }
            },
            None => return Ok(ProcessProbe::Filtered(Some(ppid))),
        }
    }

    let exe = fs::read_link(process_path.join("exe")).ok();

    let priority = stat.priority;
    let real_time_priority = if stat.rt_priority > 0 { Some(stat.rt_priority) } else { None };
    let nice = stat.nice;
    let threads = stat.num_threads;
    let vsz = stat.vsize;
    let rss = stat.rss;
    let rss_shared = stat.shared;
    let rss_anon = stat.rss_anon;
    let swap = status.vm_swap;

    // `starttime` is in clock ticks since boot, not in milliseconds.
    let start_time = btime + clock_ticks_to_duration(stat.starttime);

    let process = Process {
        pid,
        effective_uid,
        effective_gid,
        state,
        ppid,
        program,
        cmdline,
        exe,
        tty,
        priority,
        real_time_priority,
        nice,
        threads,
        vsz,
        rss,
        rss_shared,
        rss_anon,
        swap,
        start_time,
    };

    Ok(ProcessProbe::Matched(process, stat))
}

/// Get information of a specific process found by ID by reading files in the `/proc/PID` folder.
///
/// ```rust
/// use mprober_lib::process;
///
/// let (process, _) = process::get_process_with_stat(1).unwrap();
///
/// println!("{process:#?}");
/// ```
#[inline]
pub fn get_process_with_stat(pid: u32) -> Result<(Process, ProcessStat), Error> {
    let process_path = Path::new("/proc").join(pid.to_string());

    match get_process_with_stat_inner(pid, process_path, &ProcessFilter::default(), get_btime())? {
        ProcessProbe::Matched(process, stat) => Ok((process, stat)),
        ProcessProbe::Filtered(_) => unreachable!("the default filter matches every process"),
    }
}

/// Check whether `pid` is `ancestor` itself or one of its descendants by walking up the parent chain.
fn is_process_or_descendant(pid: u32, ancestor: u32, pid_ppid_map: &BTreeMap<u32, u32>) -> bool {
    let mut current = pid;

    // The chain normally ends at PID 0, which is not in the map, but the step count is capped in case PIDs were reused during the scan.
    for _ in 0..=pid_ppid_map.len() {
        if current == ancestor {
            return true;
        }

        match pid_ppid_map.get(&current) {
            Some(&ppid) => current = ppid,
            None => return false,
        }
    }

    false
}

/// Get process information by reading files in the `/proc/PID` folders. When `pid_filter` is set, the process and all of its descendants are returned.
///
/// ```rust
/// use mprober_lib::process;
///
/// let processes_with_stat =
///     process::get_processes_with_stat(&process::ProcessFilter::default())
///         .unwrap();
///
/// println!("{processes_with_stat:#?}");
/// ```
pub fn get_processes_with_stat(
    process_filter: &ProcessFilter,
) -> Result<Vec<(Process, ProcessStat)>, Error> {
    let mut processes_with_stat = Vec::new();

    // Every scanned process is recorded here (even the filtered ones), so descendants can be found no matter the scanning order.
    let mut pid_ppid_map: BTreeMap<u32, u32> = BTreeMap::new();

    // The boot time is computed once, so every process in this scan uses the same value.
    let btime = get_btime();

    for dir_entry in Path::new("/proc").read_dir()? {
        let dir_entry = dir_entry?;

        let Some(pid) = dir_entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };

        match get_process_with_stat_inner(pid, dir_entry.path(), process_filter, btime) {
            Ok(ProcessProbe::Matched(process, stat)) => {
                if process_filter.pid_filter.is_some() {
                    pid_ppid_map.insert(pid, process.ppid);
                }

                processes_with_stat.push((process, stat));
            },
            Ok(ProcessProbe::Filtered(ppid)) => {
                if let Some(ppid) = ppid {
                    pid_ppid_map.insert(pid, ppid);
                }
            },
            Err(err) => {
                if is_process_gone(&err) {
                    continue;
                }

                return Err(err);
            },
        }
    }

    if let Some(pid_filter) = process_filter.pid_filter {
        processes_with_stat.retain(|(process, _)| {
            is_process_or_descendant(process.pid, pid_filter, &pid_ppid_map)
        });
    }

    Ok(processes_with_stat)
}

/// Get process information by reading files in the `/proc/PID` folders and measure the cpu utilization in percentage within a specific time interval. If the number it returns is `1.0`, means `100%`.
///
/// ```rust
/// use std::{thread::sleep, time::Duration};
///
/// use mprober_lib::process;
///
/// let processes_with_cpu_percentage =
///     process::get_processes_with_cpu_utilization_in_percentage(
///         &process::ProcessFilter::default(),
///         Duration::from_millis(100),
///     )
///     .unwrap();
///
/// for (process, cpu_percentage) in processes_with_cpu_percentage {
///     println!("{}: {:.1}%", process.pid, cpu_percentage * 100.0);
/// }
/// ```
pub fn get_processes_with_cpu_utilization_in_percentage(
    process_filter: &ProcessFilter,
    interval: Duration,
) -> Result<Vec<(Process, f64)>, Error> {
    let pre_average_cpu_stat = get_average_cpu_stat()?;
    let processes_with_stat = get_processes_with_stat(process_filter)?;

    let mut processes_with_cpu_percentage = Vec::with_capacity(processes_with_stat.len());

    sleep(interval);

    let average_cpu_stat = get_average_cpu_stat()?;

    let total_cpu_time_f64 = {
        let pre_average_cpu_time = pre_average_cpu_stat.compute_cpu_time();
        let average_cpu_time = average_cpu_stat.compute_cpu_time();

        average_cpu_time.get_total_time().saturating_sub(pre_average_cpu_time.get_total_time())
            as f64
    };

    for (process, pre_process_stat) in processes_with_stat {
        if let Ok(process_time_stat) = get_process_time_stat(process.pid) {
            let pre_process_time_stat: ProcessTimeStat = pre_process_stat.into();

            let cpu_percentage = pre_process_time_stat
                .compute_cpu_utilization_in_percentage(&process_time_stat, total_cpu_time_f64);

            processes_with_cpu_percentage.push((process, cpu_percentage));
        }
    }

    Ok(processes_with_cpu_percentage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmdline_to_string_drops_trailing_nul() {
        assert_eq!("sleep 100", cmdline_to_string(b"sleep\x00100\x00".to_vec()));
        assert_eq!("", cmdline_to_string(Vec::new()));

        // A process that rewrote its argv may leave no trailing NUL.
        assert_eq!("postgres: writer", cmdline_to_string(b"postgres: writer".to_vec()));
    }
}
