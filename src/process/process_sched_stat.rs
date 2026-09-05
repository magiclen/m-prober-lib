use std::{
    io::{self, ErrorKind},
    path::Path,
    time::Duration,
};

use crate::{Error, scanner_rust::ScannerU8SliceAscii, utils::read_single_record_file};

/// The scheduler statistics of a process, read from the `/proc/PID/schedstat` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessSchedStat {
    /// The time the process spent running on a CPU.
    pub running:    Duration,
    /// The time the process spent on a run queue waiting for a CPU. A value that grows fast means the process is ready to run but the CPUs are busy, which no field of `/proc/PID/stat` shows.
    pub waiting:    Duration,
    /// The number of time slices the process was given.
    pub timeslices: u64,
}

impl ProcessSchedStat {
    /// Compute the fraction of time the process spent waiting for a CPU rather than running, between two `ProcessSchedStat` instances at different time. If it returns `1.0`, the process spent all of its time waiting.
    ///
    /// ```rust,no_run
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::process;
    ///
    /// let pid = std::process::id();
    ///
    /// let pre_sched_stat = process::get_process_sched_stat(pid).unwrap();
    ///
    /// sleep(Duration::from_millis(100));
    ///
    /// let sched_stat = process::get_process_sched_stat(pid).unwrap();
    ///
    /// let ratio = pre_sched_stat.compute_wait_ratio(&sched_stat);
    ///
    /// println!("{:.2}%", ratio * 100.0);
    /// ```
    #[inline]
    pub fn compute_wait_ratio(&self, sched_stat_after_this: &ProcessSchedStat) -> f64 {
        let d_running = sched_stat_after_this.running.saturating_sub(self.running);
        let d_waiting = sched_stat_after_this.waiting.saturating_sub(self.waiting);

        let total = d_running + d_waiting;

        if total.is_zero() {
            return 0.0;
        }

        d_waiting.as_secs_f64() / total.as_secs_f64()
    }
}

/// Get the scheduler statistics of a specific process found by ID by reading the `/proc/PID/schedstat` file. The file needs `CONFIG_SCHEDSTATS`, otherwise every value is `0`.
///
/// ```rust
/// use mprober_lib::process;
///
/// let sched_stat =
///     process::get_process_sched_stat(std::process::id()).unwrap();
///
/// println!("{sched_stat:#?}");
/// ```
pub fn get_process_sched_stat(pid: u32) -> Result<ProcessSchedStat, Error> {
    let path = Path::new("/proc").join(pid.to_string()).join("schedstat");

    // The file is three numbers on one line.
    let data = read_single_record_file(path, 64)?;

    let mut sc = ScannerU8SliceAscii::new(&data);

    // The first two values are in nanoseconds.
    let running = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let waiting = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let timeslices = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(ProcessSchedStat {
        running: Duration::from_nanos(running),
        waiting: Duration::from_nanos(waiting),
        timeslices,
    })
}
