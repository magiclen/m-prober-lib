use std::collections::HashSet;
use std::ffi::CString;
use std::hash::{Hash, Hasher};
use std::io::{self, ErrorKind};
use std::mem::zeroed;
use std::thread::sleep;
use std::time::Duration;

use crate::scanner_rust::{ScannerAscii, ScannerError};

use crate::volume::{get_mounts, VolumeSpeed, VolumeStat};

#[derive(Debug, Clone, Eq)]
pub struct Volume {
    pub device: String,
    pub stat: VolumeStat,
    pub size: u64,
    pub used: u64,
    pub points: Vec<String>,
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
/// extern crate mprober_lib;
///
/// use mprober_lib::volume;
///
/// let volumes = volume::get_volumes().unwrap();
///
/// println!("{:#?}", volumes);
/// ```
pub fn get_volumes() -> Result<Vec<Volume>, ScannerError> {
    let mut mounts = get_mounts()?;

    let mut sc = ScannerAscii::scan_path("/proc/diskstats")?;

    let mut volumes = Vec::with_capacity(1);

    loop {
        if sc.drop_next()?.is_none() {
            break;
        }

        sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;

        let device =
            unsafe { String::from_utf8_unchecked(sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?) };

        if let Some(points) = mounts.remove(&device) {
            for _ in 0..2 {
                sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;
            }

            let read_bytes = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

            for _ in 0..3 {
                sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;
            }

            let write_bytes = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

            for _ in 0..2 {
                sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;
            }

            let time_spent = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

            if time_spent > 0 {
                let (size, used) = {
                    let path = CString::new(points[0].as_bytes()).unwrap();

                    let mut stats: libc::statvfs = unsafe { zeroed() };

                    let rtn = unsafe { libc::statvfs(path.as_ptr(), &mut stats as *mut _) };

                    if rtn != 0 {
                        return Err(io::Error::last_os_error().into());
                    }

                    (
                        stats.f_bsize as u64 * stats.f_blocks as u64,
                        stats.f_bsize as u64 * (stats.f_blocks - stats.f_bavail) as u64,
                    )
                };

                let stat = VolumeStat {
                    read_bytes,
                    write_bytes,
                };

                let volume = Volume {
                    device,
                    stat,
                    size,
                    used,
                    points,
                };

                volumes.push(volume);
            }

            sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
        }
    }

    Ok(volumes)
}

/// Get volume information by reading the `/proc/diskstats` file and using the `statvfs` function in libc. And measure the speed within a specific time interval.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use std::time::Duration;
///
/// use mprober_lib::volume;
///
/// let volumes_with_speed = volume::get_volumes_with_speed(Duration::from_millis(100)).unwrap();
///
/// for (volume, volume_with_speed) in volumes_with_speed {
///     println!("{}: ", volume.device);
///     println!("    Read: {:.1} B/s", volume_with_speed.read);
///     println!("    Write: {:.1} B/s", volume_with_speed.write);
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
