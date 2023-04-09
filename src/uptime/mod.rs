extern crate chrono;

use std::{
    io::ErrorKind,
    time::{Duration, SystemTime},
};

use chrono::prelude::*;

use crate::scanner_rust::{generic_array::typenum::U24, ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct Uptime {
    pub total_uptime:      Duration,
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
    /// println!("{}", btime);
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
    let mut sc: ScannerAscii<_, U24> = ScannerAscii::scan_path2("/proc/uptime")?;

    let uptime = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;
    let idle_time = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;

    Ok(Uptime {
        total_uptime:      Duration::from_secs_f64(uptime),
        all_cpu_idle_time: Duration::from_secs_f64(idle_time),
    })
}
