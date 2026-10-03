use std::{
    io::{self, ErrorKind},
    thread::sleep,
    time::Duration,
};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    cpu::{CPUTime, proc_stat_capacity},
    utils::{OrEof, read_file, read_file_head},
};

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
        let (d_non_idle, d_total) = compute_elapsed_cpu_time(self, cpu_stat_after_this);

        if d_total == 0 {
            return 0.0;
        }

        d_non_idle as f64 / d_total as f64
    }
}

/// Compute the non-idle time and the total time that passed between two `CPUStat` instances.
/// `proc(5)` warns that the iowait counter can go backwards, so the idle part and the non-idle part are subtracted on their own. Subtracting the totals instead would let a smaller iowait shrink the total, and the non-idle share could go above `1.0`.
#[inline]
pub(crate) fn compute_elapsed_cpu_time(pre: &CPUStat, post: &CPUStat) -> (u64, u64) {
    let pre_cpu_time = pre.compute_cpu_time();
    let cpu_time = post.compute_cpu_time();

    let d_non_idle = cpu_time.non_idle.saturating_sub(pre_cpu_time.non_idle);
    let d_idle = cpu_time.idle.saturating_sub(pre_cpu_time.idle);

    (d_non_idle, d_non_idle + d_idle)
}

/// Read the ten time fields that follow a `cpu` label in `/proc/stat`.
#[inline]
fn read_cpu_stat(sc: &mut ScannerU8SliceAscii<'_>) -> Result<CPUStat, Error> {
    let user = sc.next_u64()?.or_eof()?;
    let nice = sc.next_u64()?.or_eof()?;
    let system = sc.next_u64()?.or_eof()?;
    let idle = sc.next_u64()?.or_eof()?;
    let iowait = sc.next_u64()?.or_eof()?;
    let irq = sc.next_u64()?.or_eof()?;
    let softirq = sc.next_u64()?.or_eof()?;
    let steal = sc.next_u64()?.or_eof()?;
    let guest = sc.next_u64()?.or_eof()?;
    let guest_nice = sc.next_u64()?.or_eof()?;

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

fn parse_average_cpu_stat(data: &[u8]) -> Result<CPUStat, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let label = sc.next()?.or_eof()?;

    if label != b"cpu" {
        return Err(io::Error::from(ErrorKind::InvalidData).into());
    }

    read_cpu_stat(&mut sc)
}

fn parse_all_cpus_stat(data: &[u8], with_average: bool) -> Result<Vec<CPUStat>, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    // One line per logical processor, which is the usual count on a machine of any size.
    let mut cpus_stat = Vec::with_capacity(16);

    if with_average {
        let label = sc.next()?.or_eof()?;

        if label != b"cpu" {
            return Err(io::Error::from(ErrorKind::InvalidData).into());
        }

        cpus_stat.push(read_cpu_stat(&mut sc)?);
    } else {
        sc.drop_next_line()?.or_eof()?;
    }

    while let Some(label) = sc.next()? {
        if !label.starts_with(b"cpu") {
            break;
        }

        cpus_stat.push(read_cpu_stat(&mut sc)?);
    }

    Ok(cpus_stat)
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
#[inline]
pub fn get_average_cpu_stat() -> Result<CPUStat, Error> {
    // The `cpu` label and its ten fields cannot fill this buffer, so reading only the head keeps the per-processor lines and the interrupt counters that follow out of it.
    parse_average_cpu_stat(&read_file_head("/proc/stat", 384)?)
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
#[inline]
pub fn get_all_cpus_stat(with_average: bool) -> Result<Vec<CPUStat>, Error> {
    parse_all_cpus_stat(&read_file("/proc/stat", proc_stat_capacity())?, with_average)
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

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &[u8] = b"cpu  228791 17 138764 57861882 19268 0 624 1 2 3
cpu0 9459 0 5828 2410479 850 0 116 0 0 0
cpu1 9107 1 5710 2411431 761 0 40 0 0 0
intr 68724016 22 1055 0 0 0
ctxt 174031029
";

    #[test]
    fn parse_average() {
        let stat = parse_average_cpu_stat(STAT).unwrap();

        assert_eq!(228791, stat.user);
        assert_eq!(17, stat.nice);
        assert_eq!(138764, stat.system);
        assert_eq!(57861882, stat.idle);
        assert_eq!(19268, stat.iowait);
        assert_eq!(0, stat.irq);
        assert_eq!(624, stat.softirq);
        assert_eq!(1, stat.steal);
        assert_eq!(2, stat.guest);
        assert_eq!(3, stat.guest_nice);
    }

    #[test]
    fn parse_all() {
        // The `intr` line that follows the last processor must end the parsing.
        let cpus_stat = parse_all_cpus_stat(STAT, false).unwrap();

        assert_eq!(2, cpus_stat.len());
        assert_eq!(9459, cpus_stat[0].user);
        assert_eq!(2411431, cpus_stat[1].idle);

        let cpus_stat = parse_all_cpus_stat(STAT, true).unwrap();

        assert_eq!(3, cpus_stat.len());
        assert_eq!(228791, cpus_stat[0].user);
        assert_eq!(9459, cpus_stat[1].user);
    }

    #[test]
    fn compute_utilization() {
        let pre = parse_average_cpu_stat(STAT).unwrap();

        let mut post = pre.clone();

        post.idle += 30;
        post.user += 10;

        assert_eq!(0.25, pre.compute_cpu_utilization_in_percentage(&post));

        // A counter that did not move at all means nothing ran in between.
        assert_eq!(0.0, pre.compute_cpu_utilization_in_percentage(&pre));

        // A smaller iowait must not shrink the total, which would push the result above `1.0`.
        let mut post = pre.clone();

        post.iowait -= 5;
        post.user += 10;

        assert_eq!(1.0, pre.compute_cpu_utilization_in_percentage(&post));
    }
}
