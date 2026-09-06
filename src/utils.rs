use std::{
    borrow::Cow,
    fs::{self, File},
    io::{self, ErrorKind, Read},
    path::{Path, PathBuf},
    str::{FromStr, from_utf8},
    time::Duration,
};

use crate::Error;

/// A shorthand for the error that every parser of this crate returns when a file ends in the middle of a record, which the scanners report as a `None`.
pub(crate) trait OrEof<T> {
    fn or_eof(self) -> Result<T, Error>;
}

impl<T> OrEof<T> for Option<T> {
    #[inline]
    fn or_eof(self) -> Result<T, Error> {
        self.ok_or_else(|| io::Error::from(ErrorKind::UnexpectedEof).into())
    }
}

/// Parse a number from ASCII bytes.
#[inline]
pub(crate) fn parse_number<T: FromStr>(value: &[u8]) -> Result<T, Error>
where
    Error: From<T::Err>, {
    let s = from_utf8(value).map_err(|_| io::Error::from(ErrorKind::InvalidData))?;

    Ok(s.parse::<T>()?)
}

/// Decode the octal escape sequences that the kernel writes in `/proc/mounts` and `/proc/self/mountinfo`, such as `\040` for a space. Almost no field holds one, so a field without a backslash is borrowed instead of copied.
pub(crate) fn unescape_octal(data: &[u8]) -> Cow<'_, [u8]> {
    if !data.contains(&b'\\') {
        return Cow::Borrowed(data);
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

    Cow::Owned(result)
}

/// Parse a CPU list like `0-3,8`, which is the format sysfs uses for `cpulist` and `cpuset.cpus`.
pub(crate) fn parse_cpu_list(list: &str) -> Vec<usize> {
    let mut cpus = Vec::new();

    for part in list.trim().split(',') {
        if part.is_empty() {
            continue;
        }

        match part.split_once('-') {
            Some((start, end)) => {
                if let (Ok(start), Ok(end)) = (start.parse::<usize>(), end.parse::<usize>()) {
                    cpus.extend(start..=end);
                }
            },
            None => {
                if let Ok(cpu) = part.parse() {
                    cpus.push(cpu);
                }
            },
        }
    }

    cpus
}

/// Read a small file that holds a single value, e.g. a sysfs attribute, a `/proc/sys` knob or `/etc/machine-id`, and return its content without the trailing whitespace.
#[inline]
pub(crate) fn read_sysfs_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let data = read_single_record_file(path, 64)?;

    Ok(String::from_utf8_lossy(data.trim_ascii_end()).into_owned())
}

/// Read a small file that holds a single value and parse its content as a number.
#[inline]
pub(crate) fn read_sysfs_number<T: FromStr, P: AsRef<Path>>(path: P) -> Result<T, Error>
where
    Error: From<T::Err>, {
    let data = read_single_record_file(path, 64)?;

    parse_number(data.trim_ascii_end())
}

/// Read a sysfs flag, which the kernel writes as `0` or `1`.
#[inline]
pub(crate) fn read_sysfs_bool<P: AsRef<Path>>(path: P) -> Option<bool> {
    read_sysfs_number::<u8, _>(path).ok().map(|flag| flag == 1)
}

/// Read a `cpulist` attribute, which looks like `0-3,8`. An attribute that cannot be read lists no processor.
#[inline]
pub(crate) fn read_cpu_list<P: AsRef<Path>>(path: P) -> Vec<usize> {
    read_sysfs_string(path).map(|list| parse_cpu_list(&list)).unwrap_or_default()
}

/// Read a symbolic link and return the name of its target, e.g. `r8169` for the `device/driver` link of a network interface.
#[inline]
pub(crate) fn read_link_name<P: AsRef<Path>>(path: P) -> Option<String> {
    let target = fs::read_link(path).ok()?;

    Some(target.file_name()?.to_string_lossy().into_owned())
}

/// Read a symbolic link and return the name of the folder its target lives in, e.g. `nvme0n1` for the link of the partition `nvme0n1p1`.
#[inline]
pub(crate) fn read_link_parent_name<P: AsRef<Path>>(path: P) -> Option<String> {
    let target = fs::read_link(path).ok()?;

    Some(target.parent()?.file_name()?.to_string_lossy().into_owned())
}

/// Read a sysfs value that the driver reports in thousandths, like millidegrees or millivolts.
#[inline]
pub(crate) fn read_sysfs_milli<P: AsRef<Path>>(path: P) -> Option<f64> {
    read_sysfs_number::<i64, _>(path).ok().map(|value| value as f64 / 1000.0)
}

/// Read a sysfs value that the driver reports in millionths, like microwatts or microvolts.
#[inline]
pub(crate) fn read_sysfs_micro<P: AsRef<Path>>(path: P) -> Option<f64> {
    read_sysfs_number::<i64, _>(path).ok().map(|value| value as f64 / 1_000_000.0)
}

/// Build the path of the `/proc/PID` folder of a process.
#[inline]
pub(crate) fn proc_pid_path(pid: u32) -> PathBuf {
    Path::new("/proc").join(pid.to_string())
}

/// Check that a name from the caller is a single path component, so that it cannot escape the folder it is joined to.
#[inline]
pub(crate) fn is_single_path_component(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/')
}

// Every text file of `/proc` and `/sys` is read into a buffer by one of the two functions below and then parsed with a `ScannerU8SliceAscii`, whose tokens borrow from that buffer instead of being allocated one by one. Splitting bytes by hand is only kept where the input is not a series of whitespace-separated tokens, e.g. `unescape_octal`, `parse_cpu_list` and the hexadecimal addresses of the socket files.

/// Read a whole file into a `Vec` with a pre-allocated capacity. Multi-record files in `/proc` (e.g. `/proc/cpuinfo`) return at most one page per read, so this reads until EOF.
#[inline]
pub(crate) fn read_file<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut buffer = Vec::with_capacity(capacity);

    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

/// Read the beginning of a file with a single read, without going on to its end. This is for a file whose first line is all the caller needs, e.g. the `cpu` line of `/proc/stat`, which the kernel follows with one line per processor and hundreds of interrupt counters.
#[inline]
pub(crate) fn read_file_head<P: AsRef<Path>>(path: P, max: usize) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;

    let mut buffer = vec![0u8; max];

    let size = file.read(&mut buffer)?;

    buffer.truncate(size);

    Ok(buffer)
}

/// Read a file that the kernel generates as a single record (e.g. `/proc/PID/stat` or a sysfs attribute). Such a file is returned completely by one read when it fits into `capacity` bytes, so the extra read for EOF is skipped. A larger file is still read completely.
#[inline]
pub(crate) fn read_single_record_file<P: AsRef<Path>>(
    path: P,
    capacity: usize,
) -> io::Result<Vec<u8>> {
    let mut buffer = Vec::new();

    read_single_record_file_into(path, capacity, &mut buffer)?;

    Ok(buffer)
}

/// Read a single-record file into a buffer the caller owns, which is left holding exactly the content. A caller that reads one such file per process keeps one buffer for the whole scan this way, instead of allocating and zeroing a new one every time.
#[inline]
pub(crate) fn read_single_record_file_into<P: AsRef<Path>>(
    path: P,
    capacity: usize,
    buffer: &mut Vec<u8>,
) -> io::Result<()> {
    let mut file = File::open(path)?;

    buffer.clear();
    buffer.resize(capacity, 0);

    let size = file.read(buffer)?;

    if size == capacity {
        // The record may be larger than the buffer, so the rest is read in the usual way.
        file.read_to_end(buffer)?;
    } else {
        buffer.truncate(size);
    }

    Ok(())
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
    fn parse_cpu_lists() {
        assert_eq!(vec![0, 1, 2, 3, 8], parse_cpu_list("0-3,8"));
        assert_eq!(vec![5], parse_cpu_list("5\n"));
        assert_eq!(vec![0, 1, 4, 5, 6], parse_cpu_list("0-1,4-6"));
        assert!(parse_cpu_list("").is_empty());
    }

    #[test]
    fn unescape_octal_sequences() {
        assert_eq!(b"/mnt/my disk".as_slice(), &*unescape_octal(b"/mnt/my\\040disk"));
        assert_eq!(b"/mnt/a\tb".as_slice(), &*unescape_octal(b"/mnt/a\\011b"));
        assert_eq!(b"/mnt/a\\b".as_slice(), &*unescape_octal(b"/mnt/a\\134b"));
        assert_eq!(b"/mnt/plain".as_slice(), &*unescape_octal(b"/mnt/plain"));

        // An incomplete or non-octal sequence is kept as it is.
        assert_eq!(b"/mnt/a\\09b".as_slice(), &*unescape_octal(b"/mnt/a\\09b"));
        assert_eq!(b"/mnt/a\\04".as_slice(), &*unescape_octal(b"/mnt/a\\04"));

        // A field without a backslash is borrowed instead of copied.
        assert!(matches!(unescape_octal(b"/mnt/plain"), Cow::Borrowed(_)));
    }
}
