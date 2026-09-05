use std::{
    collections::BTreeMap,
    fs,
    hash::{Hash, Hasher},
    io::ErrorKind,
    mem::take,
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use chrono::prelude::*;

use crate::{
    btime::get_btime,
    cpu::get_average_cpu_stat,
    process::{
        ProcessFilter, ProcessStat, ProcessState, ProcessTimeStat, get_process_stat,
        get_process_status, get_process_time_stat, process_stat::get_process_ppid,
    },
    scanner_rust::ScannerError,
    utils::clock_ticks_to_duration,
};

#[derive(Debug, Clone, Eq)]
pub struct Process {
    pub pid:                u32,
    pub effective_uid:      u32,
    pub effective_gid:      u32,
    pub state:              ProcessState,
    pub ppid:               u32,
    pub program:            String,
    pub cmdline:            String,
    /// The path of the executable. It is `None` for kernel threads or when the permission is denied.
    pub exe:                Option<PathBuf>,
    pub tty:                Option<String>,
    pub priority:           i8,
    pub real_time_priority: Option<u8>,
    pub nice:               i8,
    pub threads:            usize,
    /// Virtual Set Size (VIRT)
    pub vsz:                usize,
    /// Resident Set Size (RES)
    pub rss:                usize,
    /// Resident Shared Size (SHR)
    pub rss_shared:         usize,
    /// Resident Anonymous Memory
    pub rss_anon:           usize,
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

fn get_process_with_stat_inner<P: AsRef<Path>>(
    pid: u32,
    process_path: P,
    process_filter: &ProcessFilter,
) -> Result<ProcessProbe, ScannerError> {
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

    let cmdline = {
        let mut data = fs::read(process_path.join("cmdline"))?;

        for e in data.iter_mut() {
            if *e == 0 {
                *e = b' ';
            }
        }

        // Command line arguments are arbitrary bytes, so they may not be valid UTF-8.
        String::from_utf8_lossy(&data).into_owned()
    };

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

    // `starttime` is in clock ticks since boot, not in milliseconds.
    let start_time = get_btime() + clock_ticks_to_duration(stat.starttime);

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
pub fn get_process_with_stat(pid: u32) -> Result<(Process, ProcessStat), ScannerError> {
    let process_path = Path::new("/proc").join(pid.to_string());

    match get_process_with_stat_inner(pid, process_path, &ProcessFilter::default())? {
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
) -> Result<Vec<(Process, ProcessStat)>, ScannerError> {
    let mut processes_with_stat = Vec::new();

    // Every scanned process is recorded here (even the filtered ones), so descendants can be found no matter the scanning order.
    let mut pid_ppid_map: BTreeMap<u32, u32> = BTreeMap::new();

    for dir_entry in Path::new("/proc").read_dir()? {
        let dir_entry = dir_entry?;

        let Some(pid) = dir_entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };

        match get_process_with_stat_inner(pid, dir_entry.path(), process_filter) {
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
                // The process may have exited during the scan.
                if let ScannerError::IOError(err) = &err
                    && err.kind() == ErrorKind::NotFound
                {
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
) -> Result<Vec<(Process, f64)>, ScannerError> {
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
