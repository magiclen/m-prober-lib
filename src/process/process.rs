extern crate chrono;
extern crate regex;

use std::collections::BTreeMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::mem::replace;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use crate::btime::get_btime;
use crate::cpu::get_average_cpu_stat;
use crate::process::{
    get_process_stat, get_process_status, get_process_time_stat, ProcessFilter, ProcessStat,
    ProcessState, ProcessTimeStat,
};

use crate::scanner_rust::ScannerError;

use chrono::prelude::*;

#[derive(Debug, Clone, Eq)]
pub struct Process {
    pub pid: u32,
    pub effective_uid: u32,
    pub effective_gid: u32,
    pub state: ProcessState,
    pub ppid: u32,
    pub program: String,
    pub cmdline: String,
    pub tty: Option<String>,
    pub priority: i8,
    pub real_time_priority: Option<u8>,
    pub nice: i8,
    pub threads: usize,
    /// Virtual Set Size (VIRT)
    pub vsz: usize,
    /// Resident Set Size (RES)
    pub rss: usize,
    /// Resident Shared Size (SHR)
    pub rss_shared: usize,
    /// Resident Anonymous Memory
    pub rss_anon: usize,
    pub start_time: DateTime<Utc>,
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

fn get_process_with_stat_inner<P: AsRef<Path>>(
    pid: u32,
    process_path: P,
    process_filter: &ProcessFilter,
) -> Result<Option<(Process, ProcessStat)>, ScannerError> {
    let process_path = process_path.as_ref();

    let mut program_filter_match = true;

    let status = get_process_status(pid)?;

    if let Some(uid_filter) = process_filter.uid_filter {
        if status.real_uid != uid_filter
            && status.effective_uid != uid_filter
            && status.saved_set_uid != uid_filter
            && status.fs_uid != uid_filter
        {
            return Ok(None);
        }
    }

    if let Some(gid_filter) = process_filter.gid_filter {
        if status.real_gid != gid_filter
            && status.effective_gid != gid_filter
            && status.saved_set_gid != gid_filter
            && status.fs_gid != gid_filter
        {
            return Ok(None);
        }
    }

    let cmdline = {
        let mut data = fs::read(process_path.join("cmdline"))?;

        for e in data.iter_mut() {
            if *e == 0 {
                *e = b' ';
            }
        }

        unsafe { String::from_utf8_unchecked(data) }
    };

    if let Some(program_filter) = process_filter.program_filter.as_ref() {
        if !program_filter.is_match(&cmdline) {
            program_filter_match = false;
        }
    }

    let mut stat = get_process_stat(pid)?;

    if !program_filter_match {
        if let Some(program_filter) = process_filter.program_filter.as_ref() {
            if !program_filter.is_match(&stat.comm) {
                return Ok(None);
            }
        }
    }

    let effective_uid = status.effective_uid;
    let effective_gid = status.effective_gid;
    let state = stat.state;
    let ppid = stat.ppid;
    let program = replace(&mut stat.comm, String::new());

    let tty = {
        match stat.tty_nr_major {
            4 => {
                if stat.tty_nr_minor < 64 {
                    Some(format!("tty{}", stat.tty_nr_minor))
                } else {
                    Some(format!("ttyS{}", stat.tty_nr_minor - 64))
                }
            }
            136..=143 => Some(format!("pts/{}", stat.tty_nr_minor)),
            _ => None,
        }
    };

    if let Some(tty_filter) = process_filter.tty_filter.as_ref() {
        match tty.as_ref() {
            Some(tty) => {
                if !tty_filter.is_match(tty) {
                    return Ok(None);
                }
            }
            None => return Ok(None),
        }
    }

    let priority = stat.priority;
    let real_time_priority = if stat.rt_priority > 0 {
        Some(stat.rt_priority)
    } else {
        None
    };
    let nice = stat.nice;
    let threads = stat.num_threads;
    let vsz = stat.vsize;
    let rss = stat.rss;
    let rss_shared = stat.shared;
    let rss_anon = stat.rss_anon;

    let start_time =
        get_btime() + chrono::Duration::from_std(Duration::from_millis(stat.starttime)).unwrap();

    let process = Process {
        pid,
        effective_uid,
        effective_gid,
        state,
        ppid,
        program,
        cmdline,
        tty,
        priority,
        real_time_priority,
        nice,
        threads,
        start_time,
        vsz,
        rss,
        rss_shared,
        rss_anon,
    };

    Ok(Some((process, stat)))
}

/// Get information of a specific process found by ID by reading files in the `/proc/PID` folder.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::process;
///
/// // let (process, _) = process::get_process_with_stat(1).unwrap();
///
/// // println!("{:#?}", process);
/// ```
#[inline]
pub fn get_process_with_stat(pid: u32) -> Result<(Process, ProcessStat), ScannerError> {
    let process_path = Path::new("/proc").join(pid.to_string());

    get_process_with_stat_inner(pid, process_path, &ProcessFilter::default()).map(|o| o.unwrap())
}

/// Get process information by reading files in the `/proc/PID` folders.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::process;
///
/// let processes_with_stat =
///     process::get_processes_with_stat(&process::ProcessFilter::default()).unwrap();
///
/// println!("{:#?}", processes_with_stat);
/// ```
pub fn get_processes_with_stat(
    process_filter: &ProcessFilter,
) -> Result<Vec<(Process, ProcessStat)>, ScannerError> {
    let mut processes_with_stats = Vec::new();

    let proc = Path::new("/proc");

    if let Some(pid_filter) = process_filter.pid_filter.as_ref().copied() {
        let mut pid_ppid_map: BTreeMap<u32, u32> = BTreeMap::new();

        for dir_entry in proc.read_dir()? {
            let dir_entry = dir_entry?;

            if let Some(file_name) = dir_entry.file_name().to_str() {
                if let Ok(pid) = file_name.parse::<u32>() {
                    let process_path = dir_entry.path();

                    match get_process_with_stat_inner(pid, process_path, process_filter) {
                        Ok(r) => {
                            if let Some((process, stat)) = r {
                                if pid != pid_filter && process.ppid != pid_filter {
                                    let mut not_related = true;

                                    let mut p_ppid = pid_ppid_map.get(&process.ppid);

                                    while let Some(ppid) = p_ppid.copied() {
                                        if ppid == pid_filter {
                                            not_related = false;

                                            break;
                                        }

                                        p_ppid = pid_ppid_map.get(&ppid);
                                    }

                                    if not_related {
                                        continue;
                                    }
                                }

                                pid_ppid_map.insert(pid, process.ppid);

                                processes_with_stats.push((process, stat));
                            }
                        }
                        Err(err) => {
                            if let ScannerError::IOError(err) = &err {
                                if err.kind() == ErrorKind::NotFound {
                                    continue;
                                }
                            }

                            return Err(err);
                        }
                    }
                }
            }
        }
    } else {
        for dir_entry in proc.read_dir()? {
            let dir_entry = dir_entry?;

            if let Some(file_name) = dir_entry.file_name().to_str() {
                if let Ok(pid) = file_name.parse::<u32>() {
                    let process_path = dir_entry.path();

                    match get_process_with_stat_inner(pid, process_path, process_filter) {
                        Ok(r) => {
                            if let Some((process, stat)) = r {
                                processes_with_stats.push((process, stat));
                            }
                        }
                        Err(err) => {
                            if let ScannerError::IOError(err) = &err {
                                if err.kind() == ErrorKind::NotFound {
                                    continue;
                                }
                            }

                            return Err(err);
                        }
                    }
                }
            }
        }
    }

    Ok(processes_with_stats)
}

/// Get process information by reading files in the `/proc/PID` folders and measure the cpu utilization in percentage within a specific time interval. If the number it returns is `1.0`, means `100%`.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use std::thread::sleep;
/// use std::time::Duration;
///
/// use mprober_lib::process;
///
/// let processes_with_cpu_percentage = process::get_processes_with_cpu_utilization_in_percentage(
///     &process::ProcessFilter::default(),
///     Duration::from_millis(100),
/// )
/// .unwrap();
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
    let processes_with_stat = get_processes_with_stat(process_filter).unwrap();

    let mut processes_with_cpu_percentage = Vec::with_capacity(processes_with_stat.len());

    sleep(interval);

    let average_cpu_stat = get_average_cpu_stat()?;

    let total_cpu_time_f64 = {
        let pre_average_cpu_time = pre_average_cpu_stat.compute_cpu_time();
        let average_cpu_time = average_cpu_stat.compute_cpu_time();

        (average_cpu_time.get_total_time() - pre_average_cpu_time.get_total_time()) as f64
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
