use std::{
    io::{self, ErrorKind},
    path::Path,
    time::Duration,
};

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct ProcessIOSpeed {
    /// Bytes read from the storage layer per second.
    pub read:  f64,
    /// Bytes written to the storage layer per second.
    pub write: f64,
}

/// I/O counters read from the `/proc/PID/io` file.
#[derive(Default, Debug, Clone)]
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
    /// ```rust
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

        let d_read = process_io_after_this.read_bytes.saturating_sub(self.read_bytes);
        let d_write = process_io_after_this.write_bytes.saturating_sub(self.write_bytes);

        ProcessIOSpeed {
            read: d_read as f64 / seconds, write: d_write as f64 / seconds
        }
    }
}

/// Get the I/O counters of a specific process found by ID by reading the `/proc/PID/io` file. Reading the file of a process owned by another user needs the `CAP_SYS_PTRACE` capability, otherwise a `PermissionDenied` error is returned.
///
/// ```rust
/// use mprober_lib::process;
///
/// let process_io = process::get_process_io(std::process::id()).unwrap();
///
/// println!("{process_io:#?}");
/// ```
pub fn get_process_io(pid: u32) -> Result<ProcessIO, ScannerError> {
    let io_path = Path::new("/proc").join(pid.to_string()).join("io");

    let mut sc: ScannerAscii<_, 192> = ScannerAscii::scan_path2(io_path)?;

    let mut process_io = ProcessIO::default();

    while let Some(label) = sc.next_raw()? {
        let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        match label.as_slice() {
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
