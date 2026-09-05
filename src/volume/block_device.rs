use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{is_single_path_component, read_sysfs_number, read_sysfs_string},
};

/// The disk a partition belongs to.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PartitionInfo {
    /// The name of the parent disk, e.g. `nvme0n1` for `nvme0n1p1`.
    pub disk:   String,
    /// The number of this partition, which is `1` for `nvme0n1p1`.
    pub number: u32,
}

/// The attributes of a block device, read from the `/sys/class/block` folder.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockDeviceInfo {
    /// Whether the device is a rotational disk (HDD) rather than an SSD or a virtual device. It is `None` when the driver does not report it, which is not the same as an SSD.
    pub rotational:          Option<bool>,
    /// Whether the device is removable, e.g. a USB stick or an optical disc. It is `None` when the driver does not report it.
    pub removable:           Option<bool>,
    /// Whether the device is read-only. It is `None` when the driver does not report it.
    pub read_only:           Option<bool>,
    /// The size of the device in bytes.
    pub size:                u64,
    /// The size of the smallest unit the device can address in bytes, which is `512` for most devices. It is `None` when the driver does not report it.
    pub logical_block_size:  Option<u64>,
    /// The size of the smallest unit the device can write without a read-modify-write cycle in bytes, e.g. `4096` for an advanced format disk. It is `None` when the driver does not report it.
    pub physical_block_size: Option<u64>,
    /// The I/O scheduler in use, e.g. `none`, `mq-deadline` or `bfq`. It is `None` for a device without a request queue, like `dm-0`.
    pub scheduler:           Option<String>,
    /// The number of requests the queue holds at most.
    pub nr_requests:         Option<u64>,
    /// The number of bytes the device can discard (TRIM) in one request, which is `0` when the device does not support discarding. It is `None` when the driver does not report it at all.
    pub discard_max_bytes:   Option<u64>,
    /// The model name of the disk, e.g. `Samsung SSD 980 PRO 1TB`. It is `None` for virtual devices like `loop0` or `dm-0`.
    pub model:               Option<String>,
    /// The serial number of the disk. It is `None` when the driver does not report one.
    pub serial:              Option<String>,
    /// The disk this partition belongs to. It is `None` when the device is a whole disk.
    pub partition:           Option<PartitionInfo>,
}

/// Extract the active choice from a sysfs attribute that lists every choice with the active one in brackets, e.g. `[none] mq-deadline bfq`.
fn parse_active_choice(value: &str) -> Option<&str> {
    let start = value.find('[')?;
    let end = value[start..].find(']')? + start;

    Some(&value[(start + 1)..end])
}

/// Read a sysfs attribute that lists choices with the active one in brackets.
#[inline]
fn read_active_choice<P: AsRef<Path>>(path: P) -> Option<String> {
    let value = read_sysfs_string(path).ok()?;

    parse_active_choice(&value).map(|choice| choice.to_owned())
}

/// Get the attributes of a block device (a disk or a partition) by reading files in the `/sys/class/block/DEVICE` folder. For a partition, the attributes that only a whole disk has come from its parent disk.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let volumes = volume::get_volumes().unwrap();
///
/// if let Some(volume) = volumes.first() {
///     let block_device_info =
///         volume::get_block_device_info(&volume.device).unwrap();
///
///     println!("{block_device_info:#?}");
/// }
/// ```
pub fn get_block_device_info<S: AsRef<str>>(device: S) -> Result<BlockDeviceInfo, Error> {
    let device = device.as_ref();

    if !is_single_path_component(device) {
        return Err(io::Error::from(ErrorKind::InvalidInput).into());
    }

    let path = Path::new("/sys/class/block").join(device);

    // `size` is in 512-byte sectors, regardless of the device's sector size.
    let size = read_sysfs_number::<u64, _>(path.join("size"))? * 512;

    let read_only = read_sysfs_number::<u8, _>(path.join("ro")).ok().map(|v| v == 1);

    // A partition has no `queue` folder and no `device` link, but `..` of its symlinked folder is its parent disk.
    let partition = match read_sysfs_number::<u32, _>(path.join("partition")) {
        Ok(number) => {
            // The parent disk is the folder the partition folder lives in, whose name sysfs does not repeat anywhere else.
            let disk = fs::read_link(&path)
                .ok()
                .and_then(|link| {
                    link.parent().and_then(|parent| {
                        parent.file_name().map(|name| name.to_string_lossy().into_owned())
                    })
                })
                .unwrap_or_default();

            Some(PartitionInfo {
                disk,
                number,
            })
        },
        Err(_) => None,
    };

    let disk_path = if partition.is_some() { path.join("..") } else { path };

    let rotational =
        read_sysfs_number::<u8, _>(disk_path.join("queue/rotational")).ok().map(|v| v == 1);

    let removable = read_sysfs_number::<u8, _>(disk_path.join("removable")).ok().map(|v| v == 1);

    let logical_block_size = read_sysfs_number(disk_path.join("queue/logical_block_size")).ok();
    let physical_block_size = read_sysfs_number(disk_path.join("queue/physical_block_size")).ok();

    let scheduler = read_active_choice(disk_path.join("queue/scheduler"));

    let nr_requests = read_sysfs_number(disk_path.join("queue/nr_requests")).ok();

    let discard_max_bytes = read_sysfs_number(disk_path.join("queue/discard_max_bytes")).ok();

    // SCSI and NVMe pad the model name with spaces.
    let model = read_sysfs_string(disk_path.join("device/model"))
        .ok()
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty());

    // NVMe reports the serial number under `device`, while SCSI reports it under `device/vpd_pg80`, which is not text.
    let serial = read_sysfs_string(disk_path.join("device/serial"))
        .ok()
        .map(|serial| serial.trim().to_owned())
        .filter(|serial| !serial.is_empty());

    Ok(BlockDeviceInfo {
        rotational,
        removable,
        read_only,
        size,
        logical_block_size,
        physical_block_size,
        scheduler,
        nr_requests,
        discard_max_bytes,
        model,
        serial,
        partition,
    })
}

/// Get the attributes of every block device by reading the `/sys/class/block` folder, including the disks and the partitions that are not mounted. The devices are ordered by their names.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let block_devices = volume::get_block_devices().unwrap();
///
/// println!("{block_devices:#?}");
/// ```
pub fn get_block_devices() -> Result<Vec<(String, BlockDeviceInfo)>, Error> {
    let read_dir = match fs::read_dir("/sys/class/block") {
        Ok(read_dir) => read_dir,
        // A kernel without sysfs support simply has no devices.
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let mut names: Vec<String> = Vec::new();

    for entry in read_dir {
        let entry = entry?;

        names.push(entry.file_name().to_string_lossy().into_owned());
    }

    names.sort_unstable();

    let mut block_devices = Vec::with_capacity(names.len());

    for name in names {
        // A device can be removed while the folder is being scanned, so an unreadable one is skipped.
        if let Ok(block_device_info) = get_block_device_info(&name) {
            block_devices.push((name, block_device_info));
        }
    }

    Ok(block_devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_choice() {
        assert_eq!(Some("none"), parse_active_choice("[none] mq-deadline kyber bfq"));
        assert_eq!(Some("bfq"), parse_active_choice("none mq-deadline kyber [bfq]"));

        // A device without a request queue reports a single value with no brackets.
        assert_eq!(None, parse_active_choice("none"));
    }
}
