use std::{collections::HashMap, fs, os::unix::fs::MetadataExt};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_file, read_link_name, unescape_octal},
};

/// The mount points of one block device, read from the `/proc/mounts` file. [`crate::volume::MountInfo`] is the public superset of this, so this only feeds [`get_volumes`](crate::volume::get_volumes).
#[derive(Default, Debug, Clone)]
pub(crate) struct Mount {
    /// The file system type, e.g. `ext4` or `btrfs`.
    pub fs_type: String,
    /// Every path this device is mounted at, in the order of the `/proc/mounts` file.
    pub points:  Vec<String>,
}

/// Parse the lines of `/proc/mounts` whose source starts with `/dev/`. Each entry is the device path, the mount point and the file system type.
fn parse_mounts(data: &[u8]) -> Result<Vec<(String, String, String)>, Error> {
    // Only ASCII whitespace separates the fields, because the kernel escapes it inside the fields as octal sequences.
    let mut sc = ScannerU8SliceAscii::new(data);

    let mut entries = Vec::with_capacity(8);

    while let Some(device_path) = sc.next()? {
        if device_path.starts_with(b"/dev/") {
            let device_path = String::from_utf8_lossy(&unescape_octal(device_path)).into_owned();

            let point = sc.next()?.or_eof()?;

            // A mount point containing a space is written as `\040` in this file.
            let point = String::from_utf8_lossy(&unescape_octal(point)).into_owned();

            let fs_type = sc.next()?.or_eof()?;

            let fs_type = String::from_utf8_lossy(fs_type).into_owned();

            entries.push((device_path, point, fs_type));
        }

        // A `None` here only means the file ended without a trailing newline, which the loop condition handles.
        sc.drop_next_line()?;
    }

    Ok(entries)
}

/// Look up the name the kernel gives a block device in sysfs, which is the name `/proc/diskstats` uses.
fn device_name_by_dev(dev: u64) -> Option<String> {
    let major = libc::major(dev);
    let minor = libc::minor(dev);

    read_link_name(format!("/sys/dev/block/{major}:{minor}"))
}

/// Resolve a device path from `/proc/mounts` to the device name used in `/proc/diskstats`.
fn resolve_device_name(device_path: &str, point: &str) -> String {
    // The metadata is followed through symlinks, so `/dev/mapper/*` and `/dev/disk/by-uuid/*` need no separate `realpath` call.
    if let Some(name) = fs::metadata(device_path).ok().and_then(|m| device_name_by_dev(m.rdev())) {
        return name;
    }

    // A system booted without an initramfs mounts its root file system as `/dev/root`, a node that no longer exists afterwards, so the device of the mounted file system is asked instead. A file system without a block device of its own (e.g. btrfs) has an anonymous number that sysfs does not list.
    if let Some(name) = fs::metadata(point).ok().and_then(|m| device_name_by_dev(m.dev())) {
        return name;
    }

    // Without sysfs, or for a device node that does not exist in this mount namespace, the last path component is the best guess.
    let name = device_path.rsplit('/').next().unwrap_or(device_path);

    name.to_string()
}

/// Get mounting points of all block devices by reading the `/proc/mounts` file. The keys are device names as they appear in `/proc/diskstats`.
pub(crate) fn get_mounts() -> Result<HashMap<String, Mount>, Error> {
    let entries = parse_mounts(&read_file("/proc/mounts", 8192)?)?;

    let mut mounts: HashMap<String, Mount> = HashMap::with_capacity(entries.len());

    // Every lookup costs a `stat` and a `readlink`, so a device that is mounted more than once (two btrfs subvolumes, a bind mount) is resolved only for its first mount.
    let mut resolved: HashMap<String, String> = HashMap::with_capacity(entries.len());

    for (device_path, point, fs_type) in entries {
        let device = resolved
            .entry(device_path)
            .or_insert_with_key(|path| resolve_device_name(path, &point))
            .clone();

        let mount = mounts.entry(device).or_default();

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
