use std::{io::ErrorKind, path::Path};

use crate::scanner_rust::{generic_array::typenum::U192, ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct ProcessStatus {
    /// The user who created this process or the UID set via `setuid()` by the root caller.
    pub real_uid:      u32,
    /// The group who created this process or the GID set via `setgid()` by the root caller.
    pub real_gid:      u32,
    /// The UID set via `setuid()` by the caller.
    pub effective_uid: u32,
    /// The GID set via `setgid()` by the caller.
    pub effective_gid: u32,
    /// The UID set via `setuid()` by the root caller
    pub saved_set_uid: u32,
    /// The GID set via `setgid()` by the root caller
    pub saved_set_gid: u32,
    /// The UID of the running executable file of this process.
    pub fs_uid:        u32,
    /// The GID of the running executable file of this process.
    pub fs_gid:        u32,
}

/// Get the status of a specific process found by ID by reading the `/proc/PID/status` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::process;
///
/// let process_status = process::get_process_status(1).unwrap();
///
/// println!("{:#?}", process_status);
/// ```
pub fn get_process_status(pid: u32) -> Result<ProcessStatus, ScannerError> {
    let mut status = ProcessStatus::default();

    let status_path = Path::new("/proc").join(pid.to_string()).join("status");

    let mut sc: ScannerAscii<_, U192> = ScannerAscii::scan_path2(status_path)?;

    loop {
        let label = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

        if label.starts_with(b"Uid") {
            status.real_uid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
            status.effective_uid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
            status.saved_set_uid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
            status.fs_uid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

            break;
        } else {
            sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
        }
    }

    loop {
        let label = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

        if label.starts_with(b"Gid") {
            status.real_gid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
            status.effective_gid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
            status.saved_set_gid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;
            status.fs_gid = sc.next_u32()?.ok_or(ErrorKind::UnexpectedEof)?;

            break;
        } else {
            sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
        }
    }

    Ok(status)
}
