use std::io::ErrorKind;

use crate::scanner_rust::{ScannerAscii, ScannerError};

#[derive(Debug, Clone)]
pub struct Mem {
    pub total: usize,
    /// total - free - buffers - cached - total_cached; total_cached = cached + slab - s_unreclaim
    pub used: usize,
    pub free: usize,
    pub shared: usize,
    pub buffers: usize,
    pub cache: usize,
    pub available: usize,
}

#[derive(Debug, Clone)]
pub struct Swap {
    pub total: usize,
    /// swap_total - swap_free - swap_cached
    pub used: usize,
    pub free: usize,
    pub cache: usize,
}

#[derive(Debug, Clone)]
pub struct Free {
    pub mem: Mem,
    pub swap: Swap,
}

/// Get memory information like the `free` command by reading the `/proc/meminfo` file.
pub fn free() -> Result<Free, ScannerError> {
    const USEFUL_ITEMS: [&'static [u8]; 11] = [
        b"MemTotal",
        b"MemFree",
        b"MemAvailable",
        b"Buffers",
        b"Cached",
        b"SwapCached",
        b"SwapTotal",
        b"SwapFree",
        b"Shmem",
        b"Slab",
        b"SUnreclaim",
    ];

    let mut sc = ScannerAscii::scan_path("/proc/meminfo")?;

    let mut item_values = [0usize; USEFUL_ITEMS.len()];

    for (i, &item) in USEFUL_ITEMS.iter().enumerate() {
        loop {
            let label = sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?;

            if label.starts_with(item) {
                let value = sc.next_usize()?.ok_or(ErrorKind::UnexpectedEof)?;

                item_values[i] = value * 1024;

                sc.drop_next()?;

                break;
            } else {
                sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
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
    let slab = item_values[9];
    let s_unreclaim = item_values[10];

    let total_cached = cached + slab - s_unreclaim;

    let mem = Mem {
        total,
        used: total - free - buffers - total_cached,
        free,
        shared: shmem,
        buffers,
        cache: total_cached,
        available,
    };

    let swap = Swap {
        total: swap_total,
        used: swap_total - swap_free - swap_cached,
        free: swap_free,
        cache: swap_cached,
    };

    Ok(Free {
        mem,
        swap,
    })
}
