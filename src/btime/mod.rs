extern crate chrono;

use std::time::SystemTime;

use crate::uptime::Uptime;

use chrono::prelude::*;

/// Get the btime (boot time) by subtract uptime from the current unix epoch timestamp.
#[inline]
pub fn get_btime_by_uptime(uptime: Uptime) -> DateTime<Utc> {
    (SystemTime::now() - uptime.total_uptime).into()
}
