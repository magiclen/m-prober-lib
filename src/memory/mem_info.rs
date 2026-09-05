use std::io::Read;

use crate::{Error, scanner_rust::ScannerAscii};

/// The detailed memory information of the `/proc/meminfo` file. Every field is in bytes unless its documentation says otherwise, and a field the kernel does not report is `0`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemInfo {
    /// `Active`, the memory that has been used recently.
    pub active:             u64,
    /// `Inactive`, the memory that has not been used recently and is a candidate for reclaiming.
    pub inactive:           u64,
    /// `Unevictable`, the memory that cannot be reclaimed.
    pub unevictable:        u64,
    /// `Mlocked`, the memory that is locked into RAM by `mlock(2)`.
    pub mlocked:            u64,
    /// `Dirty`, the memory that is waiting to be written back to disk.
    pub dirty:              u64,
    /// `Writeback`, the memory that is being written back to disk right now.
    pub writeback:          u64,
    /// `AnonPages`, the memory that is not backed by a file.
    pub anon_pages:         u64,
    /// `Mapped`, the memory that is mapped into page tables, e.g. by `mmap(2)`.
    pub mapped:             u64,
    /// `Slab`, the memory the kernel uses for its own data structures.
    pub slab:               u64,
    /// `SReclaimable`, the part of the slab that can be reclaimed, e.g. the dentry and inode caches.
    pub slab_reclaimable:   u64,
    /// `SUnreclaim`, the part of the slab that cannot be reclaimed.
    pub slab_unreclaimable: u64,
    /// `KernelStack`, the memory of the kernel stacks of every task.
    pub kernel_stack:       u64,
    /// `PageTables`, the memory the page tables use.
    pub page_tables:        u64,
    /// `Percpu`, the memory the per-CPU allocator uses.
    pub percpu:             u64,
    /// `CommitLimit`, the memory that may be committed in total under the current overcommit policy.
    pub commit_limit:       u64,
    /// `Committed_AS`, the memory that has been committed, which can exceed the physical memory because most of it is never touched.
    pub committed_as:       u64,
    /// `VmallocTotal`, the size of the vmalloc address space.
    pub vmalloc_total:      u64,
    /// `VmallocUsed`, the memory that is allocated in the vmalloc address space.
    pub vmalloc_used:       u64,
    /// `HardwareCorrupted`, the memory the kernel found to be physically damaged.
    pub hardware_corrupted: u64,
    /// `AnonHugePages`, the anonymous memory that is backed by transparent huge pages.
    pub anon_huge_pages:    u64,
    /// `Shmem`, the memory of `tmpfs` and of shared anonymous mappings.
    pub shmem:              u64,
    /// `Zswap`, the compressed memory pool in RAM.
    pub zswap:              u64,
    /// `Zswapped`, the original size of the memory that is stored in `zswap`.
    pub zswapped:           u64,
    /// `HugePages_Total`, the number of preallocated huge pages. This is a count, not a size.
    pub huge_pages_total:   u64,
    /// `HugePages_Free`, the number of preallocated huge pages that are not in use. This is a count, not a size.
    pub huge_pages_free:    u64,
    /// `HugePages_Rsvd`, the number of huge pages that are promised but not yet allocated. This is a count, not a size.
    pub huge_pages_rsvd:    u64,
    /// `Hugepagesize`, the size of one preallocated huge page.
    pub huge_page_size:     u64,
}

fn parse_mem_info<R: Read>(reader: R) -> Result<MemInfo, Error> {
    let mut sc: ScannerAscii<R, 1024> = ScannerAscii::new2(reader);

    let mut info = MemInfo::default();

    while let Some(label) = sc.next_raw()? {
        // A line without a number cannot exist in this file, but the file may be trimmed in a container, so a missing value only ends the parsing.
        let Some(value) = sc.next_u64()? else {
            break;
        };

        // Most values are in kB, but the huge page fields are counts, so they are not scaled.
        match label.as_slice() {
            b"Active:" => info.active = value * 1024,
            b"Inactive:" => info.inactive = value * 1024,
            b"Unevictable:" => info.unevictable = value * 1024,
            b"Mlocked:" => info.mlocked = value * 1024,
            b"Dirty:" => info.dirty = value * 1024,
            b"Writeback:" => info.writeback = value * 1024,
            b"AnonPages:" => info.anon_pages = value * 1024,
            b"Mapped:" => info.mapped = value * 1024,
            b"Slab:" => info.slab = value * 1024,
            b"SReclaimable:" => info.slab_reclaimable = value * 1024,
            b"SUnreclaim:" => info.slab_unreclaimable = value * 1024,
            b"KernelStack:" => info.kernel_stack = value * 1024,
            b"PageTables:" => info.page_tables = value * 1024,
            b"Percpu:" => info.percpu = value * 1024,
            b"CommitLimit:" => info.commit_limit = value * 1024,
            b"Committed_AS:" => info.committed_as = value * 1024,
            b"VmallocTotal:" => info.vmalloc_total = value * 1024,
            b"VmallocUsed:" => info.vmalloc_used = value * 1024,
            b"HardwareCorrupted:" => info.hardware_corrupted = value * 1024,
            b"AnonHugePages:" => info.anon_huge_pages = value * 1024,
            b"Shmem:" => info.shmem = value * 1024,
            b"Zswap:" => info.zswap = value * 1024,
            b"Zswapped:" => info.zswapped = value * 1024,
            b"HugePages_Total:" => info.huge_pages_total = value,
            b"HugePages_Free:" => info.huge_pages_free = value,
            b"HugePages_Rsvd:" => info.huge_pages_rsvd = value,
            b"Hugepagesize:" => info.huge_page_size = value * 1024,
            _ => (),
        }

        sc.drop_next_line()?;
    }

    Ok(info)
}

/// Get the detailed memory information by reading the `/proc/meminfo` file. Unlike [`crate::memory::free`], which only reports what the `free` command shows, this covers the write-back, slab, page table, commit and huge page fields as well.
///
/// ```rust
/// use mprober_lib::memory;
///
/// let mem_info = memory::get_mem_info().unwrap();
///
/// println!("{mem_info:#?}");
/// ```
#[inline]
pub fn get_mem_info() -> Result<MemInfo, Error> {
    parse_mem_info(std::fs::File::open("/proc/meminfo")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &[u8] = b"MemTotal:       65083032 kB
MemFree:        43608388 kB
Active:         14436784 kB
Inactive:        5964948 kB
Unevictable:       95872 kB
Mlocked:             248 kB
Zswap:                 4 kB
Zswapped:              8 kB
Dirty:              1728 kB
Writeback:             0 kB
AnonPages:       8950176 kB
Mapped:          2314828 kB
Shmem:            631916 kB
Slab:             639484 kB
SReclaimable:     268052 kB
SUnreclaim:       371432 kB
KernelStack:       36064 kB
PageTables:       150044 kB
Percpu:            30720 kB
CommitLimit:    40542024 kB
Committed_AS:   32077664 kB
VmallocTotal:   34359738367 kB
VmallocUsed:      205940 kB
HardwareCorrupted:     0 kB
AnonHugePages:    124928 kB
HugePages_Total:       2
HugePages_Free:        1
HugePages_Rsvd:        0
Hugepagesize:       2048 kB
";

    #[test]
    fn parse() {
        let info = parse_mem_info(MEMINFO).unwrap();

        assert_eq!(14436784 * 1024, info.active);
        assert_eq!(5964948 * 1024, info.inactive);
        assert_eq!(248 * 1024, info.mlocked);
        assert_eq!(1728 * 1024, info.dirty);
        assert_eq!(0, info.writeback);
        assert_eq!(8950176 * 1024, info.anon_pages);
        assert_eq!(631916 * 1024, info.shmem);
        assert_eq!(639484 * 1024, info.slab);
        assert_eq!(268052 * 1024, info.slab_reclaimable);
        assert_eq!(371432 * 1024, info.slab_unreclaimable);
        assert_eq!(150044 * 1024, info.page_tables);
        assert_eq!(32077664 * 1024, info.committed_as);
        assert_eq!(124928 * 1024, info.anon_huge_pages);
        assert_eq!(4 * 1024, info.zswap);
        assert_eq!(8 * 1024, info.zswapped);

        // The huge page counts are numbers of pages, while the page size is in kB.
        assert_eq!(2, info.huge_pages_total);
        assert_eq!(1, info.huge_pages_free);
        assert_eq!(2048 * 1024, info.huge_page_size);
    }
}
