use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_file},
};

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

fn parse_vm_stat(data: &[u8]) -> Result<VmStat, Error> {
    const USEFUL_ITEMS_COUNT: usize = 6;

    let mut sc = ScannerU8SliceAscii::new(data);

    let mut vm_stat = VmStat::default();

    let mut remaining = USEFUL_ITEMS_COUNT;

    while let Some(label) = sc.next()? {
        let value = sc.next_u64()?.or_eof()?;

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

/// Get paging counters like the `vmstat` command by reading the `/proc/vmstat` file.
///
/// ```rust
/// use mprober_lib::memory;
///
/// let vm_stat = memory::get_vm_stat().unwrap();
///
/// println!("{vm_stat:#?}");
/// ```
#[inline]
pub fn get_vm_stat() -> Result<VmStat, Error> {
    parse_vm_stat(&read_file("/proc/vmstat", 8192)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VMSTAT: &[u8] = b"nr_free_pages 10902097
nr_zone_inactive_anon 6612
pgpgin 12885269
pgpgout 27766772
pswpin 12
pswpout 34
pgalloc_dma 0
pgfault 264080294
pgmajfault 78516
pgsteal_kswapd 0
";

    #[test]
    fn parse() {
        let vm_stat = parse_vm_stat(VMSTAT).unwrap();

        assert_eq!(12885269, vm_stat.pages_in);
        assert_eq!(27766772, vm_stat.pages_out);
        assert_eq!(12, vm_stat.swap_in);
        assert_eq!(34, vm_stat.swap_out);
        assert_eq!(264080294, vm_stat.page_faults);
        assert_eq!(78516, vm_stat.major_page_faults);
    }
}
