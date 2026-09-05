use std::{io, mem::zeroed};

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

/// Get the number of logical processors the current process may actually run on, using the `sched_getaffinity` function in libc. Unlike [`get_online_cpu_count`], this respects the CPU affinity mask, so it is the right number in a container or under the `taskset` command. It does not respect the CPU quota of a cgroup, which [`crate::cgroup::CgroupCPU::effective_cpu_count`] reports instead.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let available_cpu_count = cpu::get_available_cpu_count().unwrap();
///
/// println!("{available_cpu_count}");
/// ```
pub fn get_available_cpu_count() -> Result<usize, Error> {
    let mut set: libc::cpu_set_t = unsafe { zeroed() };

    // A zero PID means the calling thread.
    let rtn = unsafe { libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &mut set) };

    if rtn != 0 {
        return Err(io::Error::last_os_error().into());
    }

    let count = unsafe { libc::CPU_COUNT(&set) };

    Ok(count as usize)
}
