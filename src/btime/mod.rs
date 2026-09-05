use std::{
    mem::zeroed,
    time::{Duration, UNIX_EPOCH},
};

use chrono::prelude::*;

use crate::uptime::get_uptime;

#[inline]
fn clock_gettime(clock_id: libc::clockid_t) -> Option<Duration> {
    let mut ts: libc::timespec = unsafe { zeroed() };

    let rtn = unsafe { libc::clock_gettime(clock_id, &mut ts) };

    if rtn != 0 || ts.tv_sec < 0 {
        return None;
    }

    Some(Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32))
}

/// Get the btime (boot time) by subtracting `CLOCK_BOOTTIME` from `CLOCK_REALTIME` using the `clock_gettime` function in libc. If those clocks are unavailable, the `/proc/uptime` file is used instead.
///
/// The value follows the system clock, so it changes when the clock is adjusted (e.g. by NTP), like the `btime` field of the `/proc/stat` file. Both clocks are read through the vDSO, so this function is cheap enough to be called every time.
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
    match (clock_gettime(libc::CLOCK_REALTIME), clock_gettime(libc::CLOCK_BOOTTIME)) {
        (Some(realtime), Some(boottime)) => (UNIX_EPOCH + realtime.saturating_sub(boottime)).into(),
        _ => match get_uptime() {
            Ok(uptime) => uptime.get_btime(),
            Err(_) => UNIX_EPOCH.into(),
        },
    }
}
