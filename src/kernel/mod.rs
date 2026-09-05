use std::io::{self, ErrorKind};

use crate::scanner_rust::{ScannerAscii, ScannerError};

/// Get the kernel version by reading the `/proc/version` file.
///
/// ```rust
/// use mprober_lib::kernel;
///
/// let kernel_version = kernel::get_kernel_version().unwrap();
///
/// println!("{kernel_version}");
/// ```
#[inline]
pub fn get_kernel_version() -> Result<String, ScannerError> {
    let mut sc: ScannerAscii<_, 48> = ScannerAscii::scan_path2("/proc/version")?;

    sc.drop_next_bytes(14)?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let v = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(unsafe { String::from_utf8_unchecked(v) })
}
