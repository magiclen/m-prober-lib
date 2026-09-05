mod vm_stat;

use std::{
    fs::File,
    io::{self, ErrorKind, Read},
};

pub use vm_stat::*;

use crate::{Error, scanner_rust::ScannerAscii};

/// Memory information in bytes. The values in `/proc/meminfo` are in kB, so they are multiplied by 1024 here.
#[derive(Default, Debug, Clone)]
pub struct Mem {
    /// `MemTotal` in bytes.
    pub total:     u64,
    /// `MemTotal - MemAvailable` in bytes, the same as the `used` column of the `free` command in procps-ng 4.x.
    pub used:      u64,
    /// `MemFree` in bytes.
    pub free:      u64,
    /// `Shmem` in bytes.
    pub shared:    u64,
    /// `Buffers` in bytes.
    pub buffers:   u64,
    /// `Cached + KReclaimable` in bytes, the page cache plus the reclaimable kernel memory.
    pub cache:     u64,
    /// `MemAvailable` in bytes.
    pub available: u64,
}

/// Swap information in bytes.
#[derive(Default, Debug, Clone)]
pub struct Swap {
    /// `SwapTotal` in bytes.
    pub total: u64,
    /// `SwapTotal - SwapFree - SwapCached` in bytes.
    pub used:  u64,
    /// `SwapFree` in bytes.
    pub free:  u64,
    /// `SwapCached` in bytes.
    pub cache: u64,
}

/// The memory and swap information that the `free` command shows.
#[derive(Default, Debug, Clone)]
pub struct Free {
    /// The physical memory usage.
    pub mem:  Mem,
    /// The swap usage.
    pub swap: Swap,
}

const MEM_TOTAL: usize = 0;
const MEM_FREE: usize = 1;
const MEM_AVAILABLE: usize = 2;
const BUFFERS: usize = 3;
const CACHED: usize = 4;
const SWAP_CACHED: usize = 5;
const SWAP_TOTAL: usize = 6;
const SWAP_FREE: usize = 7;
const SHMEM: usize = 8;
const K_RECLAIMABLE: usize = 9;

// The labels include the colon so that, for example, `Shmem:` cannot be matched by `ShmemHugePages:`.
const USEFUL_ITEMS: [&[u8]; 10] = [
    b"MemTotal:",
    b"MemFree:",
    b"MemAvailable:",
    b"Buffers:",
    b"Cached:",
    b"SwapCached:",
    b"SwapTotal:",
    b"SwapFree:",
    b"Shmem:",
    b"KReclaimable:",
];

fn parse_meminfo<R: Read>(reader: R) -> Result<Free, Error> {
    let mut sc: ScannerAscii<R, 768> = ScannerAscii::new2(reader);

    // The items are looked up by label instead of by position, so neither the order nor the presence of a line matters.
    let mut item_values: [Option<u64>; USEFUL_ITEMS.len()] = [None; USEFUL_ITEMS.len()];

    let mut remaining = USEFUL_ITEMS.len();

    while let Some(label) = sc.next_raw()? {
        if let Some(i) = USEFUL_ITEMS.iter().position(|&item| label == item)
            && item_values[i].is_none()
        {
            let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            item_values[i] = Some(value * 1024);

            remaining -= 1;

            if remaining == 0 {
                break;
            }
        }

        sc.drop_next_line()?;
    }

    // Only the total is mandatory, because a container may provide a trimmed file.
    let total = item_values[MEM_TOTAL].ok_or(io::Error::from(ErrorKind::InvalidData))?;
    let free = item_values[MEM_FREE].unwrap_or(0);
    let buffers = item_values[BUFFERS].unwrap_or(0);
    let cached = item_values[CACHED].unwrap_or(0);
    let swap_cached = item_values[SWAP_CACHED].unwrap_or(0);
    let swap_total = item_values[SWAP_TOTAL].unwrap_or(0);
    let swap_free = item_values[SWAP_FREE].unwrap_or(0);
    let shmem = item_values[SHMEM].unwrap_or(0);
    let k_reclaimable = item_values[K_RECLAIMABLE].unwrap_or(0);

    // `MemAvailable` exists since Linux 3.14, so this estimate is only a fallback for a trimmed file.
    let available =
        item_values[MEM_AVAILABLE].unwrap_or_else(|| free + buffers + cached + k_reclaimable);

    let mem = Mem {
        total,
        used: total.saturating_sub(available),
        free,
        shared: shmem,
        buffers,
        cache: cached + k_reclaimable,
        available,
    };

    let swap = Swap {
        total: swap_total,
        used:  swap_total.saturating_sub(swap_free).saturating_sub(swap_cached),
        free:  swap_free,
        cache: swap_cached,
    };

    Ok(Free {
        mem,
        swap,
    })
}

/// Get memory information like the `free` command by reading the `/proc/meminfo` file.
///
/// ```rust
/// use mprober_lib::memory;
///
/// let free = memory::free().unwrap();
///
/// println!("{free:#?}");
/// ```
#[inline]
pub fn free() -> Result<Free, Error> {
    parse_meminfo(File::open("/proc/meminfo")?)
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
Active:         14436784 kB
Inactive:        5964948 kB
Unevictable:       95872 kB
Mlocked:             248 kB
SwapTotal:       8000508 kB
SwapFree:        8000508 kB
Zswap:                 0 kB
Dirty:              1728 kB
AnonPages:       8950176 kB
Mapped:          2314828 kB
Shmem:            631916 kB
KReclaimable:     268052 kB
Slab:             639484 kB
SReclaimable:     268052 kB
SUnreclaim:       371432 kB
";

    #[test]
    fn parse() {
        let free = parse_meminfo(MEMINFO).unwrap();

        assert_eq!(65083032 * 1024, free.mem.total);
        assert_eq!((65083032 - 54095372) * 1024, free.mem.used);
        assert_eq!(43608388 * 1024, free.mem.free);
        assert_eq!(631916 * 1024, free.mem.shared);
        assert_eq!(6528 * 1024, free.mem.buffers);
        assert_eq!((11571844 + 268052) * 1024, free.mem.cache);
        assert_eq!(54095372 * 1024, free.mem.available);

        assert_eq!(8000508 * 1024, free.swap.total);
        assert_eq!(0, free.swap.used);
        assert_eq!(8000508 * 1024, free.swap.free);
        assert_eq!(0, free.swap.cache);
    }

    #[test]
    fn parse_trimmed() {
        // A container may provide a file without `KReclaimable` and with the items in another order.
        const TRIMMED: &[u8] = b"MemFree:         1000 kB
MemTotal:        4000 kB
Cached:           500 kB
Buffers:          100 kB
";

        let free = parse_meminfo(TRIMMED).unwrap();

        assert_eq!(4000 * 1024, free.mem.total);
        assert_eq!(1000 * 1024, free.mem.free);
        assert_eq!(500 * 1024, free.mem.cache);
        assert_eq!(1600 * 1024, free.mem.available);
        assert_eq!((4000 - 1600) * 1024, free.mem.used);
        assert_eq!(0, free.swap.total);
    }
}
