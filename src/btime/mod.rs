use std::{
    mem::zeroed,
    sync::OnceLock,
    time::{Duration, UNIX_EPOCH},
};

use chrono::prelude::*;

#[inline]
fn clock_gettime(clock_id: libc::clockid_t) -> Duration {
    let mut ts: libc::timespec = unsafe { zeroed() };

    // `CLOCK_REALTIME` and `CLOCK_BOOTTIME` always exist on Linux, so this call cannot fail.
    unsafe {
        libc::clock_gettime(clock_id, &mut ts);
    }

    Duration::new(ts.tv_sec.max(0) as u64, ts.tv_nsec as u32)
}

/// Get the btime (boot time) by subtracting `CLOCK_BOOTTIME` from `CLOCK_REALTIME` using the `clock_gettime` function in libc. The result is cached after the first call.
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
    static BTIME: OnceLock<DateTime<Utc>> = OnceLock::new();

    *BTIME.get_or_init(|| {
        let realtime = clock_gettime(libc::CLOCK_REALTIME);
        let boottime = clock_gettime(libc::CLOCK_BOOTTIME);

        (UNIX_EPOCH + realtime.saturating_sub(boottime)).into()
    })
}
