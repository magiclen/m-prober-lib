use std::sync::Once;

use chrono::prelude::*;

use crate::uptime::get_uptime;

/// Get the btime (boot time) by subtract the current uptime from the current unix epoch timestamp.
///
/// ```rust
/// use mprober_lib::btime;
///
/// let btime = btime::get_btime();
///
/// println!("{btime}");
/// ```
#[inline]
pub fn get_btime() -> DateTime<Utc> {
    static START: Once = Once::new();
    static mut BTIME: Option<DateTime<Utc>> = None;

    unsafe {
        START.call_once(|| BTIME = Some(get_uptime().unwrap().get_btime()));

        BTIME.unwrap()
    }
}
