use std::time::Duration;

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, proc_pid_path, read_single_record_file},
};

/// The rates computed between two `ProcessIO` instances.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessIOSpeed {
    /// Bytes read from the storage layer per second.
    pub read:  f64,
    /// Bytes written to the storage layer per second.
    pub write: f64,
}

/// I/O counters read from the `/proc/PID/io` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessIO {
    /// Bytes passed to read-like syscalls, including data served from the page cache.
    pub rchar:                 u64,
    /// Bytes passed to write-like syscalls.
    pub wchar:                 u64,
    /// The number of read-like syscalls.
    pub syscr:                 u64,
    /// The number of write-like syscalls.
    pub syscw:                 u64,
    /// Bytes actually read from the storage layer.
    pub read_bytes:            u64,
    /// Bytes actually written to the storage layer.
    pub write_bytes:           u64,
    /// Bytes counted in `write_bytes` whose writes were later cancelled (e.g. by truncation).
    pub cancelled_write_bytes: u64,
}

impl ProcessIO {
    /// Calculate the storage I/O speed (based on `read_bytes` and `write_bytes`) between two `ProcessIO` instances at different time.
    ///
    /// ```rust,no_run
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::process;
    ///
    /// let pid = std::process::id();
    ///
    /// let pre_process_io = process::get_process_io(pid).unwrap();
    ///
    /// let interval = Duration::from_millis(100);
    ///
    /// sleep(interval);
    ///
    /// let process_io = process::get_process_io(pid).unwrap();
    ///
    /// let process_io_speed = pre_process_io.compute_speed(&process_io, interval);
    ///
    /// println!("Read: {:.1} B/s", process_io_speed.read);
    /// println!("Write: {:.1} B/s", process_io_speed.write);
    /// ```
    #[inline]
    pub fn compute_speed(
        &self,
        process_io_after_this: &ProcessIO,
        interval: Duration,
    ) -> ProcessIOSpeed {
        let seconds = interval.as_secs_f64();

        if seconds <= 0.0 {
            return ProcessIOSpeed::default();
        }

        let d_read = process_io_after_this.read_bytes.saturating_sub(self.read_bytes);
        let d_write = process_io_after_this.write_bytes.saturating_sub(self.write_bytes);

        ProcessIOSpeed {
            read: d_read as f64 / seconds, write: d_write as f64 / seconds
        }
    }
}

/// Parse the content of a `/proc/PID/io` file, which is one `key: value` pair per line.
fn parse_process_io(data: &[u8]) -> Result<ProcessIO, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let mut process_io = ProcessIO::default();

    while let Some(label) = sc.next()? {
        let value = sc.next_u64()?.or_eof()?;

        match label {
            b"rchar:" => process_io.rchar = value,
            b"wchar:" => process_io.wchar = value,
            b"syscr:" => process_io.syscr = value,
            b"syscw:" => process_io.syscw = value,
            b"read_bytes:" => process_io.read_bytes = value,
            b"write_bytes:" => process_io.write_bytes = value,
            b"cancelled_write_bytes:" => process_io.cancelled_write_bytes = value,
            _ => (),
        }
    }

    Ok(process_io)
}

/// Get the I/O counters of a specific process found by ID by reading the `/proc/PID/io` file. The file needs `CONFIG_TASK_IO_ACCOUNTING`, and reading the file of a process owned by another user needs the `CAP_SYS_PTRACE` capability, otherwise a `PermissionDenied` error is returned.
///
/// ```rust,no_run
/// use mprober_lib::process;
///
/// let process_io = process::get_process_io(std::process::id()).unwrap();
///
/// println!("{process_io:#?}");
/// ```
#[inline]
pub fn get_process_io(pid: u32) -> Result<ProcessIO, Error> {
    parse_process_io(&read_single_record_file(proc_pid_path(pid).join("io"), 256)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IO: &[u8] = b"rchar: 3238612
wchar: 323177
syscr: 2214
syscw: 435
read_bytes: 4096000
write_bytes: 20480
cancelled_write_bytes: 4096
";

    #[test]
    fn parse() {
        let process_io = parse_process_io(IO).unwrap();

        assert_eq!(3238612, process_io.rchar);
        assert_eq!(323177, process_io.wchar);
        assert_eq!(2214, process_io.syscr);
        assert_eq!(435, process_io.syscw);
        assert_eq!(4096000, process_io.read_bytes);
        assert_eq!(20480, process_io.write_bytes);
        assert_eq!(4096, process_io.cancelled_write_bytes);
    }

    #[test]
    fn compute_speed() {
        let pre = parse_process_io(IO).unwrap();

        let mut post = pre.clone();

        post.read_bytes += 512;
        post.write_bytes += 256;

        let speed = pre.compute_speed(&post, Duration::from_millis(500));

        assert_eq!(1024.0, speed.read);
        assert_eq!(512.0, speed.write);
    }
}
