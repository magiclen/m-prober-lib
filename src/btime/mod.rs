extern crate chrono;

use std::sync::Once;

use crate::uptime::get_uptime;

use chrono::prelude::*;

/// Get the btime (boot time) by subtract the current uptime from the current unix epoch timestamp.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::btime;
///
/// let btime = btime::get_btime();
///
/// println!("{}", btime);
/// ```
#[inline]
pub fn get_btime() -> &'static DateTime<Utc> {
    static START: Once = Once::new();
    static mut BTIME: Option<DateTime<Utc>> = None;

    unsafe {
        START.call_once(|| BTIME = Some(get_uptime().unwrap().get_btime()));

        BTIME.as_ref().unwrap()
    }
}
