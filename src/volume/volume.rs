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
    Error,
    scanner_rust::ScannerAscii,
    volume::{VolumeSpeed, VolumeStat, disk_stat::read_volume_stat, mounts::get_mounts},
};

/// One mounted block device. Two instances are equal when their device names are equal.
#[derive(Debug, Clone, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Volume {
    /// The device name as it appears in the `/proc/diskstats` file, e.g. `nvme0n1p1`.
    pub device:      String,
    /// The I/O counters of the device.
    pub stat:        VolumeStat,
    /// The size of the file system in bytes.
    pub size:        u64,
    /// `size - available`, which includes the blocks reserved for the root user. The `df` command shows `size - free` instead.
    pub used:        u64,
    /// The space available to unprivileged users in bytes.
    pub available:   u64,
    /// The free space in bytes, including the blocks reserved for the root user.
    pub free:        u64,
    /// The total number of inodes. Some file systems (e.g. btrfs) report `0` because they allocate inodes dynamically.
    pub inodes:      u64,
    /// The number of free inodes.
    pub inodes_free: u64,
    /// The file system type, e.g. `ext4`.
    pub fs_type:     String,
    /// Every path this device is mounted at. The sizes above are those of the first one.
    pub points:      Vec<String>,
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

/// The sizes of a file system, in bytes and in inodes.
struct FsUsage {
    size:        u64,
    used:        u64,
    available:   u64,
    free:        u64,
    inodes:      u64,
    inodes_free: u64,
}

/// Get the sizes of the file system at `point`. It returns `None` when the mount point cannot be reached.
fn statvfs(point: &str) -> Option<FsUsage> {
    let path = CString::new(point.as_bytes()).ok()?;

    let mut stats: libc::statvfs = unsafe { zeroed() };

    let rtn = unsafe { libc::statvfs(path.as_ptr(), &mut stats as *mut _) };

    if rtn != 0 {
        return None;
    }

    // POSIX defines the block counts in units of `f_frsize`, not `f_bsize`.
    #[allow(clippy::unnecessary_cast)]
    {
        let fragment_size = stats.f_frsize as u64;
        let blocks = stats.f_blocks as u64;
        let available_blocks = stats.f_bavail as u64;
        let free_blocks = stats.f_bfree as u64;

        Some(FsUsage {
            size:        fragment_size * blocks,
            used:        fragment_size * blocks.saturating_sub(available_blocks),
            available:   fragment_size * available_blocks,
            free:        fragment_size * free_blocks,
            inodes:      stats.f_files as u64,
            inodes_free: stats.f_ffree as u64,
        })
    }
}

/// Get volume information by reading the `/proc/diskstats` file and using the `statvfs` function in libc. A mounted device whose mount point cannot be reached is skipped.
///
/// `statvfs` has no timeout, so this blocks for as long as the file system takes to answer. A network mount whose server is unreachable can therefore make this never return, which is not the same as the skipped case above.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let volumes = volume::get_volumes().unwrap();
///
/// println!("{volumes:#?}");
/// ```
pub fn get_volumes() -> Result<Vec<Volume>, Error> {
    let mut mounts = get_mounts()?;

    let mut sc: ScannerAscii<_, 1024> = ScannerAscii::scan_path2("/proc/diskstats")?;

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

        if let Some(mount) = mounts.remove(&device) {
            let stat = read_volume_stat(&mut sc)?;

            // A mount point can be unreachable (a disconnected network device, a directory without the search permission), so a failure here only skips this volume.
            let Some(usage) = statvfs(&mount.points[0]) else {
                sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

                continue;
            };

            let volume = Volume {
                device,
                stat,
                size: usage.size,
                used: usage.used,
                available: usage.available,
                free: usage.free,
                inodes: usage.inodes,
                inodes_free: usage.inodes_free,
                fs_type: mount.fs_type,
                points: mount.points,
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
pub fn get_volumes_with_speed(interval: Duration) -> Result<Vec<(Volume, VolumeSpeed)>, Error> {
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
