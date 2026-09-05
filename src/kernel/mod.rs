use std::io::{self, ErrorKind};

use crate::{
    Error,
    scanner_rust::ScannerAscii,
    utils::{read_single_record_file, read_sysfs_number, uname, utsname_field_to_string},
};

/// The system-wide file handle usage read from the `/proc/sys/fs/file-nr` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Get the highest PID the kernel assigns before wrapping around, by reading the `/proc/sys/kernel/pid_max` file. It is also the number of processes the system can have at most.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let pid_max = kernel::get_pid_max().unwrap();
///
/// println!("{pid_max}");
/// ```
#[inline]
pub fn get_pid_max() -> Result<u32, Error> {
    read_sysfs_number("/proc/sys/kernel/pid_max")
}

/// Get the number of threads the kernel allows in total, by reading the `/proc/sys/kernel/threads-max` file. The kernel derives the default from the size of the memory.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let threads_max = kernel::get_threads_max().unwrap();
///
/// println!("{threads_max}");
/// ```
#[inline]
pub fn get_threads_max() -> Result<u64, Error> {
    read_sysfs_number("/proc/sys/kernel/threads-max")
}

/// Get the parameters the bootloader passed to the kernel, by reading the `/proc/cmdline` file.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let cmdline = kernel::get_kernel_cmdline().unwrap();
///
/// println!("{cmdline:?}");
/// ```
pub fn get_kernel_cmdline() -> Result<Vec<String>, Error> {
    let data = read_single_record_file("/proc/cmdline", 1024)?;

    let cmdline = data
        .split(|b| b.is_ascii_whitespace())
        .filter(|parameter| !parameter.is_empty())
        .map(|parameter| String::from_utf8_lossy(parameter).into_owned())
        .collect();

    Ok(cmdline)
}

/// Get the taint flags of the kernel by reading the `/proc/sys/kernel/tainted` file. A value of `0` means the kernel is not tainted, and any other value means something happened that makes a bug report less trustworthy. Use [`get_kernel_taint_reasons`] to get the names of the flags.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let tainted = kernel::get_kernel_taint().unwrap();
///
/// println!("{tainted}");
/// ```
#[inline]
pub fn get_kernel_taint() -> Result<u64, Error> {
    read_sysfs_number("/proc/sys/kernel/tainted")
}

/// Get the names of the taint flags that are set in `tainted`, e.g. `proprietary-module` for a system with the NVIDIA driver loaded. The flags this crate does not know are left out.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let tainted = kernel::get_kernel_taint().unwrap();
///
/// println!("{:?}", kernel::get_kernel_taint_reasons(tainted));
/// ```
pub fn get_kernel_taint_reasons(tainted: u64) -> Vec<&'static str> {
    // The bit numbers are the `TAINT_*` constants of the kernel, which `Documentation/admin-guide/tainted-kernels.rst` lists together with the letter the kernel logs.
    const REASONS: [&str; 20] = [
        "proprietary-module",   // P
        "forced-module",        // F
        "cpu-out-of-spec",      // S
        "forced-module-unload", // R
        "machine-check",        // M
        "bad-page",             // B
        "user-requested",       // U
        "died-recently",        // D
        "acpi-overridden",      // A
        "warning",              // W
        "staging-driver",       // C
        "firmware-workaround",  // I
        "out-of-tree-module",   // O
        "unsigned-module",      // E
        "soft-lockup",          // L
        "live-patched",         // K
        "auxiliary",            // X
        "struct-randomized",    // T
        "in-kernel-test",       // N
        "fwctl",                // J
    ];

    REASONS
        .iter()
        .enumerate()
        .filter(|(bit, _)| tainted & (1 << bit) != 0)
        .map(|(_, reason)| *reason)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taint_reasons() {
        assert!(get_kernel_taint_reasons(0).is_empty());

        // Bit 0 is a proprietary module and bit 12 is an out-of-tree one.
        assert_eq!(
            vec!["proprietary-module", "out-of-tree-module"],
            get_kernel_taint_reasons(0x1001)
        );

        // A machine with the NVIDIA driver loaded reports bits 12 and 13.
        assert_eq!(vec!["out-of-tree-module", "unsigned-module"], get_kernel_taint_reasons(12288));

        // The bits this crate does not know are left out instead of shifting the rest.
        assert!(get_kernel_taint_reasons(1 << 40).is_empty());
    }
}
