use std::io;

use crate::scanner_rust::ScannerError;

/// Get the hostname using the `gethostname` function in libc.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::hostname;
///
/// let hostname = hostname::get_hostname().unwrap();
///
/// println!("{}", hostname);
/// ```
#[inline]
pub fn get_hostname() -> Result<String, ScannerError> {
    let buffer_size = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) } as usize;

    let mut buffer: Vec<u8> = Vec::with_capacity(buffer_size);

    unsafe {
        buffer.set_len(buffer_size);
    }

    let c = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut libc::c_char, buffer_size) }
        as usize;

    if c != 0 {
        return Err(io::Error::last_os_error().into());
    }

    if let Some(end) = buffer.iter().copied().position(|e| e == b'\0') {
        unsafe {
            buffer.set_len(end);
        }
    }

    Ok(unsafe { String::from_utf8_unchecked(buffer) })
}
