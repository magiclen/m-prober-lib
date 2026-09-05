use std::{
    fs::File,
    io::{self, ErrorKind, Read},
    path::Path,
    str::{FromStr, from_utf8},
    time::Duration,
};

use crate::scanner_rust::ScannerError;

/// Parse a number from ASCII bytes.
#[inline]
pub(crate) fn parse_number<T: FromStr>(value: &[u8]) -> Result<T, ScannerError>
where
    ScannerError: From<T::Err>, {
    let s = from_utf8(value).map_err(|_| io::Error::from(ErrorKind::InvalidData))?;

    Ok(s.parse::<T>()?)
}

/// Decode the octal escape sequences that the kernel writes in `/proc/mounts` and `/proc/self/mountinfo`, such as `\040` for a space.
pub(crate) fn unescape_octal(data: &[u8]) -> Vec<u8> {
    if !data.contains(&b'\\') {
        return data.to_vec();
    }

    let mut result = Vec::with_capacity(data.len());

    let mut i = 0;

    while i < data.len() {
        // An escape sequence is a backslash followed by exactly three octal digits.
        if data[i] == b'\\'
            && let Some(digits) = data.get((i + 1)..(i + 4))
            && digits.iter().all(|b| (b'0'..=b'7').contains(b))
        {
            let value = digits.iter().fold(0u16, |acc, b| acc * 8 + u16::from(b - b'0'));

            if value <= u16::from(u8::MAX) {
                result.push(value as u8);

                i += 4;

                continue;
            }
        }

        result.push(data[i]);

        i += 1;
    }

    result
}

/// Read a small sysfs file and return its content without the trailing whitespace.
#[inline]
pub(crate) fn read_sysfs_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let data = read_single_record_file(path, 64)?;

    Ok(String::from_utf8_lossy(data.trim_ascii_end()).into_owned())
}

/// Read a small sysfs file and parse its content as a number.
#[inline]
pub(crate) fn read_sysfs_number<T: FromStr, P: AsRef<Path>>(path: P) -> Result<T, ScannerError>
where
    ScannerError: From<T::Err>, {
    let data = read_single_record_file(path, 64)?;

    parse_number(data.trim_ascii_end())
}

/// Read a whole file into a `Vec` with a pre-allocated capacity. Multi-record files in `/proc` (e.g. `/proc/cpuinfo`) return at most one page per read, so this reads until EOF.
#[inline]
pub(crate) fn read_file<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut buffer = Vec::with_capacity(capacity);

    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

/// Read a file that the kernel generates as a single record (e.g. `/proc/PID/stat` or a sysfs attribute). Such a file is returned completely by one read when it fits into `capacity` bytes, so the extra read for EOF is skipped. A larger file is still read completely.
#[inline]
pub(crate) fn read_single_record_file<P: AsRef<Path>>(
    path: P,
    capacity: usize,
) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut buffer = vec![0u8; capacity];

    let size = file.read(&mut buffer)?;

    if size == capacity {
        // The record may be larger than the buffer, so the rest is read in the usual way.
        file.read_to_end(&mut buffer)?;
    } else {
        buffer.truncate(size);
    }

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
    // `c_char` is signed on x86 and unsigned on ARM, so every byte is cast instead of transmuting the slice.
    #[allow(clippy::unnecessary_cast)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_octal_sequences() {
        assert_eq!(b"/mnt/my disk".to_vec(), unescape_octal(b"/mnt/my\\040disk"));
        assert_eq!(b"/mnt/a\tb".to_vec(), unescape_octal(b"/mnt/a\\011b"));
        assert_eq!(b"/mnt/a\\b".to_vec(), unescape_octal(b"/mnt/a\\134b"));
        assert_eq!(b"/mnt/plain".to_vec(), unescape_octal(b"/mnt/plain"));

        // An incomplete or non-octal sequence is kept as it is.
        assert_eq!(b"/mnt/a\\09b".to_vec(), unescape_octal(b"/mnt/a\\09b"));
        assert_eq!(b"/mnt/a\\04".to_vec(), unescape_octal(b"/mnt/a\\04"));
    }
}
