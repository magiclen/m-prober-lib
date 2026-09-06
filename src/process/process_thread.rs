use std::fs;

use crate::{
    Error,
    process::{ProcessStat, process_stat::parse_process_stat},
    utils::{proc_pid_path, read_single_record_file},
};

/// Get the thread IDs of a specific process found by ID by reading the `/proc/PID/task` folder. The main thread is included, and its ID equals the PID.
///
/// ```rust
/// use mprober_lib::process;
///
/// let tids = process::get_process_thread_ids(std::process::id()).unwrap();
///
/// println!("{tids:?}");
/// ```
pub fn get_process_thread_ids(pid: u32) -> Result<Vec<u32>, Error> {
    let task_path = proc_pid_path(pid).join("task");

    let mut tids = Vec::with_capacity(1);

    for entry in fs::read_dir(task_path)? {
        let entry = entry?;

        let Some(tid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };

        tids.push(tid);
    }

    tids.sort_unstable();

    Ok(tids)
}

/// Get the stat of one thread of a specific process by reading the `/proc/PID/task/TID/stat` file. The file has the same format as the one of a process, so the fields mean the same, except that they describe the thread alone.
///
/// The memory fields are not filled in, because every thread of a process shares the same address space; read them from the process with [`crate::process::get_process_stat`] instead.
///
/// ```rust,no_run
/// use mprober_lib::process;
///
/// let pid = std::process::id();
///
/// let thread_stat = process::get_thread_stat(pid, pid).unwrap();
///
/// println!("{thread_stat:#?}");
/// ```
pub fn get_thread_stat(pid: u32, tid: u32) -> Result<ProcessStat, Error> {
    let stat_path = proc_pid_path(pid).join("task").join(tid.to_string()).join("stat");

    let line = read_single_record_file(stat_path, 1024)?;

    parse_process_stat(&line)
}

/// Get the stat of every thread of a specific process found by ID by reading the files in the `/proc/PID/task` folder. A thread that exits during the scan is skipped, so the result can be shorter than the `num_threads` field of the process.
///
/// As in [`get_thread_stat`], the memory fields of each [`crate::process::ProcessStat`] are left at `0`, because a thread has no address space of its own.
///
/// ```rust
/// use mprober_lib::process;
///
/// let threads = process::get_process_threads(std::process::id()).unwrap();
///
/// for (tid, stat) in threads {
///     println!("{tid}: {} {}", stat.comm, stat.state);
/// }
/// ```
pub fn get_process_threads(pid: u32) -> Result<Vec<(u32, ProcessStat)>, Error> {
    let tids = get_process_thread_ids(pid)?;

    let mut threads = Vec::with_capacity(tids.len());

    for tid in tids {
        // A thread can exit while the folder is being scanned.
        if let Ok(stat) = get_thread_stat(pid, tid) {
            threads.push((tid, stat));
        }
    }

    Ok(threads)
}
