use std::borrow::Cow;

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_single_record_file, read_sysfs_number, uname, utsname_field_to_string},
};

/// The system-wide file handle usage read from the `/proc/sys/fs/file-nr` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileNr {
    /// The number of allocated file handles.
    pub allocated: u64,
    /// The maximum number of file handles.
    pub max:       u64,
}

/// Parse the content of `/proc/sys/fs/file-nr`, which is three numbers on one line.
fn parse_file_nr(data: &[u8]) -> Result<FileNr, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let allocated = sc.next_u64()?.or_eof()?;

    // The second field is the number of unused handles, which has been `0` since Linux 2.6.
    sc.drop_next()?.or_eof()?;

    let max = sc.next_u64()?.or_eof()?;

    Ok(FileNr {
        allocated,
        max,
    })
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
    parse_file_nr(&read_single_record_file("/proc/sys/fs/file-nr", 64)?)
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

/// Get the highest PID the kernel assigns before wrapping around, by reading the `/proc/sys/kernel/pid_max` file. Every process and every thread takes a PID below it, so it also caps how many of them can exist at the same time.
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

    Ok(split_kernel_cmdline(&data))
}

/// Drop the double quotes that the kernel strips from a parameter. They surround either the whole parameter or only the value that follows the first `=`.
fn unquote_kernel_parameter(parameter: &[u8]) -> Cow<'_, [u8]> {
    let quote = if parameter.first() == Some(&b'"') {
        0
    } else {
        match parameter.iter().position(|&b| b == b'=') {
            Some(equals) if parameter.get(equals + 1) == Some(&b'"') => equals + 1,
            _ => return Cow::Borrowed(parameter),
        }
    };

    // The opening quote needs a closing one, and both have to fit.
    if parameter.len() < quote + 2 || parameter.last() != Some(&b'"') {
        return Cow::Borrowed(parameter);
    }

    let mut result = Vec::with_capacity(parameter.len() - 2);

    result.extend_from_slice(&parameter[..quote]);
    result.extend_from_slice(&parameter[(quote + 1)..(parameter.len() - 1)]);

    Cow::Owned(result)
}

/// Split the content of `/proc/cmdline` into parameters, the way `next_arg` in `lib/cmdline.c` does: a parameter ends at whitespace that is not inside double quotes, so a quoted value keeps its spaces.
fn split_kernel_cmdline(data: &[u8]) -> Vec<String> {
    let mut cmdline = Vec::new();

    let mut i = 0;

    while i < data.len() {
        if data[i].is_ascii_whitespace() {
            i += 1;

            continue;
        }

        let start = i;

        let mut in_quote = false;

        while i < data.len() {
            let byte = data[i];

            if byte.is_ascii_whitespace() && !in_quote {
                break;
            }

            if byte == b'"' {
                in_quote = !in_quote;
            }

            i += 1;
        }

        // A parameter is arbitrary bytes, so it may not be valid UTF-8.
        cmdline
            .push(String::from_utf8_lossy(&unquote_kernel_parameter(&data[start..i])).into_owned());
    }

    cmdline
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
    fn parse_file_handles() {
        let file_nr = parse_file_nr(b"9280\t0\t9223372036854775807\n".as_slice()).unwrap();

        assert_eq!(9280, file_nr.allocated);
        assert_eq!(9223372036854775807, file_nr.max);
    }

    #[test]
    fn split_cmdline() {
        // A machine without any quoted parameter is split exactly as before.
        assert_eq!(
            vec!["BOOT_IMAGE=/vmlinuz-6.17.0-40-generic", "root=UUID=7fa5eea6", "ro", "quiet"],
            split_kernel_cmdline(
                b"BOOT_IMAGE=/vmlinuz-6.17.0-40-generic root=UUID=7fa5eea6 ro quiet\n"
            )
        );

        // Whitespace inside quotes does not end a parameter, and the kernel drops the quotes.
        assert_eq!(vec!["a=1", "b=x y", "c=2"], split_kernel_cmdline(b"a=1 b=\"x y\" c=2\n"));
        assert_eq!(vec!["a b"], split_kernel_cmdline(b"\"a b\"\n"));

        // A quote that surrounds neither the parameter nor its whole value is kept.
        assert_eq!(vec!["k=a\"b\"c"], split_kernel_cmdline(b"k=a\"b\"c\n"));

        assert!(split_kernel_cmdline(b"\n").is_empty());
    }

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
