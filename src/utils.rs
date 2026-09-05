use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    time::Duration,
};

/// Read a whole file into a `Vec` with a pre-allocated capacity, so small `/proc` files usually need only one read syscall.
#[inline]
pub(crate) fn read_file<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut buffer = Vec::with_capacity(capacity);

    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

/// Call `uname(2)` and return the raw struct.
#[inline]
pub(crate) fn uname() -> io::Result<libc::utsname> {
    let mut buffer: libc::utsname = unsafe { std::mem::zeroed() };

    let rtn = unsafe { libc::uname(&mut buffer) };

    if rtn != 0 {
        return Err(io::Error::last_os_error());
    }

    Ok(buffer)
}

/// Convert a NUL-terminated field of `utsname` to a `String`.
#[inline]
pub(crate) fn utsname_field_to_string(field: &[libc::c_char]) -> String {
    // `c_char` is signed on some targets, so every byte is cast instead of transmuting the slice.
    let bytes: Vec<u8> = field.iter().map(|&c| c as u8).take_while(|&b| b != 0).collect();

    String::from_utf8_lossy(&bytes).into_owned()
}

/// Get the page size in bytes.
#[inline]
pub(crate) fn page_size() -> usize {
    // libc caches this value from the auxiliary vector, so calling it repeatedly costs no syscall.
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

    if size > 0 { size as usize } else { 4096 }
}

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
