use std::io::{self, ErrorKind};

use scanner_rust::ScannerU8SliceAscii;

use crate::{Error, utils::read_file};

/// Paging counters read from the `/proc/vmstat` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VmStat {
    /// KiB paged in from block devices (`pgpgin`).
    pub pages_in:          u64,
    /// KiB paged out to block devices (`pgpgout`).
    pub pages_out:         u64,
    /// Pages swapped in (`pswpin`).
    pub swap_in:           u64,
    /// Pages swapped out (`pswpout`).
    pub swap_out:          u64,
    /// Page faults (`pgfault`).
    pub page_faults:       u64,
    /// Major page faults, which needed disk I/O (`pgmajfault`).
    pub major_page_faults: u64,
}

/// Get paging counters like the `vmstat` command by reading the `/proc/vmstat` file.
///
/// ```rust
/// use mprober_lib::memory;
///
/// let vm_stat = memory::get_vm_stat().unwrap();
///
/// println!("{vm_stat:#?}");
/// ```
pub fn get_vm_stat() -> Result<VmStat, Error> {
    const USEFUL_ITEMS_COUNT: usize = 6;

    let data = read_file("/proc/vmstat", 8192)?;

    let mut sc = ScannerU8SliceAscii::new(&data);

    let mut vm_stat = VmStat::default();

    let mut remaining = USEFUL_ITEMS_COUNT;

    while let Some(label) = sc.next()? {
        let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        match label {
            b"pgpgin" => vm_stat.pages_in = value,
            b"pgpgout" => vm_stat.pages_out = value,
            b"pswpin" => vm_stat.swap_in = value,
            b"pswpout" => vm_stat.swap_out = value,
            b"pgfault" => vm_stat.page_faults = value,
            b"pgmajfault" => vm_stat.major_page_faults = value,
            _ => continue,
        }

        // The file has more than a hundred lines, so stop as soon as everything is found. The count is saturating so that a repeated label cannot underflow it.
        remaining = remaining.saturating_sub(1);

        if remaining == 0 {
            break;
        }
    }

    Ok(vm_stat)
}
