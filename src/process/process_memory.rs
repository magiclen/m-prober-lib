use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{parse_number, proc_pid_path, read_single_record_file},
};

/// The memory of a process summed over all of its mappings, read from the `/proc/PID/smaps_rollup` file. Every field is in bytes.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessMemory {
    /// The resident set size (`Rss`), which counts every page the process has in RAM, no matter how many processes share it.
    pub rss:             u64,
    /// The proportional set size (`Pss`), which divides every shared page by the number of processes sharing it. Adding this up over every process stays within the physical memory, which is what makes it the fair per-process number.
    pub pss:             u64,
    /// The part of `pss` that is anonymous memory (`Pss_Anon`).
    pub pss_anon:        u64,
    /// The part of `pss` that is file-backed (`Pss_File`).
    pub pss_file:        u64,
    /// The part of `pss` that is shared memory (`Pss_Shmem`).
    pub pss_shmem:       u64,
    /// The clean pages that are shared with another process (`Shared_Clean`).
    pub shared_clean:    u64,
    /// The dirty pages that are shared with another process (`Shared_Dirty`).
    pub shared_dirty:    u64,
    /// The clean pages that no other process maps (`Private_Clean`).
    pub private_clean:   u64,
    /// The dirty pages that no other process maps (`Private_Dirty`). Together with `private_clean` this is the unique set size (USS), which is the memory that would be freed by killing the process.
    pub private_dirty:   u64,
    /// The pages that have been referenced recently (`Referenced`).
    pub referenced:      u64,
    /// The anonymous memory of the process (`Anonymous`).
    pub anonymous:       u64,
    /// The memory backed by transparent huge pages (`AnonHugePages`).
    pub anon_huge_pages: u64,
    /// The memory that has been swapped out (`Swap`).
    pub swap:            u64,
    /// The proportional share of the swapped-out memory (`SwapPss`).
    pub swap_pss:        u64,
    /// The memory that has been locked into RAM by `mlock(2)` (`Locked`).
    pub locked:          u64,
}

impl ProcessMemory {
    /// Get the unique set size (USS), which is `private_clean + private_dirty`. This is the memory that killing the process would actually free.
    #[inline]
    pub fn unique_set_size(&self) -> u64 {
        self.private_clean + self.private_dirty
    }
}

fn parse_process_memory(data: &[u8]) -> Result<ProcessMemory, Error> {
    let mut memory = ProcessMemory::default();

    let mut sc = ScannerU8SliceAscii::new(data);

    // The first line is the address range of the rollup, which carries no counter.
    sc.drop_next_line()?;

    while let Some(line) = sc.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        let (Some(label), Some(value)) = (sc.next()?, sc.next()?) else {
            continue;
        };

        // A line whose value is not a number is skipped, so a field a later kernel adds cannot fail the parsing.
        let Ok(value) = parse_number::<u64>(value) else {
            continue;
        };

        // Every field of this file is in kB.
        let value = value * 1024;

        match label {
            b"Rss:" => memory.rss = value,
            b"Pss:" => memory.pss = value,
            b"Pss_Anon:" => memory.pss_anon = value,
            b"Pss_File:" => memory.pss_file = value,
            b"Pss_Shmem:" => memory.pss_shmem = value,
            b"Shared_Clean:" => memory.shared_clean = value,
            b"Shared_Dirty:" => memory.shared_dirty = value,
            b"Private_Clean:" => memory.private_clean = value,
            b"Private_Dirty:" => memory.private_dirty = value,
            b"Referenced:" => memory.referenced = value,
            b"Anonymous:" => memory.anonymous = value,
            b"AnonHugePages:" => memory.anon_huge_pages = value,
            b"Swap:" => memory.swap = value,
            b"SwapPss:" => memory.swap_pss = value,
            b"Locked:" => memory.locked = value,
            _ => (),
        }
    }

    Ok(memory)
}

/// Get the memory of a specific process found by ID by reading the `/proc/PID/smaps_rollup` file. Unlike the `rss` of [`crate::process::ProcessStat`], which counts a shared page in full for every process that maps it, the `pss` this reports splits a shared page between them, so adding it up over every process stays within the physical memory.
///
/// The kernel sums the file over every mapping of the process, which costs more than reading `/proc/PID/statm`, but far less than reading the whole `/proc/PID/smaps` file. Reading the file of a process owned by another user needs the `CAP_SYS_PTRACE` capability, otherwise a `PermissionDenied` error is returned. A kernel thread or a zombie process has no address space, so the kernel answers with `ESRCH` instead, which is not a `NotFound` error; check `raw_os_error()` for `libc::ESRCH` to tell it apart.
///
/// ```rust,no_run
/// use mprober_lib::process;
///
/// let process_memory =
///     process::get_process_memory(std::process::id()).unwrap();
///
/// println!("{process_memory:#?}");
/// println!("USS: {}", process_memory.unique_set_size());
/// ```
#[inline]
pub fn get_process_memory(pid: u32) -> Result<ProcessMemory, Error> {
    let path = proc_pid_path(pid).join("smaps_rollup");

    parse_process_memory(&read_single_record_file(path, 1024)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMAPS_ROLLUP: &[u8] =
        b"5f520d98a000-7ffe880af000 ---p 00000000 00:00 0                          [rollup]
Rss:                1960 kB
Pss:                 130 kB
Pss_Dirty:           108 kB
Pss_Anon:            108 kB
Pss_File:             22 kB
Pss_Shmem:             0 kB
Shared_Clean:       1852 kB
Shared_Dirty:          0 kB
Private_Clean:         4 kB
Private_Dirty:       108 kB
Referenced:         1960 kB
Anonymous:           108 kB
AnonHugePages:         0 kB
Swap:                 16 kB
SwapPss:               8 kB
Locked:                0 kB
";

    #[test]
    fn parse() {
        let memory = parse_process_memory(SMAPS_ROLLUP).unwrap();

        assert_eq!(1960 * 1024, memory.rss);
        assert_eq!(130 * 1024, memory.pss);
        assert_eq!(108 * 1024, memory.pss_anon);
        assert_eq!(22 * 1024, memory.pss_file);
        assert_eq!(1852 * 1024, memory.shared_clean);
        assert_eq!(4 * 1024, memory.private_clean);
        assert_eq!(108 * 1024, memory.private_dirty);
        assert_eq!(108 * 1024, memory.anonymous);
        assert_eq!(16 * 1024, memory.swap);
        assert_eq!(8 * 1024, memory.swap_pss);

        assert_eq!((4 + 108) * 1024, memory.unique_set_size());
    }
}
