use std::io::ErrorKind;
use std::thread::sleep;
use std::time::Duration;

use crate::cpu::CPUTime;

use crate::scanner_rust::generic_array::typenum::{U1024, U72};
use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct CPUStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,
}

impl CPUStat {
    /// Add all idle time and non-idle time respectively.
    ///
    /// ```rust
    /// extern crate mprober_lib;
    ///
    /// use mprober_lib::cpu;
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    /// let cpu_time = average_cpu_stat.compute_cpu_time();
    ///
    /// println!("{:#?}", cpu_time);
    /// ```
    #[inline]
    pub fn compute_cpu_time(&self) -> CPUTime {
        let idle = self.idle + self.iowait;

        let non_idle = self.user + self.nice + self.system + self.irq + self.softirq + self.steal;

        CPUTime {
            idle,
            non_idle,
        }
    }

    /// Compute CPU utilization in percentage between two `CPUStat` instances at different time. If it returns `1.0`, means `100%`.
    ///
    /// ```rust
    /// extern crate mprober_lib;
    ///
    /// use std::thread::sleep;
    /// use std::time::Duration;
    ///
    /// use mprober_lib::cpu;
    ///
    /// let pre_average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    ///
    /// sleep(Duration::from_millis(100));
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    ///
    /// let cpu_percentage =
    ///     pre_average_cpu_stat.compute_cpu_utilization_in_percentage(&average_cpu_stat);
    ///
    /// println!("{:.2}%", cpu_percentage * 100.0);
    /// ```
    #[inline]
    pub fn compute_cpu_utilization_in_percentage(&self, cpu_stat_after_this: &CPUStat) -> f64 {
        let pre_cpu_time = self.compute_cpu_time();
        let cpu_time = cpu_stat_after_this.compute_cpu_time();

        let d_total = cpu_time.get_total_time() - pre_cpu_time.get_total_time();
        let d_non_idle = cpu_time.non_idle - pre_cpu_time.non_idle;

        d_non_idle as f64 / d_total as f64
    }
}

/// Get average CPU stats by reading the `/proc/stat` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::cpu;
///
/// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
///
/// println!("{:#?}", average_cpu_stat);
/// ```
pub fn get_average_cpu_stat() -> Result<CPUStat, ScannerError> {
    let mut sc: ScannerAscii<_, U72> = ScannerAscii::scan_path2("/proc/stat")?;

    let label = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

    if label == b"cpu" {
        let user = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let nice = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let system = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let idle = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let iowait = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let irq = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let softirq = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let steal = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let guest = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
        let guest_nice = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

        Ok(CPUStat {
            user,
            nice,
            system,
            idle,
            iowait,
            irq,
            softirq,
            steal,
            guest,
            guest_nice,
        })
    } else {
        Err(ErrorKind::InvalidData.into())
    }
}

/// Get all CPUs' stats with or without the average by reading the `/proc/stat` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::cpu;
///
/// let all_cpus_stat = cpu::get_all_cpus_stat(false).unwrap();
///
/// println!("{:#?}", all_cpus_stat);
/// ```
pub fn get_all_cpus_stat(with_average: bool) -> Result<Vec<CPUStat>, ScannerError> {
    let mut sc: ScannerAscii<_, U1024> = ScannerAscii::scan_path2("/proc/stat")?;

    let mut cpus_stat = Vec::with_capacity(1);

    if with_average {
        let label = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

        if label == b"cpu" {
            let user = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let nice = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let system = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let idle = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let iowait = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let irq = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let softirq = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let steal = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let guest = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let guest_nice = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

            let cpu_stat = CPUStat {
                user,
                nice,
                system,
                idle,
                iowait,
                irq,
                softirq,
                steal,
                guest,
                guest_nice,
            };

            cpus_stat.push(cpu_stat);
        } else {
            return Err(ErrorKind::InvalidData.into());
        }
    } else {
        sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
    }

    loop {
        let label = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

        if label.starts_with(b"cpu") {
            let user = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let nice = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let system = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let idle = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let iowait = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let irq = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let softirq = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let steal = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let guest = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;
            let guest_nice = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

            let cpu_stat = CPUStat {
                user,
                nice,
                system,
                idle,
                iowait,
                irq,
                softirq,
                steal,
                guest,
                guest_nice,
            };

            cpus_stat.push(cpu_stat);
        } else {
            break;
        }
    }

    Ok(cpus_stat)
}

/// Calculate average CPU utilization in percentage within a specific time interval. It will cause the current thread to sleep. If the number it returns is `1.0`, means `100%`.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use std::time::Duration;
///
/// use mprober_lib::cpu;
///
/// let cpu_percentage =
///     cpu::get_average_cpu_utilization_in_percentage(Duration::from_millis(100)).unwrap();
///
/// println!("{:.2}%", cpu_percentage * 100.0);
/// ```
#[inline]
pub fn get_average_cpu_utilization_in_percentage(interval: Duration) -> Result<f64, ScannerError> {
    let pre_cpu_stat = get_average_cpu_stat()?;

    sleep(interval);

    let cpu_stat = get_average_cpu_stat()?;

    Ok(pre_cpu_stat.compute_cpu_utilization_in_percentage(&cpu_stat))
}

/// Calculate all CPU utilization in percentage with or without the average within a specific time interval. It will cause the current thread to sleep. If the number it returns is `1.0`, means `100%`.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use std::time::Duration;
///
/// use mprober_lib::cpu;
///
/// let all_cpu_percentage_without_average: Vec<String> =
///     cpu::get_all_cpu_utilization_in_percentage(false, Duration::from_millis(100))
///         .unwrap()
///         .into_iter()
///         .map(|cpu_percentage| format!("{:.2}%", cpu_percentage * 100.0))
///         .collect();
///
/// println!("{:#?}", all_cpu_percentage_without_average);
/// ```
#[inline]
pub fn get_all_cpu_utilization_in_percentage(
    with_average: bool,
    interval: Duration,
) -> Result<Vec<f64>, ScannerError> {
    let pre_cpus_stat = get_all_cpus_stat(with_average)?;

    sleep(interval);

    let cpus_stat = get_all_cpus_stat(with_average)?;

    let result = pre_cpus_stat
        .into_iter()
        .zip(cpus_stat.into_iter())
        .map(|(pre_cpus_stat, cpus_stat)| {
            pre_cpus_stat.compute_cpu_utilization_in_percentage(&cpus_stat)
        })
        .collect();

    Ok(result)
}
