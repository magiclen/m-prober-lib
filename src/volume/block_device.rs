use std::{
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// The attributes of a block device, read from the `/sys/class/block` folder.
#[derive(Default, Debug, Clone)]
pub struct BlockDeviceInfo {
    /// Whether the device is a rotational disk (HDD) rather than an SSD or a virtual device.
    pub rotational: bool,
    /// Whether the device is removable, e.g. a USB stick or an optical disc.
    pub removable:  bool,
    /// The size of the device in bytes.
    pub size:       u64,
    /// The model name of the disk, e.g. `Samsung SSD 980 PRO 1TB`. It is `None` for virtual devices like `loop0` or `dm-0`.
    pub model:      Option<String>,
}

/// Get the attributes of a block device (a disk or a partition) by reading files in the `/sys/class/block/DEVICE` folder. For a partition, `rotational`, `removable` and `model` come from its parent disk.
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

    // The name must be a single path component.
    if device.is_empty() || device == "." || device == ".." || device.contains('/') {
        return Err(io::Error::from(ErrorKind::InvalidInput).into());
    }

    let path = Path::new("/sys/class/block").join(device);

    // `size` is in 512-byte sectors, regardless of the device's sector size.
    let size = read_sysfs_number::<u64, _>(path.join("size"))? * 512;

    // A partition has no `queue` folder and no `device` link, but `..` of its symlinked folder is its parent disk.
    let disk_path = if path.join("partition").exists() { path.join("..") } else { path };

    let rotational =
        read_sysfs_number::<u8, _>(disk_path.join("queue/rotational")).is_ok_and(|v| v == 1);

    let removable = read_sysfs_number::<u8, _>(disk_path.join("removable")).is_ok_and(|v| v == 1);

    // SCSI and NVMe pad the model name with spaces.
    let model = read_sysfs_string(disk_path.join("device/model"))
        .ok()
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty());

    Ok(BlockDeviceInfo {
        rotational,
        removable,
        size,
        model,
    })
}
