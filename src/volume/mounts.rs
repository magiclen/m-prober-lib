use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::Path;
use std::str::from_utf8_unchecked;

use crate::scanner_rust::generic_array::typenum::U1024;
use crate::scanner_rust::{Scanner, ScannerError};

/// Get mounting points of all block devices by reading the `/proc/mounts` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::volume;
///
/// let mounts = volume::get_mounts().unwrap();
///
/// println!("{:#?}", mounts);
/// ```
pub fn get_mounts() -> Result<HashMap<String, Vec<String>>, ScannerError> {
    let mut sc: Scanner<_, U1024> = Scanner::scan_path2("/proc/mounts")?;

    let mut mounts: HashMap<String, Vec<String>> = HashMap::with_capacity(1);

    while let Some(device_path) = sc.next_raw()? {
        if device_path.starts_with(b"/dev/") {
            let device = {
                let device = &device_path[5..];

                if device.starts_with(b"mapper/") {
                    let device_path =
                        Path::new(unsafe { from_utf8_unchecked(device_path.as_ref()) })
                            .canonicalize()?;

                    device_path.file_name().unwrap().to_string_lossy().into_owned()
                } else {
                    unsafe { from_utf8_unchecked(device) }.to_string()
                }
            };

            let point = unsafe {
                String::from_utf8_unchecked(sc.next_raw()?.ok_or(ErrorKind::UnexpectedEof)?)
            };

            match mounts.get_mut(&device) {
                Some(devices) => {
                    devices.push(point);
                }
                None => {
                    mounts.insert(device, vec![point]);
                }
            }
        }

        sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
    }

    Ok(mounts)
}
