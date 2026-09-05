use std::io::{self, ErrorKind};

use chrono::prelude::*;

use crate::scanner_rust::{ScannerAscii, ScannerError};

/// Get the RTC datetime by reading the `/proc/driver/rtc` file.
///
/// ```rust
/// use mprober_lib::rtc_time;
///
/// let rtc_date_time = rtc_time::get_rtc_date_time().unwrap();
///
/// println!("{rtc_date_time}");
/// ```
#[inline]
pub fn get_rtc_date_time() -> Result<NaiveDateTime, ScannerError> {
    let mut sc: ScannerAscii<_, 52> = ScannerAscii::scan_path2("/proc/driver/rtc")?;

    sc.drop_next_bytes("rtc_time".len())?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    sc.drop_next_until(": ")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let hour = sc.next_u32_until(":")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let minute = sc.next_u32_until(":")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let second = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next_bytes("rtc_time".len())?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    sc.drop_next_until(": ")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let year = sc.next_i32_until("-")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let month = sc.next_u32_until("-")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let date = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let date = NaiveDate::from_ymd_opt(year, month, date)
        .ok_or(io::Error::from(ErrorKind::InvalidData))?;
    let time = NaiveTime::from_hms_opt(hour, minute, second)
        .ok_or(io::Error::from(ErrorKind::InvalidData))?;

    Ok(NaiveDateTime::new(date, time))
}
