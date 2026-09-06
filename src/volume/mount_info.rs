use std::io::{self, ErrorKind};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_file, unescape_octal},
};

/// One mount in the mount namespace of a process, read from the `/proc/PID/mountinfo` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MountInfo {
    /// The unique ID of this mount.
    pub id:            u32,
    /// The ID of the parent mount, which points to itself for the root of a mount namespace.
    pub parent_id:     u32,
    /// The major number of the device.
    pub major:         u32,
    /// The minor number of the device.
    pub minor:         u32,
    /// The path of this mount inside its file system, e.g. `/` or `/@home` for a btrfs subvolume.
    pub root:          String,
    /// The path this file system is mounted at.
    pub point:         String,
    /// The per-mount options, e.g. `rw,nosuid,relatime`.
    pub options:       String,
    /// The file system type, e.g. `ext4`, `nfs4` or `overlay`.
    pub fs_type:       String,
    /// The source of the mount, e.g. `/dev/nvme0n1p4`, `tmpfs` or `10.0.0.1:/export`.
    pub source:        String,
    /// The per-super-block options, which every mount of the same file system shares.
    pub super_options: String,
}

impl MountInfo {
    /// Whether `option` is set, either as a per-mount option or as a per-super-block option.
    pub fn has_option<S: AsRef<str>>(&self, option: S) -> bool {
        let option = option.as_ref();

        // An option is either a bare name or a `name=value` pair, so a prefix match alone would also hit a longer name.
        let matches = |options: &str| {
            options.split(',').any(|item| {
                item == option || item.split_once('=').is_some_and(|(name, _)| name == option)
            })
        };

        matches(&self.options) || matches(&self.super_options)
    }

    /// Whether the file system is mounted read-only.
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.has_option("ro")
    }
}

/// Read the next field of a `mountinfo` line, which the kernel always writes in full.
#[inline]
fn next_field<'a>(sc: &mut ScannerU8SliceAscii<'a>) -> Result<&'a [u8], Error> {
    sc.next()?.or_eof()
}

/// Parse one line of a `mountinfo` file.
fn parse_mount_info(line: &[u8]) -> Result<MountInfo, Error> {
    // The kernel escapes whitespace inside the fields as octal sequences, so ASCII whitespace always separates the fields.
    let mut sc = ScannerU8SliceAscii::new(line);

    let id = sc.next_u32()?.or_eof()?;
    let parent_id = sc.next_u32()?.or_eof()?;

    let (major, minor) = {
        // This field looks like `259:1`.
        let mut device = ScannerU8SliceAscii::new(next_field(&mut sc)?);

        let major = device.next_u32_until(":")?.ok_or(io::Error::from(ErrorKind::InvalidData))?;
        let minor = device.next_u32()?.ok_or(io::Error::from(ErrorKind::InvalidData))?;

        (major, minor)
    };

    let root = String::from_utf8_lossy(&unescape_octal(next_field(&mut sc)?)).into_owned();
    let point = String::from_utf8_lossy(&unescape_octal(next_field(&mut sc)?)).into_owned();
    let options = String::from_utf8_lossy(next_field(&mut sc)?).into_owned();

    // A variable number of optional fields follows, terminated by a single hyphen.
    loop {
        if next_field(&mut sc)? == b"-" {
            break;
        }
    }

    let fs_type = String::from_utf8_lossy(next_field(&mut sc)?).into_owned();
    let source = String::from_utf8_lossy(&unescape_octal(next_field(&mut sc)?)).into_owned();
    let super_options = String::from_utf8_lossy(next_field(&mut sc)?).into_owned();

    Ok(MountInfo {
        id,
        parent_id,
        major,
        minor,
        root,
        point,
        options,
        fs_type,
        source,
        super_options,
    })
}

/// Parse the content of a `mountinfo` file.
fn parse_mount_infos(data: &[u8]) -> Result<Vec<MountInfo>, Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    let mut mount_infos = Vec::with_capacity(16);

    while let Some(line) = lines.next_line()? {
        // A line the caller was fed without its fields, e.g. from a trimmed file, only loses that mount instead of ending the parsing.
        if let Ok(mount_info) = parse_mount_info(line) {
            mount_infos.push(mount_info);
        }
    }

    Ok(mount_infos)
}

/// Get every mount by reading the `/proc/self/mountinfo` file. Unlike the mount points of [`crate::volume::Volume`], this includes the file systems that are not backed by a block device, e.g. `tmpfs`, `nfs`, `overlay` and `zfs`.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let mount_infos = volume::get_mount_infos().unwrap();
///
/// println!("{mount_infos:#?}");
/// ```
#[inline]
pub fn get_mount_infos() -> Result<Vec<MountInfo>, Error> {
    parse_mount_infos(&read_file("/proc/self/mountinfo", 8 * 1024)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUNT_INFO: &[u8] =
        b"27 34 0:24 / /sys rw,nosuid,nodev,noexec,relatime shared:7 - sysfs sysfs rw
34 1 0:32 /@ / rw,relatime shared:1 - btrfs /dev/nvme0n1p4 rw,ssd,subvol=/@
95 34 259:1 / /boot/efi ro,relatime - vfat /dev/nvme0n1p1 ro,fmask=0077
120 34 0:59 / /media/user/my\\040disk rw,nosuid,nodev,relatime - ext4 /dev/sda1 rw
131 34 0:61 / /mnt/nas rw,relatime - nfs4 10.0.0.1:/export rw,vers=4.2
";

    #[test]
    fn parse() {
        let mount_infos = parse_mount_infos(MOUNT_INFO).unwrap();

        assert_eq!(5, mount_infos.len());

        assert_eq!(27, mount_infos[0].id);
        assert_eq!(34, mount_infos[0].parent_id);
        assert_eq!(0, mount_infos[0].major);
        assert_eq!(24, mount_infos[0].minor);
        assert_eq!("sysfs", mount_infos[0].fs_type);

        // The optional fields before the hyphen must not shift the rest.
        assert_eq!("/@", mount_infos[1].root);
        assert_eq!("/", mount_infos[1].point);
        assert_eq!("/dev/nvme0n1p4", mount_infos[1].source);
        assert_eq!("btrfs", mount_infos[1].fs_type);

        // This line has no optional field at all.
        assert_eq!(259, mount_infos[2].major);
        assert_eq!(1, mount_infos[2].minor);
        assert!(mount_infos[2].is_read_only());
        assert!(!mount_infos[1].is_read_only());

        assert_eq!("/media/user/my disk", mount_infos[3].point);

        // A file system without a block device is included too.
        assert_eq!("10.0.0.1:/export", mount_infos[4].source);
        assert_eq!("nfs4", mount_infos[4].fs_type);
    }

    #[test]
    fn parse_skipping_truncated_lines() {
        // A line without its fields must not cost the caller every other mount.
        const TRUNCATED: &[u8] = b"27 34 0:24 / /sys rw,nosuid shared:7 - sysfs sysfs rw
truncated
95 34 259:1 / /boot/efi ro,relatime - vfat /dev/nvme0n1p1 ro
120 34 0:59 / /mnt rw,relatime shared:2
";

        let mount_infos = parse_mount_infos(TRUNCATED).unwrap();

        assert_eq!(2, mount_infos.len());
        assert_eq!("/sys", mount_infos[0].point);

        // The line without the hyphen that ends the optional fields is skipped as well.
        assert_eq!("/boot/efi", mount_infos[1].point);
    }

    #[test]
    fn options() {
        let mount_infos = parse_mount_infos(MOUNT_INFO).unwrap();

        assert!(mount_infos[1].has_option("relatime"));
        assert!(mount_infos[1].has_option("subvol"));

        // `rw` must not be matched by the `rw,ssd` prefix of another option, and `no` must not match `nosuid`.
        assert!(!mount_infos[0].has_option("no"));
        assert!(!mount_infos[0].has_option("atime"));
    }
}
