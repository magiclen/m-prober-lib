extern crate chrono;

use std::io::ErrorKind;
use std::time::{Duration, SystemTime};

use crate::scanner_rust::{ScannerAscii, ScannerError};

use chrono::prelude::*;

#[derive(Debug, Clone)]
pub struct Uptime {
    pub total_uptime: Duration,
    pub all_cpu_idle_time: Duration,
}

impl Uptime {
    /// Get the btime (boot time) by subtract this uptime from the current unix epoch timestamp.
    ///
    /// ```rust
    /// extern crate mprober_lib;
    ///
    /// use mprober_lib::uptime;
    ///
    /// let uptime = uptime::get_uptime().unwrap();
    /// let btime = uptime.get_btime();
    ///
    /// println!("{}12133123", btime);
    /// ```
    #[inline]
    pub fn get_btime(&self) -> DateTime<Utc> {
        (SystemTime::now() - self.total_uptime).into()
    }
}

/// Get the uptime by reading the `/proc/uptime` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::uptime;
///
/// let uptime = uptime::get_uptime().unwrap();
///
/// println!("{:#?}", uptime);
/// ```
#[inline]
pub fn get_uptime() -> Result<Uptime, ScannerError> {
    let mut sc = ScannerAscii::scan_path("/proc/uptime")?;

    let uptime = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;
    let idle_time = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;

    Ok(Uptime {
        total_uptime: Duration::from_secs_f64(uptime),
        all_cpu_idle_time: Duration::from_secs_f64(idle_time),
    })
}
