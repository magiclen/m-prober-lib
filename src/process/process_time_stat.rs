use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    process::{
        ProcessStat,
        process_stat::{read_process_stat_file, split_process_stat_line},
    },
    utils::{OrEof, proc_pid_path},
};

/// CPU times of a process in `USER_HZ` clock ticks.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessTimeStat {
    /// Time spent in user mode, in clock ticks.
    pub utime: u64,
    /// Time spent in kernel mode, in clock ticks.
    pub stime: u64,
}

impl ProcessTimeStat {
    /// Compute CPU utilization in percentage between two `ProcessTimeStat` instances at different time. If it returns `1.0`, means `100%`.
    ///
    /// `total_cpu_time` is the total time of all CPUs, so a process using one whole core on a 4-core machine gives `0.25`.
    ///
    /// ```rust
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::{cpu, process};
    ///
    /// let pre_average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    /// let pre_process_time_stat = process::get_process_time_stat(1).unwrap();
    ///
    /// sleep(Duration::from_millis(100));
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    /// let process_time_stat = process::get_process_time_stat(1).unwrap();
    ///
    /// let total_cpu_time_f64 = {
    ///     let pre_average_cpu_time = pre_average_cpu_stat.compute_cpu_time();
    ///     let average_cpu_time = average_cpu_stat.compute_cpu_time();
    ///
    ///     average_cpu_time
    ///         .get_total_time()
    ///         .saturating_sub(pre_average_cpu_time.get_total_time())
    ///         as f64
    /// };
    ///
    /// let cpu_percentage = pre_process_time_stat
    ///     .compute_cpu_utilization_in_percentage(
    ///         &process_time_stat,
    ///         total_cpu_time_f64,
    ///     );
    ///
    /// println!("{:.2}%", cpu_percentage * 100.0);
    /// ```
    #[inline]
    pub fn compute_cpu_utilization_in_percentage(
        &self,
        process_time_stat_after_this: &ProcessTimeStat,
        total_cpu_time: f64,
    ) -> f64 {
        // The PID may have been reused by a new process, so the counters can go backwards.
        let d_utime = process_time_stat_after_this.utime.saturating_sub(self.utime);
        let d_stime = process_time_stat_after_this.stime.saturating_sub(self.stime);
        let d_time_f64 = (d_utime + d_stime) as f64;

        if total_cpu_time < 1.0 {
            0.0
        } else if d_time_f64 >= total_cpu_time {
            1.0
        } else {
            d_time_f64 / total_cpu_time
        }
    }
}

impl From<ProcessStat> for ProcessTimeStat {
    #[inline]
    fn from(process_stat: ProcessStat) -> Self {
        ProcessTimeStat {
            utime: process_stat.utime, stime: process_stat.stime
        }
    }
}

fn parse_process_time_stat(line: &[u8]) -> Result<ProcessTimeStat, Error> {
    let (_, fields) = split_process_stat_line(line)?;

    let mut sc = ScannerU8SliceAscii::new(fields);

    for _ in 0..11 {
        sc.drop_next()?.or_eof()?;
    }

    let utime = sc.next_u64()?.or_eof()?;
    let stime = sc.next_u64()?.or_eof()?;

    Ok(ProcessTimeStat {
        utime,
        stime,
    })
}

/// Get the time stat of a specific process found by ID by reading the `/proc/PID/stat` file.
///
/// ```rust
/// use mprober_lib::process;
///
/// let process_time_stat = process::get_process_time_stat(1).unwrap();
///
/// println!("{process_time_stat:#?}");
/// ```
#[inline]
pub fn get_process_time_stat(pid: u32) -> Result<ProcessTimeStat, Error> {
    let mut buffer = Vec::new();

    read_process_stat_file(&proc_pid_path(pid), &mut buffer)?;

    parse_process_time_stat(&buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::process_stat::tests::STAT_LINE;

    #[test]
    fn parse_time_stat_line() {
        let time_stat = parse_process_time_stat(STAT_LINE).unwrap();

        assert_eq!(26, time_stat.utime);
        assert_eq!(45, time_stat.stime);
    }
}
