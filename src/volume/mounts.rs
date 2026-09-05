use std::{
    collections::HashMap,
    fs,
    io::{self, ErrorKind},
    path::Path,
};

use crate::scanner_rust::{Scanner, ScannerError};

/// Get mounting points of all block devices by reading the `/proc/mounts` file. The keys are device names as they appear in `/proc/diskstats`.
///
/// ```rust
/// use mprober_lib::volume;
///
/// let mounts = volume::get_mounts().unwrap();
///
/// println!("{mounts:#?}");
/// ```
pub fn get_mounts() -> Result<HashMap<String, Vec<String>>, ScannerError> {
    let mut sc: Scanner<_, 1024> = Scanner::scan_path2("/proc/mounts")?;

    let mut mounts: HashMap<String, Vec<String>> = HashMap::with_capacity(1);

    while let Some(device_path) = sc.next_raw()? {
        if device_path.starts_with(b"/dev/") {
            let device_path = String::from_utf8_lossy(&device_path);

            let path = Path::new(device_path.as_ref());

            // Only symlinks like `/dev/mapper/*` or `/dev/disk/by-uuid/*` need to be resolved to the real device name, and checking that first is cheaper than always calling `realpath`.
            let is_symlink =
                fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink());

            let real_device_name = if is_symlink {
                path.canonicalize().ok().and_then(|real_path| {
                    real_path.file_name().map(|name| name.to_string_lossy().into_owned())
                })
            } else {
                None
            };

            let device = real_device_name.unwrap_or_else(|| device_path[5..].to_string());

            let point = String::from_utf8_lossy(
                &sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?,
            )
            .into_owned();

            mounts.entry(device).or_default().push(point);
        }

        sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    Ok(mounts)
}
