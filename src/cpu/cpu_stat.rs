use std::{
    io::{self, ErrorKind, Read},
    thread::sleep,
    time::Duration,
};

use scanner_rust::ScannerAscii;

use crate::{Error, cpu::CPUTime};

/// CPU times in `USER_HZ` clock ticks, read from the `cpu` lines of `/proc/stat`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPUStat {
    /// Time spent in user mode.
    pub user:       u64,
    /// Time spent in user mode with a low priority.
    pub nice:       u64,
    /// Time spent in kernel mode.
    pub system:     u64,
    /// Time spent doing nothing.
    pub idle:       u64,
    /// Time spent waiting for I/O to complete. It is not reliable, see `proc(5)`.
    pub iowait:     u64,
    /// Time spent servicing hardware interrupts.
    pub irq:        u64,
    /// Time spent servicing software interrupts.
    pub softirq:    u64,
    /// Time stolen by other operating systems running in a virtualized environment.
    pub steal:      u64,
    /// Time spent running a virtual CPU for a guest operating system.
    pub guest:      u64,
    /// Time spent running a virtual CPU for a guest operating system with a low priority.
    pub guest_nice: u64,
}

impl CPUStat {
    /// Add all idle time and non-idle time respectively.
    ///
    /// ```rust
    /// use mprober_lib::cpu;
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    /// let cpu_time = average_cpu_stat.compute_cpu_time();
    ///
    /// println!("{cpu_time:#?}");
    /// ```
    #[inline]
    pub fn compute_cpu_time(&self) -> CPUTime {
        let idle = self.idle + self.iowait;

        let non_idle = self.user + self.nice + self.system + self.irq + self.softirq + self.steal;

        CPUTime {
            non_idle,
            idle,
        }
    }

    /// Compute CPU utilization in percentage between two `CPUStat` instances at different time. If it returns `1.0`, means `100%`.
    ///
    /// ```rust
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::cpu;
    ///
    /// let pre_average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    ///
    /// sleep(Duration::from_millis(100));
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    ///
    /// let cpu_percentage = pre_average_cpu_stat
    ///     .compute_cpu_utilization_in_percentage(&average_cpu_stat);
    ///
    /// println!("{:.2}%", cpu_percentage * 100.0);
    /// ```
    #[inline]
    pub fn compute_cpu_utilization_in_percentage(&self, cpu_stat_after_this: &CPUStat) -> f64 {
        let pre_cpu_time = self.compute_cpu_time();
        let cpu_time = cpu_stat_after_this.compute_cpu_time();

        let d_total = cpu_time.get_total_time().saturating_sub(pre_cpu_time.get_total_time());
        let d_non_idle = cpu_time.non_idle.saturating_sub(pre_cpu_time.non_idle);

        if d_total == 0 {
            return 0.0;
        }

        d_non_idle as f64 / d_total as f64
    }
}

/// Read the ten time fields that follow a `cpu` label in `/proc/stat`.
#[inline]
fn read_cpu_stat<R: Read, const N: usize>(sc: &mut ScannerAscii<R, N>) -> Result<CPUStat, Error> {
    let user = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let nice = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let system = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let idle = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let iowait = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let irq = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let softirq = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let steal = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let guest = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let guest_nice = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

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
}

/// Get average CPU stats by reading the `/proc/stat` file.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
///
/// println!("{average_cpu_stat:#?}");
/// ```
pub fn get_average_cpu_stat() -> Result<CPUStat, Error> {
    let mut sc: ScannerAscii<_, 72> = ScannerAscii::scan_path2("/proc/stat")?;

    let label = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    if label != b"cpu" {
        return Err(io::Error::from(ErrorKind::InvalidData).into());
    }

    read_cpu_stat(&mut sc)
}

/// Get all CPUs' stats with or without the average by reading the `/proc/stat` file. The stats are in the order of the `cpuN` lines, and offline CPUs are not listed, so the index is not always the CPU number.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let all_cpus_stat = cpu::get_all_cpus_stat(false).unwrap();
///
/// println!("{all_cpus_stat:#?}");
/// ```
pub fn get_all_cpus_stat(with_average: bool) -> Result<Vec<CPUStat>, Error> {
    let mut sc: ScannerAscii<_, 1024> = ScannerAscii::scan_path2("/proc/stat")?;

    let mut cpus_stat = Vec::with_capacity(1);

    if with_average {
        let label = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        if label != b"cpu" {
            return Err(io::Error::from(ErrorKind::InvalidData).into());
        }

        cpus_stat.push(read_cpu_stat(&mut sc)?);
    } else {
        sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    while let Some(label) = sc.next_raw()? {
        if !label.starts_with(b"cpu") {
            break;
        }

        cpus_stat.push(read_cpu_stat(&mut sc)?);
    }

    Ok(cpus_stat)
}

/// Calculate average CPU utilization in percentage within a specific time interval. It will cause the current thread to sleep. If the number it returns is `1.0`, means `100%`.
///
/// ```rust
/// use std::time::Duration;
///
/// use mprober_lib::cpu;
///
/// let cpu_percentage = cpu::get_average_cpu_utilization_in_percentage(
///     Duration::from_millis(100),
/// )
/// .unwrap();
///
/// println!("{:.2}%", cpu_percentage * 100.0);
/// ```
#[inline]
pub fn get_average_cpu_utilization_in_percentage(interval: Duration) -> Result<f64, Error> {
    let pre_cpu_stat = get_average_cpu_stat()?;

    sleep(interval);

    let cpu_stat = get_average_cpu_stat()?;

    Ok(pre_cpu_stat.compute_cpu_utilization_in_percentage(&cpu_stat))
}

/// Calculate all CPU utilization in percentage with or without the average within a specific time interval. It will cause the current thread to sleep. If the number it returns is `1.0`, means `100%`. The two reads are matched by position, so a CPU going online or offline in between shifts the result.
///
/// ```rust
/// use std::time::Duration;
///
/// use mprober_lib::cpu;
///
/// let all_cpu_percentage_without_average: Vec<String> =
///     cpu::get_all_cpu_utilization_in_percentage(
///         false,
///         Duration::from_millis(100),
///     )
///     .unwrap()
///     .into_iter()
///     .map(|cpu_percentage| format!("{:.2}%", cpu_percentage * 100.0))
///     .collect();
///
/// println!("{:#?}", all_cpu_percentage_without_average);
/// ```
#[inline]
pub fn get_all_cpu_utilization_in_percentage(
    with_average: bool,
    interval: Duration,
) -> Result<Vec<f64>, Error> {
    let pre_cpus_stat = get_all_cpus_stat(with_average)?;

    sleep(interval);

    let cpus_stat = get_all_cpus_stat(with_average)?;

    let result = pre_cpus_stat
        .into_iter()
        .zip(cpus_stat)
        .map(|(pre_cpus_stat, cpus_stat)| {
            pre_cpus_stat.compute_cpu_utilization_in_percentage(&cpus_stat)
        })
        .collect();

    Ok(result)
}
