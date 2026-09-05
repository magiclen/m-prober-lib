use std::io::{self, ErrorKind};

use crate::{Error, scanner_rust::ScannerAscii};

/// Paging counters read from the `/proc/vmstat` file.
#[derive(Default, Debug, Clone)]
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

    let mut sc: ScannerAscii<_, 1024> = ScannerAscii::scan_path2("/proc/vmstat")?;

    let mut vm_stat = VmStat::default();

    let mut remaining = USEFUL_ITEMS_COUNT;

    while let Some(label) = sc.next_raw()? {
        let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        match label.as_slice() {
            b"pgpgin" => vm_stat.pages_in = value,
            b"pgpgout" => vm_stat.pages_out = value,
            b"pswpin" => vm_stat.swap_in = value,
            b"pswpout" => vm_stat.swap_out = value,
            b"pgfault" => vm_stat.page_faults = value,
            b"pgmajfault" => vm_stat.major_page_faults = value,
            _ => continue,
        }

        remaining -= 1;

        // The file has more than a hundred lines, so stop as soon as everything is found.
        if remaining == 0 {
            break;
        }
    }

    Ok(vm_stat)
}
