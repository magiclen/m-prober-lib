use std::io::ErrorKind;

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Debug, Clone)]
pub struct LoadAverage {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
    // Not include the numbers of active/total scheduled entities and the last created PID.
}

/// Get the load average by reading the `/proc/loadavg` file.
#[inline]
pub fn get_load_average() -> Result<LoadAverage, ScannerError> {
    let mut sc = ScannerAscii::scan_path("/proc/loadavg")?;

    let one = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;
    let five = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;
    let fifteen = sc.next_f64()?.ok_or(ErrorKind::UnexpectedEof)?;

    Ok(LoadAverage {
        one,
        five,
        fifteen,
    })
}
