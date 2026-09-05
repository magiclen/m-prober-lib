use std::time::Duration;

/// Get the number of clock ticks per second (`USER_HZ`), which is the unit of the time fields in `/proc`.
#[inline]
pub(crate) fn clock_ticks_per_second() -> u64 {
    // libc caches this value from the auxiliary vector, so calling it repeatedly costs no syscall.
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };

    if ticks > 0 { ticks as u64 } else { 100 }
}

/// Convert clock ticks to a `Duration` without going through floating point.
#[inline]
pub(crate) fn clock_ticks_to_duration(ticks: u64) -> Duration {
    let ticks_per_second = clock_ticks_per_second();

    let seconds = ticks / ticks_per_second;
    let nanos = (ticks % ticks_per_second) * 1_000_000_000 / ticks_per_second;

    Duration::new(seconds, nanos as u32)
}
