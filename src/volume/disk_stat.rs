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

/// Parse the content of `/proc/diskstats`, whose lines start with the major and the minor number of the device.
fn parse_disk_stats(data: &[u8]) -> Result<HashMap<String, VolumeStat>, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

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

/// Get the I/O counters of every block device by reading the `/proc/diskstats` file. The keys are device names, e.g. `nvme0n1` or `nvme0n1p1`. Unlike [`crate::volume::get_volumes`], the devices that are not mounted are included too, e.g. whole disks, swap partitions and the members of a RAID array.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let disk_stats = volume::get_disk_stats().unwrap();
///
/// println!("{disk_stats:#?}");
/// ```
#[inline]
pub fn get_disk_stats() -> Result<HashMap<String, VolumeStat>, Error> {
    parse_disk_stats(&read_file("/proc/diskstats", 8192)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISKSTATS: &[u8] =
        b" 259       0 nvme0n1 141481 21114 17103283 43061 1293837 434373 50291767 448262 0 356452 668635 392811 0 133826107 41425 22580 24783
 259       1 nvme0n1p1 358 1489 20904 137 2 0 2 0 0 132 137 0 0 0 0 0 0
";

    #[test]
    fn parse() {
        let disk_stats = parse_disk_stats(DISKSTATS).unwrap();

        assert_eq!(2, disk_stats.len());

        let stat = &disk_stats["nvme0n1"];

        assert_eq!(141481, stat.reads_completed);
        // The sector counts are always in 512-byte sectors.
        assert_eq!(17103283 * 512, stat.read_bytes);
        assert_eq!(Duration::from_millis(43061), stat.read_time);
        assert_eq!(1293837, stat.writes_completed);
        assert_eq!(50291767 * 512, stat.write_bytes);
        assert_eq!(Duration::from_millis(448262), stat.write_time);
        assert_eq!(0, stat.io_in_progress);
        assert_eq!(Duration::from_millis(356452), stat.io_time);
        assert_eq!(Duration::from_millis(668635), stat.weighted_io_time);

        // The discard fields exist since Linux 4.18 and the flush ones since Linux 5.5.
        assert_eq!(392811, stat.discards_completed);
        assert_eq!(133826107 * 512, stat.discard_bytes);
        assert_eq!(Duration::from_millis(41425), stat.discard_time);
        assert_eq!(22580, stat.flushes_completed);
        assert_eq!(Duration::from_millis(24783), stat.flush_time);

        assert_eq!(358, disk_stats["nvme0n1p1"].reads_completed);
    }
}
