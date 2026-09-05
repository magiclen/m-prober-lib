use crate::{
    scanner_rust::ScannerError,
    utils::{uname, utsname_field_to_string},
};

/// Get the hostname (the `nodename` field of `uname`) using the `uname` function in libc.
///
/// ```rust
/// use mprober_lib::hostname;
///
/// let hostname = hostname::get_hostname().unwrap();
///
/// println!("{hostname}");
/// ```
#[inline]
pub fn get_hostname() -> Result<String, ScannerError> {
    let buffer = uname()?;

    Ok(utsname_field_to_string(&buffer.nodename))
}
