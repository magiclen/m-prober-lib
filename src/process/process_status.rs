use std::path::Path;

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, proc_pid_path, read_single_record_file_into},
};

/// Fields read from the `/proc/PID/status` file. Memory fields are in bytes.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessStatus {
    /// The real UID, which is the UID of the user who started the process.
    pub real_uid:                   u32,
    /// The real GID.
    pub real_gid:                   u32,
    /// The effective UID, which is used for most permission checks.
    pub effective_uid:              u32,
    /// The effective GID.
    pub effective_gid:              u32,
    /// The saved set-user-ID, which a set-user-ID program uses to switch its effective UID back and forth.
    pub saved_set_uid:              u32,
    /// The saved set-group-ID.
    pub saved_set_gid:              u32,
    /// The filesystem UID, which is used for permission checks on file system access. It is normally the same as the effective UID.
    pub fs_uid:                     u32,
    /// The filesystem GID.
    pub fs_gid:                     u32,
    /// The peak resident set size (`VmHWM`) in bytes.
    pub vm_hwm:                     u64,
    /// The swapped-out memory size (`VmSwap`) in bytes. Swapped-out shmem is not included.
    pub vm_swap:                    u64,
    /// The number of voluntary context switches.
    pub voluntary_ctxt_switches:    u64,
    /// The number of involuntary context switches.
    pub nonvoluntary_ctxt_switches: u64,
}

/// Parse the content of a `/proc/PID/status` file. When `stop_at_threads` is set, the parsing ends at the `Threads:` line, which the kernel writes after every field except the context switch counters, so a caller that does not need those reads only about half of the file.
fn parse_process_status(data: &[u8], stop_at_threads: bool) -> Result<ProcessStatus, Error> {
    let mut status = ProcessStatus::default();

    let mut sc = ScannerU8SliceAscii::new(data);

    while let Some(label) = sc.next()? {
        if stop_at_threads && label == b"Threads:" {
            break;
        }

        match label {
            b"Uid:" => {
                status.real_uid = sc.next_u32()?.or_eof()?;
                status.effective_uid = sc.next_u32()?.or_eof()?;
                status.saved_set_uid = sc.next_u32()?.or_eof()?;
                status.fs_uid = sc.next_u32()?.or_eof()?;
            },
            b"Gid:" => {
                status.real_gid = sc.next_u32()?.or_eof()?;
                status.effective_gid = sc.next_u32()?.or_eof()?;
                status.saved_set_gid = sc.next_u32()?.or_eof()?;
                status.fs_gid = sc.next_u32()?.or_eof()?;
            },
            b"VmHWM:" => {
                status.vm_hwm = sc.next_u64()?.or_eof()? * 1024;
            },
            b"VmSwap:" => {
                status.vm_swap = sc.next_u64()?.or_eof()? * 1024;
            },
            b"voluntary_ctxt_switches:" => {
                status.voluntary_ctxt_switches = sc.next_u64()?.or_eof()?;
            },
            b"nonvoluntary_ctxt_switches:" => {
                status.nonvoluntary_ctxt_switches = sc.next_u64()?.or_eof()?;
            },
            _ => (),
        }

        sc.drop_next_line()?;
    }

    Ok(status)
}

/// Get the status of a specific process found by ID by reading the `/proc/PID/status` file.
///
/// ```rust
/// use mprober_lib::process;
///
/// let process_status = process::get_process_status(1).unwrap();
///
/// println!("{process_status:#?}");
/// ```
pub fn get_process_status(pid: u32) -> Result<ProcessStatus, Error> {
    let mut buffer = Vec::new();

    read_process_status(&proc_pid_path(pid), false, &mut buffer)
}

/// Read a `/proc/PID/status` file. The context switch counters are the last two lines of the file, so a caller that does not need them can skip the second half of the parsing. The folder of the process and the buffer are passed in, so that a caller which reads several files of one process builds the path and allocates the buffer only once.
pub(crate) fn read_process_status(
    process_path: &Path,
    stop_at_threads: bool,
    buffer: &mut Vec<u8>,
) -> Result<ProcessStatus, Error> {
    // The kernel generates the whole file at once and it is about 1.5 KB, so one read normally gets everything.
    read_single_record_file_into(process_path.join("status"), 2048, buffer)?;

    parse_process_status(buffer, stop_at_threads)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &[u8] = b"Name:\tWeb Content
Umask:\t0002
State:\tS (sleeping)
Tgid:\t1234
Pid:\t1234
PPid:\t1
Uid:\t1000\t1001\t1002\t1003
Gid:\t2000\t2001\t2002\t2003
FDSize:\t64
Groups:\t4 24 27
VmPeak:\t    8752 kB
VmSize:\t    8752 kB
VmHWM:\t    1956 kB
VmRSS:\t    1956 kB
VmSwap:\t     128 kB
Threads:\t1
voluntary_ctxt_switches:\t12
nonvoluntary_ctxt_switches:\t3
";

    #[test]
    fn parse_status() {
        let status = parse_process_status(STATUS, false).unwrap();

        assert_eq!(1000, status.real_uid);
        assert_eq!(1001, status.effective_uid);
        assert_eq!(1002, status.saved_set_uid);
        assert_eq!(1003, status.fs_uid);
        assert_eq!(2000, status.real_gid);
        assert_eq!(2001, status.effective_gid);
        assert_eq!(2002, status.saved_set_gid);
        assert_eq!(2003, status.fs_gid);
        assert_eq!(1956 * 1024, status.vm_hwm);
        assert_eq!(128 * 1024, status.vm_swap);
        assert_eq!(12, status.voluntary_ctxt_switches);
        assert_eq!(3, status.nonvoluntary_ctxt_switches);
    }

    #[test]
    fn parse_status_stopping_at_threads() {
        let status = parse_process_status(STATUS, true).unwrap();

        // Everything a process scan needs comes before the `Threads:` line.
        assert_eq!(1000, status.real_uid);
        assert_eq!(2001, status.effective_gid);
        assert_eq!(128 * 1024, status.vm_swap);

        // The context switch counters are the last two lines, so they are not read.
        assert_eq!(0, status.voluntary_ctxt_switches);
        assert_eq!(0, status.nonvoluntary_ctxt_switches);
    }
}
