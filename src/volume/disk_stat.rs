use std::{collections::HashMap, time::Duration};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_file},
    volume::VolumeStat,
};

/// Read the counters that follow the device name in a `/proc/diskstats` line. The line is not consumed to its end, because a caller may want to skip it instead.
pub(crate) fn read_volume_stat(sc: &mut ScannerU8SliceAscii<'_>) -> Result<VolumeStat, Error> {
    let reads_completed = sc.next_u64()?.or_eof()?;

    // reads merged
    sc.drop_next()?.or_eof()?;

    // The sector fields in `/proc/diskstats` always use 512-byte sectors, regardless of the device's sector size.
    let read_bytes = sc.next_u64()?.or_eof()? * 512;

    let read_time = Duration::from_millis(sc.next_u64()?.or_eof()?);

    let writes_completed = sc.next_u64()?.or_eof()?;

    // writes merged
    sc.drop_next()?.or_eof()?;

    let write_bytes = sc.next_u64()?.or_eof()? * 512;

    let write_time = Duration::from_millis(sc.next_u64()?.or_eof()?);

    let io_in_progress = sc.next_u64()?.or_eof()?;

    let io_time = Duration::from_millis(sc.next_u64()?.or_eof()?);

    let weighted_io_time = Duration::from_millis(sc.next_u64()?.or_eof()?);

    // The discard and flush fields exist since Linux 4.18 and 5.5 respectively.
    let discards_completed = sc.next_u64()?.or_eof()?;

    // discards merged
    sc.drop_next()?.or_eof()?;

    let discard_bytes = sc.next_u64()?.or_eof()? * 512;

    let discard_time = Duration::from_millis(sc.next_u64()?.or_eof()?);

    let flushes_completed = sc.next_u64()?.or_eof()?;

    let flush_time = Duration::from_millis(sc.next_u64()?.or_eof()?);

    Ok(VolumeStat {
        reads_completed,
        read_bytes,
        read_time,
        writes_completed,
        write_bytes,
        write_time,
        io_in_progress,
        io_time,
        weighted_io_time,
        discards_completed,
        discard_bytes,
        discard_time,
        flushes_completed,
        flush_time,
    })
}

/// Get the I/O counters of every block device by reading the `/proc/diskstats` file. The keys are device names, e.g. `nvme0n1` or `nvme0n1p1`. Unlike [`crate::volume::get_volumes`], the devices that are not mounted are included too, e.g. whole disks, swap partitions and the members of a RAID array.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let disk_stats = volume::get_disk_stats().unwrap();
///
/// println!("{disk_stats:#?}");
/// ```
pub fn get_disk_stats() -> Result<HashMap<String, VolumeStat>, Error> {
    let data = read_file("/proc/diskstats", 8192)?;

    let mut sc = ScannerU8SliceAscii::new(&data);

    let mut disk_stats = HashMap::with_capacity(16);

    loop {
        // The major and the minor numbers come before the device name.
        if sc.drop_next()?.is_none() {
            break;
        }

        sc.drop_next()?.or_eof()?;

        let device = String::from_utf8_lossy(sc.next()?.or_eof()?).into_owned();

        disk_stats.insert(device, read_volume_stat(&mut sc)?);

        // A `None` here only means the file ended without a trailing newline, which the loop condition handles.
        sc.drop_next_line()?;
    }

    Ok(disk_stats)
}
