use std::{
    io::{self, ErrorKind},
    mem::zeroed,
};

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

    if rtn == 0 {
        return Ok(unsafe { libc::CPU_COUNT(&set) } as usize);
    }

    let err = io::Error::last_os_error();

    // The kernel rejects a mask that cannot hold every CPU it knows about, which is what happens on a machine with more than the 1024 processors a `cpu_set_t` covers.
    if err.raw_os_error() != Some(libc::EINVAL) {
        return Err(err.into());
    }

    get_available_cpu_count_with_large_mask()
}

/// Ask for the affinity mask again with a buffer that keeps doubling until the kernel accepts it. Only a machine with more processors than a `cpu_set_t` can hold reaches this.
#[cold]
fn get_available_cpu_count_with_large_mask() -> Result<usize, Error> {
    // This covers far more processors than any kernel supports, so the loop always ends.
    const MAX_WORDS: usize = 16 * 1024;

    let mut words = size_of::<libc::cpu_set_t>() / size_of::<libc::c_ulong>();

    loop {
        words *= 2;

        if words > MAX_WORDS {
            return Err(io::Error::from(ErrorKind::InvalidData).into());
        }

        let mut mask: Vec<libc::c_ulong> = vec![0; words];

        let size = words * size_of::<libc::c_ulong>();

        let rtn =
            unsafe { libc::sched_getaffinity(0, size, mask.as_mut_ptr() as *mut libc::cpu_set_t) };

        if rtn == 0 {
            // The kernel writes the mask as an array of `unsigned long`, so adding up the bits of every word gives what `CPU_COUNT` would.
            return Ok(mask.iter().map(|word| word.count_ones() as usize).sum());
        }

        let err = io::Error::last_os_error();

        if err.raw_os_error() != Some(libc::EINVAL) {
            return Err(err.into());
        }
    }
}
