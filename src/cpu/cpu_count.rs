use std::io;

use crate::Error;

#[inline]
fn sysconf_count(name: libc::c_int) -> Result<usize, Error> {
    let count = unsafe { libc::sysconf(name) };

    if count < 0 {
        return Err(io::Error::last_os_error().into());
    }

    Ok(count as usize)
}

/// Get the number of logical processors currently online using the `sysconf` function in libc.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let online_cpu_count = cpu::get_online_cpu_count().unwrap();
///
/// println!("{online_cpu_count}");
/// ```
#[inline]
pub fn get_online_cpu_count() -> Result<usize, Error> {
    sysconf_count(libc::_SC_NPROCESSORS_ONLN)
}

/// Get the number of logical processors configured in the system (including offline ones) using the `sysconf` function in libc.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let configured_cpu_count = cpu::get_configured_cpu_count().unwrap();
///
/// println!("{configured_cpu_count}");
/// ```
#[inline]
pub fn get_configured_cpu_count() -> Result<usize, Error> {
    sysconf_count(libc::_SC_NPROCESSORS_CONF)
}
