use std::{
    collections::BTreeMap,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use chrono::prelude::*;

use crate::{
    Error,
    btime::get_btime,
    cpu::{compute_elapsed_cpu_time, get_average_cpu_stat},
    process::{
        ProcessFilter, ProcessStat, ProcessState, ProcessTimeStat, get_process_time_stat,
        process_stat::{
            get_process_ppid, get_process_stat_without_memory, read_process_statm_file,
        },
        process_status::read_process_status,
    },
    utils::{clock_ticks_to_duration, proc_pid_path, read_link_name, read_single_record_file_into},
};

/// The names of the terminal devices that were already looked up in one scan. Every lookup is a `readlink` in sysfs, and a machine full of processes normally has only a handful of terminals.
type TtyCache = BTreeMap<(u16, u32), Option<String>>;

/// One running process. Two instances are equal when their PIDs are equal.
#[derive(Debug, Clone, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// The program name, which is the `comm` field of the `/proc/PID/stat` file. The kernel stores it in `TASK_COMM_LEN` bytes, so it is at most 15 characters long and a longer executable name is cut short; `cmdline` and `exe` carry the full name. A thread that renamed itself (e.g. with `prctl(PR_SET_NAME)`) reports that name instead of the executable.
    pub program:            String,
    /// The command line with its NUL separators replaced by spaces. It is empty for a kernel thread.
    pub cmdline:            String,
    /// The path of the executable. It is `None` for kernel threads or when the permission is denied.
    /// The kernel appends ` (deleted)` to the path when the executable file has been removed or replaced.
    pub exe:                Option<PathBuf>,
    /// The name of the controlling terminal, e.g. `pts/0`, `tty1` or `ttyS0`. It is `None` when there is none.
    pub tty:                Option<String>,
    /// The scheduling priority.
    pub priority:           i8,
    /// The real-time scheduling priority. It is `None` for a process not running under a real-time policy.
    pub real_time_priority: Option<u8>,
    /// The nice value, from `-20` (high priority) to `19` (low priority).
    pub nice:               i8,
    /// The number of threads in this process.
    pub threads:            usize,
    /// The virtual memory size in bytes (VIRT).
    pub vsz:                u64,
    /// The resident set size in bytes (RES).
    pub rss:                u64,
    /// The resident shared size in bytes (SHR).
    pub rss_shared:         u64,
    /// The resident anonymous memory in bytes.
    pub rss_anon:           u64,
    /// The swapped-out memory size in bytes (`VmSwap`).
    pub swap:               u64,
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

/// Convert the content of `/proc/PID/cmdline` to a string with the arguments separated by spaces.
fn cmdline_to_string(data: &[u8]) -> String {
    // Every argument ends with a NUL, so the last one is dropped instead of becoming a trailing space.
    let data = data.strip_suffix(b"\0").unwrap_or(data);

    // Command line arguments are arbitrary bytes, so they may not be valid UTF-8.
    String::from_utf8_lossy(data).replace('\0', " ")
}

/// Get the name of a terminal device from its major and minor numbers, e.g. `pts/0` or `ttyS0`.
fn tty_name(major: u16, minor: u32) -> Option<String> {
    match major {
        0 => None,
        // Pseudo terminals have no sysfs entries, but their names follow a fixed rule.
        136..=143 => Some(format!("pts/{minor}")),
        _ => {
            // The kernel links every character device to its sysfs folder, whose name is the device name (e.g. `ttyS0`, `ttyUSB0`, `ttyAMA0` or `hvc0`).
            let name = read_link_name(format!("/sys/dev/char/{major}:{minor}"));

            // Without sysfs, at least the classic serial and virtual consoles can still be named.
            name.or_else(|| match major {
                4 if minor < 64 => Some(format!("tty{minor}")),
                4 => Some(format!("ttyS{}", minor - 64)),
                _ => None,
            })
        },
    }
}

/// Check whether an error means that a process cannot contribute to the scan, so that the scan goes on with the other processes instead of failing.
/// The kernel returns `ENOENT` when the `/proc/PID` folder is gone and `ESRCH` when a file in it is read after the process was reaped, and `EACCES` when `/proc` is mounted with `hidepid`, which hides the processes of other users.
#[inline]
fn is_process_unreadable(err: &Error) -> bool {
    if let Error::IOError(err) = err {
        matches!(err.raw_os_error(), Some(libc::ENOENT | libc::ESRCH | libc::EACCES))
    } else {
        false
    }
}

fn get_process_with_stat_inner(
    pid: u32,
    process_path: &Path,
    process_filter: &ProcessFilter,
    btime: DateTime<Utc>,
    tty_cache: &mut TtyCache,
    buffer: &mut Vec<u8>,
) -> Result<Option<(Process, ProcessStat)>, Error> {
    // Only the IDs and the swap size are used here, and both come before the `Threads:` line.
    let status = read_process_status(process_path, true, buffer)?;

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
        return Ok(None);
    }

    // The kernel writes the whole command line in one record, and a longer one is still read completely.
    read_single_record_file_into(process_path.join("cmdline"), 512, buffer)?;

    let cmdline = cmdline_to_string(buffer);

    let program_filter_match =
        process_filter.program_filter.is_none_or(|program_filter| program_filter(&cmdline));

    // The memory fields live in a separate file, which a process that is dropped right here does not need.
    let mut stat = get_process_stat_without_memory(process_path, buffer)?;

    if !program_filter_match
        && let Some(program_filter) = process_filter.program_filter
        && !program_filter(&stat.comm)
    {
        return Ok(None);
    }

    let effective_uid = status.effective_uid;
    let effective_gid = status.effective_gid;
    let state = stat.state;
    let ppid = stat.ppid;
    // The name is cloned rather than taken, so that the `ProcessStat` returned alongside keeps its `comm`.
    let program = stat.comm.clone();

    let tty = tty_cache
        .entry((stat.tty_nr_major, stat.tty_nr_minor))
        .or_insert_with(|| tty_name(stat.tty_nr_major, stat.tty_nr_minor))
        .clone();

    if let Some(tty_filter) = process_filter.tty_filter {
        match tty.as_ref() {
            Some(tty) => {
                if !tty_filter(tty) {
                    return Ok(None);
                }
            },
            None => return Ok(None),
        }
    }

    let exe = fs::read_link(process_path.join("exe")).ok();

    // The process survived every filter, so the memory fields are worth the extra read now.
    read_process_statm_file(process_path, &mut stat, buffer)?;

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

    Ok(Some((process, stat)))
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
    let process_path = proc_pid_path(pid);

    let mut tty_cache = TtyCache::new();

    let mut buffer = Vec::new();

    match get_process_with_stat_inner(
        pid,
        &process_path,
        &ProcessFilter::default(),
        get_btime(),
        &mut tty_cache,
        &mut buffer,
    )? {
        Some(process_with_stat) => Ok(process_with_stat),
        None => unreachable!("the default filter matches every process"),
    }
}

/// Get the current working directory of a specific process found by ID by reading the `/proc/PID/cwd` link. Reading the link of a process owned by another user needs the `CAP_SYS_PTRACE` capability, otherwise a `PermissionDenied` error is returned. A kernel thread shares the working directory of the kernel, which is normally `/`, while a zombie process has none any more, so a `NotFound` error is returned for one.
///
/// ```rust
/// use mprober_lib::process;
///
/// let cwd = process::get_process_cwd(std::process::id()).unwrap();
///
/// println!("{}", cwd.display());
/// ```
#[inline]
pub fn get_process_cwd(pid: u32) -> Result<PathBuf, Error> {
    Ok(fs::read_link(proc_pid_path(pid).join("cwd"))?)
}

/// Get the root directory of a specific process found by ID by reading the `/proc/PID/root` link. It is `/` unless the process was put into a `chroot` or a mount namespace of its own. Reading the link of a process owned by another user needs the `CAP_SYS_PTRACE` capability, otherwise a `PermissionDenied` error is returned.
///
/// ```rust
/// use mprober_lib::process;
///
/// let root = process::get_process_root(std::process::id()).unwrap();
///
/// println!("{}", root.display());
/// ```
#[inline]
pub fn get_process_root(pid: u32) -> Result<PathBuf, Error> {
    Ok(fs::read_link(proc_pid_path(pid).join("root"))?)
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
    // Every file of every process is read into this one buffer, which keeps the scan from allocating and zeroing a new one for each of them.
    let mut buffer = Vec::new();

    let pids = match process_filter.pid_filter {
        Some(pid_filter) => find_process_and_descendants(pid_filter, &mut buffer)?,
        None => read_pids()?,
    };

    let mut processes_with_stat = Vec::with_capacity(pids.len());

    // The boot time is computed once, so every process in this scan uses the same value.
    let btime = get_btime();

    let mut tty_cache = TtyCache::new();

    for pid in pids {
        match get_process_with_stat_inner(
            pid,
            &proc_pid_path(pid),
            process_filter,
            btime,
            &mut tty_cache,
            &mut buffer,
        ) {
            Ok(Some(process_with_stat)) => processes_with_stat.push(process_with_stat),
            Ok(None) => (),
            Err(err) if is_process_unreadable(&err) => (),
            Err(err) => return Err(err),
        }
    }

    Ok(processes_with_stat)
}

/// List the PIDs of every process, in the order of the `/proc` folder.
fn read_pids() -> Result<Vec<u32>, Error> {
    let mut pids = Vec::new();

    for dir_entry in Path::new("/proc").read_dir()? {
        let dir_entry = dir_entry?;

        if let Some(pid) = dir_entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) {
            pids.push(pid);
        }
    }

    Ok(pids)
}

/// Find `ancestor` and all of its descendants, in the order of their PIDs. Only the `stat` file of every process is read for this, so a PID filter does not pay for the other files of the processes it drops.
fn find_process_and_descendants(ancestor: u32, buffer: &mut Vec<u8>) -> Result<Vec<u32>, Error> {
    // Every process is recorded here before any of them is checked, so descendants are found no matter the scanning order.
    let mut pid_ppid_map: BTreeMap<u32, u32> = BTreeMap::new();

    for pid in read_pids()? {
        match get_process_ppid(&proc_pid_path(pid), buffer) {
            Ok(ppid) => {
                pid_ppid_map.insert(pid, ppid);
            },
            Err(err) if is_process_unreadable(&err) => (),
            Err(err) => return Err(err),
        }
    }

    Ok(pid_ppid_map
        .keys()
        .copied()
        .filter(|&pid| is_process_or_descendant(pid, ancestor, &pid_ppid_map))
        .collect())
}

/// Get process information by reading files in the `/proc/PID` folders and measure the cpu utilization in percentage within a specific time interval. If the number it returns is `1.0`, means `100%`.
///
/// The CPU time of the whole system is sampled before the scan of the processes starts, while the time of each process is sampled during it, so the denominator covers a slightly longer span than the numerator and the result comes out a little low. The error is roughly half the scan time divided by the interval, so an interval well above the time [`get_processes_with_stat`] takes keeps it small.
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

    let (_, total_cpu_time) = compute_elapsed_cpu_time(&pre_average_cpu_stat, &average_cpu_stat);

    let total_cpu_time_f64 = total_cpu_time as f64;

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
        assert_eq!("sleep 100", cmdline_to_string(b"sleep\x00100\x00"));
        assert_eq!("", cmdline_to_string(b""));

        // A process that rewrote its argv may leave no trailing NUL.
        assert_eq!("postgres: writer", cmdline_to_string(b"postgres: writer"));
    }

    #[test]
    fn the_returned_stat_keeps_its_comm() {
        let (process, stat) = get_process_with_stat(std::process::id()).unwrap();

        assert!(!stat.comm.is_empty());
        assert_eq!(process.program, stat.comm);
    }

    #[test]
    fn pid_filter_keeps_the_process_and_its_descendants() {
        let pid = std::process::id();

        let filter = ProcessFilter {
            pid_filter: Some(pid),
            ..ProcessFilter::default()
        };

        let pids: Vec<u32> = get_processes_with_stat(&filter)
            .unwrap()
            .into_iter()
            .map(|(process, _)| process.pid)
            .collect();

        // The test process starts no child process, so it is the only one left.
        assert_eq!(vec![pid], pids);
    }
}
