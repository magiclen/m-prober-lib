use std::io::{self, ErrorKind};

use crate::{
    Error,
    scanner_rust::ScannerAscii,
    utils::{uname, utsname_field_to_string},
};

/// The system-wide file handle usage read from the `/proc/sys/fs/file-nr` file.
#[derive(Default, Debug, Clone)]
pub struct FileNr {
    /// The number of allocated file handles.
    pub allocated: u64,
    /// The number of allocated but unused file handles (always `0` since Linux 2.6).
    pub unused:    u64,
    /// The maximum number of file handles.
    pub max:       u64,
}

/// Get the system-wide file handle usage by reading the `/proc/sys/fs/file-nr` file.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let file_nr = kernel::get_file_nr().unwrap();
///
/// println!("{file_nr:#?}");
/// ```
#[inline]
pub fn get_file_nr() -> Result<FileNr, Error> {
    let mut sc: ScannerAscii<_, 64> = ScannerAscii::scan_path2("/proc/sys/fs/file-nr")?;

    let allocated = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let unused = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let max = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(FileNr {
        allocated,
        unused,
        max,
    })
}

/// The fields that the `uname` function in libc reports.
#[derive(Default, Debug, Clone)]
pub struct Uname {
    /// The operating system name, e.g. `Linux`.
    pub sysname:  String,
    /// The hostname.
    pub nodename: String,
    /// The kernel release, e.g. `6.17.0-40-generic`.
    pub release:  String,
    /// The kernel version, e.g. `#40~24.04.1-Ubuntu SMP PREEMPT_DYNAMIC Tue Jun 23 16:48:12 UTC 2`.
    pub version:  String,
    /// The hardware name, e.g. `x86_64`.
    pub machine:  String,
}

/// Get the system information using the `uname` function in libc.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let uname = kernel::get_uname().unwrap();
///
/// println!("{uname:#?}");
/// ```
#[inline]
pub fn get_uname() -> Result<Uname, Error> {
    let buffer = uname()?;

    Ok(Uname {
        sysname:  utsname_field_to_string(&buffer.sysname),
        nodename: utsname_field_to_string(&buffer.nodename),
        release:  utsname_field_to_string(&buffer.release),
        version:  utsname_field_to_string(&buffer.version),
        machine:  utsname_field_to_string(&buffer.machine),
    })
}

/// Get the kernel version (the `release` field of `uname`) using the `uname` function in libc.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let kernel_version = kernel::get_kernel_version().unwrap();
///
/// println!("{kernel_version}");
/// ```
#[inline]
pub fn get_kernel_version() -> Result<String, Error> {
    let buffer = uname()?;

    Ok(utsname_field_to_string(&buffer.release))
}
