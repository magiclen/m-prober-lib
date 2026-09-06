use std::io::{self, ErrorKind};

use chrono::prelude::*;
use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_single_record_file},
};

/// Parse the content of `/proc/driver/rtc`, whose lines look like `rtc_time\t: 12:42:30`.
fn parse_rtc_date_time(data: &[u8]) -> Result<NaiveDateTime, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    // The labels are searched for explicitly, so the order and the width of the fields do not matter.
    sc.drop_next_until("rtc_time")?.or_eof()?;
    sc.drop_next_until(": ")?.or_eof()?;

    let hour = sc.next_u32_until(":")?.or_eof()?;
    let minute = sc.next_u32_until(":")?.or_eof()?;
    let second = sc.next_u32()?.or_eof()?;

    sc.drop_next_until("rtc_date")?.or_eof()?;
    sc.drop_next_until(": ")?.or_eof()?;

    let year = sc.next_i32_until("-")?.or_eof()?;
    let month = sc.next_u32_until("-")?.or_eof()?;
    let date = sc.next_u32()?.or_eof()?;

    let date = NaiveDate::from_ymd_opt(year, month, date)
        .ok_or(io::Error::from(ErrorKind::InvalidData))?;
    let time = NaiveTime::from_hms_opt(hour, minute, second)
        .ok_or(io::Error::from(ErrorKind::InvalidData))?;

    Ok(NaiveDateTime::new(date, time))
}

/// Get the RTC datetime by reading the `/proc/driver/rtc` file. The RTC is normally set to UTC, but the file carries no timezone, so a `NaiveDateTime` is returned. The file only exists when an RTC driver is loaded, which is not the case in most containers.
///
/// ```rust,no_run
/// use mprober_lib::rtc_time;
///
/// let rtc_date_time = rtc_time::get_rtc_date_time().unwrap();
///
/// println!("{rtc_date_time}");
/// ```
#[inline]
pub fn get_rtc_date_time() -> Result<NaiveDateTime, Error> {
    parse_rtc_date_time(&read_single_record_file("/proc/driver/rtc", 512)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RTC: &[u8] = b"rtc_time\t: 12:42:30
rtc_date\t: 2026-09-06
alrm_time\t: 00:00:00
alrm_date\t: 2026-09-07
alarm_IRQ\t: no
24hr\t\t: yes
";

    #[test]
    fn parse() {
        let date_time = parse_rtc_date_time(RTC).unwrap();

        // The alarm lines that follow must not be picked up instead of the time and the date.
        assert_eq!("2026-09-06 12:42:30", date_time.to_string());
    }
}
