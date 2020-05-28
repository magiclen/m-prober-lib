use std::io::ErrorKind;

use crate::scanner_rust::{ScannerAscii, ScannerError};

/// Get the kernel version by reading the `/proc/version` file.
#[inline]
pub fn get_kernel_version() -> Result<String, ScannerError> {
    let mut sc = ScannerAscii::scan_path("/proc/version")?;

    sc.drop_next_bytes(14)?.ok_or(ErrorKind::UnexpectedEof)?;

    let v = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

    Ok(unsafe { String::from_utf8_unchecked(v) })
}
