use std::time::{Duration, SystemTime};

use chrono::prelude::*;
use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_single_record_file},
};

/// The uptime read from the `/proc/uptime` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Uptime {
    /// The time since boot, including the time spent in suspend.
    pub total_uptime:      Duration,
    /// The idle time summed over all CPUs, so it can be larger than `total_uptime`.
    pub all_cpu_idle_time: Duration,
}

impl Uptime {
    /// Get the btime (boot time) by subtracting this uptime from the current unix epoch timestamp.
    ///
    /// This is computed from the moment this `Uptime` was read, so it differs by a few milliseconds from [`crate::btime::get_btime`], which is derived from the system clocks at the moment it is called.
    ///
    /// ```rust
    /// use mprober_lib::uptime;
    ///
    /// let uptime = uptime::get_uptime().unwrap();
    /// let btime = uptime.get_btime();
    ///
    /// println!("{btime}");
    /// ```
    #[inline]
    pub fn get_btime(&self) -> DateTime<Utc> {
        (SystemTime::now() - self.total_uptime).into()
    }
}

fn parse_uptime(data: &[u8]) -> Result<Uptime, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let uptime = sc.next_f64()?.or_eof()?;
    let idle_time = sc.next_f64()?.or_eof()?;

    Ok(Uptime {
        total_uptime:      Duration::from_secs_f64(uptime),
        all_cpu_idle_time: Duration::from_secs_f64(idle_time),
    })
}

/// Get the uptime by reading the `/proc/uptime` file.
///
/// ```rust
/// use mprober_lib::uptime;
///
/// let uptime = uptime::get_uptime().unwrap();
///
/// println!("{uptime:#?}");
/// ```
#[inline]
pub fn get_uptime() -> Result<Uptime, Error> {
    parse_uptime(&read_single_record_file("/proc/uptime", 64)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let uptime = parse_uptime(b"57990.61 1381406.75\n".as_slice()).unwrap();

        assert_eq!(Duration::from_secs_f64(57990.61), uptime.total_uptime);

        // The idle time is summed over all CPUs, so it can be larger than the uptime.
        assert_eq!(Duration::from_secs_f64(1381406.75), uptime.all_cpu_idle_time);
    }
}
