use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, ErrorKind, Read},
    path::Path,
};

use crate::{Error, scanner_rust::ScannerAscii, utils::unescape_octal};

/// The mount points of one block device, read from the `/proc/mounts` file.
#[derive(Default, Debug, Clone)]
pub struct Mount {
    /// The file system type, e.g. `ext4` or `btrfs`.
    pub fs_type: String,
    /// Every path this device is mounted at, in the order of the `/proc/mounts` file.
    pub points:  Vec<String>,
}

/// Parse the lines of `/proc/mounts` whose source starts with `/dev/`. Each entry is the device path, the mount point and the file system type.
fn parse_mounts<R: Read>(reader: R) -> Result<Vec<(String, String, String)>, Error> {
    // Only ASCII whitespace separates the fields, because the kernel escapes it inside the fields as octal sequences.
    let mut sc: ScannerAscii<R, 1024> = ScannerAscii::new2(reader);

    let mut entries = Vec::with_capacity(1);

    while let Some(device_path) = sc.next_raw()? {
        if device_path.starts_with(b"/dev/") {
            let device_path = String::from_utf8_lossy(&unescape_octal(&device_path)).into_owned();

            let point = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            // A mount point containing a space is written as `\040` in this file.
            let point = String::from_utf8_lossy(&unescape_octal(&point)).into_owned();

            let fs_type = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

            let fs_type = String::from_utf8_lossy(&fs_type).into_owned();

            entries.push((device_path, point, fs_type));
        }

        sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    Ok(entries)
}

/// Resolve a device path from `/proc/mounts` to the device name used in `/proc/diskstats`.
fn resolve_device_name(device_path: &str) -> String {
    let path = Path::new(device_path);

    // Only symlinks like `/dev/mapper/*` or `/dev/disk/by-uuid/*` need to be resolved to the real device name, and checking that first is cheaper than always calling `realpath`.
    let is_symlink =
        fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink());

    if is_symlink
        && let Some(name) = path.canonicalize().ok().and_then(|real_path| {
            real_path.file_name().map(|name| name.to_string_lossy().into_owned())
        })
    {
        return name;
    }

    device_path[5..].to_string()
}

/// Get mounting points of all block devices by reading the `/proc/mounts` file. The keys are device names as they appear in `/proc/diskstats`.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let mounts = volume::get_mounts().unwrap();
///
/// println!("{mounts:#?}");
/// ```
pub fn get_mounts() -> Result<HashMap<String, Mount>, Error> {
    let entries = parse_mounts(File::open("/proc/mounts")?)?;

    let mut mounts: HashMap<String, Mount> = HashMap::with_capacity(entries.len());

    for (device_path, point, fs_type) in entries {
        let mount = mounts.entry(resolve_device_name(&device_path)).or_default();

        if mount.points.is_empty() {
            mount.fs_type = fs_type;
        }

        mount.points.push(point);
    }

    Ok(mounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNTS: &[u8] = b"sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0
/dev/nvme0n1p4 / btrfs rw,relatime,ssd,subvol=/@ 0 0
/dev/nvme0n1p4 /home btrfs rw,relatime,ssd,subvol=/@home 0 0
/dev/sda1 /media/user/my\\040disk ext4 rw,relatime 0 0
/dev/sdb1 /media/user/\xe6\x88\x91\xe7\x9a\x84\xe3\x80\x80\xe7\xa3\x81\xe7\xa2\x9f vfat rw 0 0
tmpfs /run tmpfs rw,nosuid,nodev 0 0
";

    #[test]
    fn parse() {
        let entries = parse_mounts(MOUNTS).unwrap();

        assert_eq!(4, entries.len());

        assert_eq!(
            ("/dev/nvme0n1p4".to_string(), "/".to_string(), "btrfs".to_string()),
            entries[0]
        );
        assert_eq!("/home", entries[1].1);
        assert_eq!("/media/user/my disk", entries[2].1);

        // U+3000 is a Unicode whitespace, but the kernel does not escape it, so it must not split the fields.
        assert_eq!("/media/user/我的\u{3000}磁碟", entries[3].1);
        assert_eq!("vfat", entries[3].2);
    }
}
