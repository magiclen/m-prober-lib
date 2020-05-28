extern crate chrono;

use std::io::ErrorKind;

use crate::scanner_rust::{ScannerAscii, ScannerError};

use chrono::prelude::*;

/// Get the uptime by reading the `/sys/class/rtc/rtc0/date` file and the `/sys/class/rtc/rtc0/time` file.
#[inline]
pub fn get_rtc_date_time() -> Result<NaiveDateTime, ScannerError> {
    let rtc_date = {
        let mut sc = ScannerAscii::scan_path("/sys/class/rtc/rtc0/date")?;

        let year = sc.next_i32_until("-")?.ok_or(ErrorKind::UnexpectedEof)?;

        let month = sc.next_u32_until("-")?.ok_or(ErrorKind::UnexpectedEof)?;

        let date = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

        NaiveDate::from_ymd(year, month, date)
    };

    let rtc_time = {
        let mut sc = ScannerAscii::scan_path("/sys/class/rtc/rtc0/time")?;

        let hour = sc.next_u32_until(":")?.ok_or(ErrorKind::UnexpectedEof)?;

        let minute = sc.next_u32_until(":")?.ok_or(ErrorKind::UnexpectedEof)?;

        let second = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

        NaiveTime::from_hms(hour, minute, second)
    };

    Ok(NaiveDateTime::new(rtc_date, rtc_time))
}
