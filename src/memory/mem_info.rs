use std::io::Read;

use scanner_rust::ScannerAscii;

use crate::Error;

/// The detailed memory information of the `/proc/meminfo` file. Every field is in bytes unless its documentation says otherwise. The fields that depend on a kernel option or on a kernel version are `Option`, and every other field is reported by every kernel this crate supports.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MemInfo {
    /// `MemTotal`, the usable memory, which is the physical memory minus what the firmware and the kernel binary reserve.
    pub total:              u64,
    /// `MemFree`, the memory that is not used at all.
    pub free:               u64,
    /// `MemAvailable`, the memory that is available for starting new applications without swapping, which the kernel estimates from the free memory and the reclaimable caches.
    pub available:          u64,
    /// `Buffers`, the temporary storage for raw disk blocks.
    pub buffers:            u64,
    /// `Cached`, the page cache, which holds file contents. `Shmem` is part of it.
    pub cached:             u64,
    /// `SwapCached`, the memory that was swapped out but is still in RAM as well, so it can be swapped out again without being written.
    pub swap_cached:        u64,
    /// `SwapTotal`, the size of the swap space.
    pub swap_total:         u64,
    /// `SwapFree`, the swap space that is not in use.
    pub swap_free:          u64,
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
    /// `HardwareCorrupted`, the memory the kernel found to be physically damaged. It is `None` on a kernel without `CONFIG_MEMORY_FAILURE`.
    pub hardware_corrupted: Option<u64>,
    /// `AnonHugePages`, the anonymous memory that is backed by transparent huge pages. It is `None` on a kernel without `CONFIG_TRANSPARENT_HUGEPAGE`.
    pub anon_huge_pages:    Option<u64>,
    /// `Shmem`, the memory of `tmpfs` and of shared anonymous mappings.
    pub shmem:              u64,
    /// `Zswap`, the compressed memory pool in RAM. It is `None` on kernels older than 5.19 and on a kernel without `CONFIG_ZSWAP`.
    pub zswap:              Option<u64>,
    /// `Zswapped`, the original size of the memory that is stored in `zswap`. It is `None` on kernels older than 5.19 and on a kernel without `CONFIG_ZSWAP`.
    pub zswapped:           Option<u64>,
    /// `HugePages_Total`, the number of preallocated huge pages. This is a count, not a size. It is `None` on a kernel without `CONFIG_HUGETLB_PAGE`.
    pub huge_pages_total:   Option<u64>,
    /// `HugePages_Free`, the number of preallocated huge pages that are not in use. This is a count, not a size. It is `None` on a kernel without `CONFIG_HUGETLB_PAGE`.
    pub huge_pages_free:    Option<u64>,
    /// `HugePages_Rsvd`, the number of huge pages that are promised but not yet allocated. This is a count, not a size. It is `None` on a kernel without `CONFIG_HUGETLB_PAGE`.
    pub huge_pages_rsvd:    Option<u64>,
    /// `Hugepagesize`, the size of one preallocated huge page. It is `None` on a kernel without `CONFIG_HUGETLB_PAGE`.
    pub huge_page_size:     Option<u64>,
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
            b"MemTotal:" => info.total = value * 1024,
            b"MemFree:" => info.free = value * 1024,
            b"MemAvailable:" => info.available = value * 1024,
            b"Buffers:" => info.buffers = value * 1024,
            b"Cached:" => info.cached = value * 1024,
            b"SwapCached:" => info.swap_cached = value * 1024,
            b"SwapTotal:" => info.swap_total = value * 1024,
            b"SwapFree:" => info.swap_free = value * 1024,
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
            b"HardwareCorrupted:" => info.hardware_corrupted = Some(value * 1024),
            b"AnonHugePages:" => info.anon_huge_pages = Some(value * 1024),
            b"Shmem:" => info.shmem = value * 1024,
            b"Zswap:" => info.zswap = Some(value * 1024),
            b"Zswapped:" => info.zswapped = Some(value * 1024),
            b"HugePages_Total:" => info.huge_pages_total = Some(value),
            b"HugePages_Free:" => info.huge_pages_free = Some(value),
            b"HugePages_Rsvd:" => info.huge_pages_rsvd = Some(value),
            b"Hugepagesize:" => info.huge_page_size = Some(value * 1024),
            _ => (),
        }

        sc.drop_next_line()?;
    }

    Ok(info)
}

/// Get the detailed memory information by reading the `/proc/meminfo` file. It is a superset of [`crate::memory::free`], which only reports what the `free` command shows, and covers the write-back, slab, page table, commit and huge page fields as well.
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
MemAvailable:   54095372 kB
Buffers:            6528 kB
Cached:         11571844 kB
SwapCached:            0 kB
SwapTotal:       8000508 kB
SwapFree:        7000000 kB
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

        assert_eq!(65083032 * 1024, info.total);
        assert_eq!(43608388 * 1024, info.free);
        assert_eq!(54095372 * 1024, info.available);
        assert_eq!(6528 * 1024, info.buffers);
        assert_eq!(11571844 * 1024, info.cached);
        assert_eq!(0, info.swap_cached);
        assert_eq!(8000508 * 1024, info.swap_total);
        assert_eq!(7000000 * 1024, info.swap_free);
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
        assert_eq!(Some(124928 * 1024), info.anon_huge_pages);
        assert_eq!(Some(0), info.hardware_corrupted);
        assert_eq!(Some(4 * 1024), info.zswap);
        assert_eq!(Some(8 * 1024), info.zswapped);

        // The huge page counts are numbers of pages, while the page size is in kB.
        assert_eq!(Some(2), info.huge_pages_total);
        assert_eq!(Some(1), info.huge_pages_free);
        assert_eq!(Some(0), info.huge_pages_rsvd);
        assert_eq!(Some(2048 * 1024), info.huge_page_size);
    }

    #[test]
    fn parse_without_optional_fields() {
        // A kernel older than 5.19 has no zswap lines, and one built without huge pages has no huge page lines.
        const MEMINFO: &[u8] = b"MemTotal:        4000 kB
Active:          1000 kB
Shmem:            500 kB
";

        let info = parse_mem_info(MEMINFO).unwrap();

        assert_eq!(1000 * 1024, info.active);
        assert_eq!(500 * 1024, info.shmem);
        assert_eq!(None, info.zswap);
        assert_eq!(None, info.zswapped);
        assert_eq!(None, info.hardware_corrupted);
        assert_eq!(None, info.anon_huge_pages);
        assert_eq!(None, info.huge_pages_total);
        assert_eq!(None, info.huge_page_size);
    }
}
