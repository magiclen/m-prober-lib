use std::io::ErrorKind;
use std::path::Path;

use crate::scanner_rust::generic_array::typenum::U96;
use crate::scanner_rust::{ScannerAscii, ScannerError};

use crate::process::ProcessStat;

#[derive(Default, Debug, Clone)]
pub struct ProcessTimeStat {
    pub utime: u32,
    pub stime: u32,
}

impl ProcessTimeStat {
    /// Compute CPU utilization in percentage between two `ProcessTimeStat` instances at different time. If it returns `1.0`, means `100%`.
    ///
    /// ```rust
    /// extern crate mprober_lib;
    ///
    /// use std::thread::sleep;
    /// use std::time::Duration;
    ///
    /// use mprober_lib::cpu;
    /// use mprober_lib::process;
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
    ///     (average_cpu_time.get_total_time() - pre_average_cpu_time.get_total_time()) as f64
    /// };
    ///
    /// let cpu_percentage = pre_process_time_stat
    ///     .compute_cpu_utilization_in_percentage(&process_time_stat, total_cpu_time_f64);
    ///
    /// println!("{:.2}%", cpu_percentage * 100.0);
    /// ```
    #[inline]
    pub fn compute_cpu_utilization_in_percentage(
        &self,
        process_time_stat_after_this: &ProcessTimeStat,
        total_cpu_time: f64,
    ) -> f64 {
        let d_utime = process_time_stat_after_this.utime - self.utime;
        let d_stime = process_time_stat_after_this.stime - self.stime;
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
            utime: process_stat.utime,
            stime: process_stat.stime,
        }
    }
}

/// Get the time stat of a specific process found by ID by reading the `/proc/PID/stat` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::process;
///
/// let process_time_stat = process::get_process_time_stat(1).unwrap();
///
/// println!("{:#?}", process_time_stat);
/// ```
pub fn get_process_time_stat(pid: u32) -> Result<ProcessTimeStat, ScannerError> {
    let stat_path = Path::new("/proc").join(pid.to_string()).join("stat");

    let mut sc: ScannerAscii<_, U96> = ScannerAscii::scan_path2(stat_path)?;

    sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;

    loop {
        let comm = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

        if comm.ends_with(b")") {
            break;
        }
    }

    for _ in 0..11 {
        sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;
    }

    let utime = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
    let stime = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

    let time_stat = ProcessTimeStat {
        utime,
        stime,
    };

    Ok(time_stat)
}
