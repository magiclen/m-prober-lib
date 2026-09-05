use std::{
    io,
    mem::zeroed,
    time::{Duration, UNIX_EPOCH},
};

use chrono::prelude::*;

#[inline]
fn clock_gettime(clock_id: libc::clockid_t) -> Duration {
    let mut ts: libc::timespec = unsafe { zeroed() };

    let rtn = unsafe { libc::clock_gettime(clock_id, &mut ts) };

    // Both clocks exist on every kernel this crate supports, so a failure is a bug, and std panics in the same situation in `SystemTime::now`.
    assert_eq!(0, rtn, "clock_gettime failed: {}", io::Error::last_os_error());

    Duration::new(u64::try_from(ts.tv_sec).unwrap_or(0), ts.tv_nsec as u32)
}

/// Get the btime (boot time) by subtracting `CLOCK_BOOTTIME` from `CLOCK_REALTIME` using the `clock_gettime` function in libc.
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
    let realtime = clock_gettime(libc::CLOCK_REALTIME);
    let boottime = clock_gettime(libc::CLOCK_BOOTTIME);

    (UNIX_EPOCH + realtime.saturating_sub(boottime)).into()
}
