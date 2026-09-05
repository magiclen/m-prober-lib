use std::{
    collections::HashSet,
    ffi::CString,
    hash::{Hash, Hasher},
    io::{self, ErrorKind},
    mem::zeroed,
    thread::sleep,
    time::Duration,
};

use crate::{
    scanner_rust::{ScannerAscii, ScannerError},
    volume::{VolumeSpeed, VolumeStat, get_mounts},
};

#[derive(Debug, Clone, Eq)]
pub struct Volume {
    pub device:    String,
    pub stat:      VolumeStat,
    /// The size of the file system in bytes.
    pub size:      u64,
    /// `size - available`, which includes the blocks reserved for the root user.
    pub used:      u64,
    /// The space available to unprivileged users in bytes.
    pub available: u64,
    pub points:    Vec<String>,
}

impl Hash for Volume {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.device.hash(state)
    }
}

impl PartialEq for Volume {
    #[inline]
    fn eq(&self, other: &Volume) -> bool {
        self.device.eq(&other.device)
    }
}

/// Get volume information by reading the `/proc/diskstats` file and using the `statvfs` function in libc.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let volumes = volume::get_volumes().unwrap();
///
/// println!("{volumes:#?}");
/// ```
pub fn get_volumes() -> Result<Vec<Volume>, ScannerError> {
    let mut mounts = get_mounts()?;

    let mut sc = ScannerAscii::scan_path("/proc/diskstats")?;

    let mut volumes = Vec::with_capacity(1);

    loop {
        if sc.drop_next()?.is_none() {
            break;
        }

        sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        let device = String::from_utf8_lossy(
            &sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?,
        )
        .into_owned();

        if let Some(points) = mounts.remove(&device) {
            let reads_completed =
                sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            // reads merged
            sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            // The sector fields in `/proc/diskstats` always use 512-byte sectors, regardless of the device's sector size.
            let read_bytes = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * 512;

            let read_time = Duration::from_millis(
                sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?,
            );

            let writes_completed =
                sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            // writes merged
            sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            let write_bytes =
                sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * 512;

            let write_time = Duration::from_millis(
                sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?,
            );

            let io_in_progress = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            let io_time = Duration::from_millis(
                sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?,
            );

            let (size, used, available) = {
                let path = CString::new(points[0].as_bytes()).unwrap();

                let mut stats: libc::statvfs = unsafe { zeroed() };

                let rtn = unsafe { libc::statvfs(path.as_ptr(), &mut stats as *mut _) };

                if rtn != 0 {
                    return Err(io::Error::last_os_error().into());
                }

                // POSIX defines the block counts in units of `f_frsize`, not `f_bsize`.
                #[allow(clippy::unnecessary_cast)]
                {
                    let fragment_size = stats.f_frsize as u64;
                    let blocks = stats.f_blocks as u64;
                    let available_blocks = stats.f_bavail as u64;

                    (
                        fragment_size * blocks,
                        fragment_size * blocks.saturating_sub(available_blocks),
                        fragment_size * available_blocks,
                    )
                }
            };

            let stat = VolumeStat {
                reads_completed,
                read_bytes,
                read_time,
                writes_completed,
                write_bytes,
                write_time,
                io_in_progress,
                io_time,
            };

            let volume = Volume {
                device,
                stat,
                size,
                used,
                available,
                points,
            };

            volumes.push(volume);
        }

        sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    Ok(volumes)
}

/// Get volume information by reading the `/proc/diskstats` file and using the `statvfs` function in libc. And measure the speed within a specific time interval.
///
/// ```rust
/// use std::time::Duration;
///
/// use mprober_lib::volume;
///
/// let volumes_with_speed =
///     volume::get_volumes_with_speed(Duration::from_millis(100)).unwrap();
///
/// for (volume, volume_speed) in volumes_with_speed {
///     println!("{}: ", volume.device);
///     println!("    Read: {:.1} B/s", volume_speed.read);
///     println!("    Write: {:.1} B/s", volume_speed.write);
///     println!("    Utilization: {:.1}%", volume_speed.utilization * 100.0);
/// }
/// ```
pub fn get_volumes_with_speed(
    interval: Duration,
) -> Result<Vec<(Volume, VolumeSpeed)>, ScannerError> {
    let pre_volumes = get_volumes()?;

    let pre_volumes_length = pre_volumes.len();

    let mut pre_volumes_hashset = HashSet::with_capacity(pre_volumes_length);

    for pre_volume in pre_volumes {
        pre_volumes_hashset.insert(pre_volume);
    }

    sleep(interval);

    let volumes = get_volumes()?;

    let mut volumes_with_speed = Vec::with_capacity(volumes.len().min(pre_volumes_length));

    for volume in volumes {
        if let Some(pre_volume) = pre_volumes_hashset.get(&volume) {
            let volume_speed = pre_volume.stat.compute_speed(&volume.stat, interval);

            volumes_with_speed.push((volume, volume_speed));
        }
    }

    Ok(volumes_with_speed)
}
