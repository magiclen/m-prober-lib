use std::{
    collections::HashMap,
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

            // Paths like `/dev/mapper/*` or `/dev/disk/by-uuid/*` are symlinks, so they are resolved to the real device name.
            let device = match Path::new(device_path.as_ref()).canonicalize() {
                Ok(real_path) => match real_path.file_name() {
                    Some(file_name) => file_name.to_string_lossy().into_owned(),
                    None => device_path[5..].to_string(),
                },
                Err(_) => device_path[5..].to_string(),
            };

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
