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
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::load_average;
///
/// let load_average = load_average::get_load_average().unwrap();
///
/// println!("{:#?}", load_average);
/// ```
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
