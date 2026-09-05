use std::io::{self, ErrorKind};

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct LoadAverage {
    pub one:     f64,
    pub five:    f64,
    pub fifteen: f64,
    // Not include the numbers of active/total scheduled entities and the last created PID.
}

/// Get the load average by reading the `/proc/loadavg` file.
///
/// ```rust
/// use mprober_lib::load_average;
///
/// let load_average = load_average::get_load_average().unwrap();
///
/// println!("{load_average:#?}");
/// ```
#[inline]
pub fn get_load_average() -> Result<LoadAverage, ScannerError> {
    let mut sc: ScannerAscii<_, 24> = ScannerAscii::scan_path2("/proc/loadavg")?;

    let one = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let five = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let fifteen = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(LoadAverage {
        one,
        five,
        fifteen,
    })
}
