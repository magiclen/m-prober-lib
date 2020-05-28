use std::io::ErrorKind;
use std::time::Duration;

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Debug, Clone)]
pub struct Uptime {
    pub total_uptime: Duration,
    pub all_cpu_idle_time: Duration,
}

/// Get the uptime by reading the `/proc/uptime` file.
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
