mod vm_stat;

use std::{
    fs::File,
    io::{self, ErrorKind, Read},
};

pub use vm_stat::*;

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Default, Debug, Clone)]
pub struct Mem {
    /// `MemTotal`
    pub total:     u64,
    /// `MemTotal - MemAvailable`, the same as the `used` column of the `free` command in procps-ng 4.x
    pub used:      u64,
    /// `MemFree`
    pub free:      u64,
    /// `Shmem`
    pub shared:    u64,
    /// `Buffers`
    pub buffers:   u64,
    /// `Cached + KReclaimable`, the page cache plus the reclaimable kernel memory
    pub cache:     u64,
    /// `MemAvailable`
    pub available: u64,
}

#[derive(Default, Debug, Clone)]
pub struct Swap {
    /// `SwapTotal`
    pub total: u64,
    /// `SwapTotal - SwapFree - SwapCached`
    pub used:  u64,
    /// `SwapFree`
    pub free:  u64,
    /// `SwapCached`
    pub cache: u64,
}

#[derive(Default, Debug, Clone)]
pub struct Free {
    pub mem:  Mem,
    pub swap: Swap,
}

fn parse_meminfo<R: Read>(reader: R) -> Result<Free, ScannerError> {
    // These items must be listed in the same order as they appear in the file.
    const USEFUL_ITEMS: [&[u8]; 10] = [
        b"MemTotal",
        b"MemFree",
        b"MemAvailable",
        b"Buffers",
        b"Cached",
        b"SwapCached",
        b"SwapTotal",
        b"SwapFree",
        b"Shmem",
        b"KReclaimable",
    ];

    let mut sc: ScannerAscii<R, 768> = ScannerAscii::new2(reader);

    let mut item_values = [0u64; USEFUL_ITEMS.len()];

    for (i, &item) in USEFUL_ITEMS.iter().enumerate() {
        loop {
            let label = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            if label.starts_with(item) {
                let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

                item_values[i] = value * 1024;

                sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

                break;
            } else {
                sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            }
        }
    }

    let total = item_values[0];
    let free = item_values[1];
    let available = item_values[2];
    let buffers = item_values[3];
    let cached = item_values[4];
    let swap_cached = item_values[5];
    let swap_total = item_values[6];
    let swap_free = item_values[7];
    let shmem = item_values[8];
    let k_reclaimable = item_values[9];

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
pub fn free() -> Result<Free, ScannerError> {
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
}
