use std::fs;

use crate::{Error, utils::proc_pid_path};

/// Get the number of open file descriptors of a specific process found by ID by counting the entries of the `/proc/PID/fd` folder. Reading the folder of a process owned by another user needs the `CAP_SYS_PTRACE` capability, otherwise a `PermissionDenied` error is returned. For the current process, the descriptor used for counting is included.
///
/// ```rust
/// use mprober_lib::process;
///
/// let fd_count = process::get_process_fd_count(std::process::id()).unwrap();
///
/// println!("{fd_count}");
/// ```
pub fn get_process_fd_count(pid: u32) -> Result<usize, Error> {
    let fd_path = proc_pid_path(pid).join("fd");

    let mut count = 0;

    for entry in fs::read_dir(fd_path)? {
        entry?;

        count += 1;
    }

    Ok(count)
}
