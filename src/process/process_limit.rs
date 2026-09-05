use std::{io, ptr};

use crate::Error;

/// One resource limit of a process. A limit that is `None` means the resource is unlimited.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessLimit {
    /// The soft limit, which the process itself may raise up to the hard limit.
    pub soft: Option<u64>,
    /// The hard limit, which only a privileged process may raise.
    pub hard: Option<u64>,
}

/// The resource limits of a process, read with the `prlimit64` system call. See `getrlimit(2)` for what each one covers.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessLimits {
    /// `RLIMIT_NOFILE`, the number of file descriptors the process may open. Comparing it with [`crate::process::get_process_fd_count`] tells how close the process is to running out.
    pub open_files:     ProcessLimit,
    /// `RLIMIT_NPROC`, the number of processes the real user of this process may have.
    pub processes:      ProcessLimit,
    /// `RLIMIT_AS`, the size of the virtual address space in bytes.
    pub address_space:  ProcessLimit,
    /// `RLIMIT_DATA`, the size of the data segment in bytes.
    pub data_size:      ProcessLimit,
    /// `RLIMIT_STACK`, the size of the stack in bytes.
    pub stack_size:     ProcessLimit,
    /// `RLIMIT_CORE`, the size of a core dump in bytes. A limit of `0` disables core dumps.
    pub core_file_size: ProcessLimit,
    /// `RLIMIT_FSIZE`, the size of a file the process may create in bytes.
    pub file_size:      ProcessLimit,
    /// `RLIMIT_MEMLOCK`, the memory the process may lock into RAM in bytes.
    pub locked_memory:  ProcessLimit,
    /// `RLIMIT_CPU`, the CPU time the process may use in seconds.
    pub cpu_time:       ProcessLimit,
}

/// The `struct rlimit64` of the kernel. It is declared here instead of taken from libc, because only glibc exposes the `prlimit64` wrapper, while the system call itself is always there.
#[repr(C)]
#[derive(Default)]
struct RawRlimit {
    rlim_cur: u64,
    rlim_max: u64,
}

/// Read one resource limit of a process. `RLIM64_INFINITY` is `u64::MAX`, which becomes `None`.
fn prlimit(pid: u32, resource: libc::c_int) -> Result<ProcessLimit, Error> {
    let mut limit = RawRlimit::default();

    let rtn = unsafe {
        libc::syscall(
            libc::SYS_prlimit64,
            pid as libc::pid_t,
            resource,
            ptr::null::<RawRlimit>(),
            &mut limit as *mut RawRlimit,
        )
    };

    if rtn != 0 {
        return Err(io::Error::last_os_error().into());
    }

    let to_option = |value: u64| if value == u64::MAX { None } else { Some(value) };

    Ok(ProcessLimit {
        soft: to_option(limit.rlim_cur), hard: to_option(limit.rlim_max)
    })
}

/// Get the resource limits of a specific process found by ID using the `prlimit64` system call, which is much cheaper than parsing the `/proc/PID/limits` file. Reading the limits of a process owned by another user needs the `CAP_SYS_RESOURCE` capability, otherwise a `PermissionDenied` error is returned.
///
/// ```rust
/// use mprober_lib::process;
///
/// let limits = process::get_process_limits(std::process::id()).unwrap();
///
/// println!("{limits:#?}");
/// ```
pub fn get_process_limits(pid: u32) -> Result<ProcessLimits, Error> {
    Ok(ProcessLimits {
        open_files:     prlimit(pid, libc::RLIMIT_NOFILE as libc::c_int)?,
        processes:      prlimit(pid, libc::RLIMIT_NPROC as libc::c_int)?,
        address_space:  prlimit(pid, libc::RLIMIT_AS as libc::c_int)?,
        data_size:      prlimit(pid, libc::RLIMIT_DATA as libc::c_int)?,
        stack_size:     prlimit(pid, libc::RLIMIT_STACK as libc::c_int)?,
        core_file_size: prlimit(pid, libc::RLIMIT_CORE as libc::c_int)?,
        file_size:      prlimit(pid, libc::RLIMIT_FSIZE as libc::c_int)?,
        locked_memory:  prlimit(pid, libc::RLIMIT_MEMLOCK as libc::c_int)?,
        cpu_time:       prlimit(pid, libc::RLIMIT_CPU as libc::c_int)?,
    })
}
