use std::io::{self, ErrorKind};

use crate::{
    Error,
    scanner_rust::ScannerAscii,
    utils::{
        parse_number, read_file, read_single_record_file, read_sysfs_number, uname,
        utsname_field_to_string,
    },
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

/// Get the entropy available to the random number generator in bits, by reading the `/proc/sys/kernel/random/entropy_avail` file. Since Linux 5.6 the pool is considered full at `256`, and a system that stays far below that may block in `getrandom(2)` early after boot.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let entropy = kernel::get_entropy_available().unwrap();
///
/// println!("{entropy}");
/// ```
#[inline]
pub fn get_entropy_available() -> Result<u32, Error> {
    read_sysfs_number("/proc/sys/kernel/random/entropy_avail")
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

/// The system-wide inode usage read from the `/proc/sys/fs/inode-nr` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InodeNr {
    /// The number of allocated inodes.
    pub allocated: u64,
    /// The number of allocated inodes that are free.
    pub free:      u64,
}

/// Get the system-wide inode usage by reading the `/proc/sys/fs/inode-nr` file. These are the inodes of the in-memory cache, not the ones of a file system, which [`crate::volume::Volume`] reports instead.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let inode_nr = kernel::get_inode_nr().unwrap();
///
/// println!("{inode_nr:#?}");
/// ```
#[inline]
pub fn get_inode_nr() -> Result<InodeNr, Error> {
    let mut sc: ScannerAscii<_, 64> = ScannerAscii::scan_path2("/proc/sys/fs/inode-nr")?;

    let allocated = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let free = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(InodeNr {
        allocated,
        free,
    })
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

/// One loaded kernel module, read from the `/proc/modules` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KernelModule {
    /// The name of the module, e.g. `nvme`.
    pub name:       String,
    /// The size of the module in memory in bytes.
    pub size:       u64,
    /// How many other modules and open handles depend on this one. A module with `0` can be unloaded.
    pub used_by:    u64,
    /// The names of the modules that depend on this one.
    pub dependents: Vec<String>,
    /// The state of the module, which is `Live`, `Loading` or `Unloading`.
    pub state:      String,
}

/// Get the loaded kernel modules by reading the `/proc/modules` file, like the `lsmod` command.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let modules = kernel::get_modules().unwrap();
///
/// println!("{modules:#?}");
/// ```
pub fn get_modules() -> Result<Vec<KernelModule>, Error> {
    let data = read_file("/proc/modules", 32 * 1024)?;

    parse_modules(&data)
}

/// Parse the content of `/proc/modules`, whose lines look like `nvme 61440 4 nvme_core,dep, Live 0x0000000000000000`.
fn parse_modules(data: &[u8]) -> Result<Vec<KernelModule>, Error> {
    let mut modules = Vec::with_capacity(64);

    for line in data.split(|&b| b == b'\n') {
        let mut fields = line.split(|b| b.is_ascii_whitespace()).filter(|f| !f.is_empty());

        let (Some(name), Some(size), Some(used_by), Some(dependents), Some(state)) =
            (fields.next(), fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };

        // A module nothing depends on has a single hyphen in place of the list.
        let dependents = if dependents == b"-" {
            Vec::new()
        } else {
            dependents
                .split(|&b| b == b',')
                .filter(|dependent| !dependent.is_empty())
                .map(|dependent| String::from_utf8_lossy(dependent).into_owned())
                .collect()
        };

        modules.push(KernelModule {
            name: String::from_utf8_lossy(name).into_owned(),
            size: parse_number(size)?,
            used_by: parse_number(used_by)?,
            dependents,
            state: String::from_utf8_lossy(state).into_owned(),
        });
    }

    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_module_list() {
        const MODULES: &[u8] = b"nvme 61440 4 nvme_core,dep, Live 0x0000000000000000
nvme_core 200704 5 nvme Live 0x0000000000000000
btrfs 2039808 2 - Live 0x0000000000000000
";

        let modules = parse_modules(MODULES).unwrap();

        assert_eq!(3, modules.len());

        assert_eq!("nvme", modules[0].name);
        assert_eq!(61440, modules[0].size);
        assert_eq!(4, modules[0].used_by);
        assert_eq!(vec!["nvme_core", "dep"], modules[0].dependents);
        assert_eq!("Live", modules[0].state);

        // A module nothing depends on has a hyphen instead of a list.
        assert_eq!("btrfs", modules[2].name);
        assert!(modules[2].dependents.is_empty());
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
