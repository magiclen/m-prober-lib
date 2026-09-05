use std::{
    io::{self, ErrorKind},
    time::{Duration, SystemTime},
};

use chrono::prelude::*;

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct Uptime {
    /// The time since boot, including the time spent in suspend.
    pub total_uptime:      Duration,
    /// The idle time summed over all CPUs, so it can be larger than `total_uptime`.
    pub all_cpu_idle_time: Duration,
}

impl Uptime {
    /// Get the btime (boot time) by subtracting this uptime from the current unix epoch timestamp.
    ///
    /// This is computed from the moment this `Uptime` was read, so it differs by a few milliseconds from [`crate::btime::get_btime`], which is cached and derived from the system clocks.
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
pub fn get_uptime() -> Result<Uptime, ScannerError> {
    let mut sc: ScannerAscii<_, 24> = ScannerAscii::scan_path2("/proc/uptime")?;

    let uptime = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let idle_time = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(Uptime {
        total_uptime:      Duration::from_secs_f64(uptime),
        all_cpu_idle_time: Duration::from_secs_f64(idle_time),
    })
}
