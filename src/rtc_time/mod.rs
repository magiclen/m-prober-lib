extern crate chrono;

use std::io::ErrorKind;

use crate::scanner_rust::{ScannerAscii, ScannerError};

use chrono::prelude::*;

/// Get the RTC datetime by reading the `/proc/driver/rtc` file.
#[inline]
pub fn get_rtc_date_time() -> Result<NaiveDateTime, ScannerError> {
    let mut sc = ScannerAscii::scan_path("/proc/driver/rtc")?;

    sc.drop_next_bytes("rtc_time".len())?.ok_or(ErrorKind::UnexpectedEof)?;
    sc.drop_next_until(": ")?.ok_or(ErrorKind::UnexpectedEof)?;

    let hour = sc.next_u32_until(":")?.ok_or(ErrorKind::UnexpectedEof)?;
    let minute = sc.next_u32_until(":")?.ok_or(ErrorKind::UnexpectedEof)?;
    let second = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

    sc.drop_next_bytes("rtc_time".len())?.ok_or(ErrorKind::UnexpectedEof)?;
    sc.drop_next_until(": ")?.ok_or(ErrorKind::UnexpectedEof)?;

    let year = sc.next_i32_until("-")?.ok_or(ErrorKind::UnexpectedEof)?;
    let month = sc.next_u32_until("-")?.ok_or(ErrorKind::UnexpectedEof)?;
    let date = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

    Ok(NaiveDateTime::new(
        NaiveDate::from_ymd(year, month, date),
        NaiveTime::from_hms(hour, minute, second),
    ))
}
