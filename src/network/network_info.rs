use std::{
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    scanner_rust::ScannerError,
    utils::{read_sysfs_number, read_sysfs_string},
};

#[derive(Default, Debug, Clone)]
pub struct NetworkInfo {
    /// The operational state, e.g. `up`, `down` or `unknown`.
    pub operstate:   String,
    /// The link speed in Mbps. It is `None` when the link is down or the interface is virtual.
    pub speed_mbps:  Option<u32>,
    /// The maximum transmission unit in bytes.
    pub mtu:         u32,
    /// The hardware (MAC) address, e.g. `60:cf:84:ac:5a:19`.
    pub mac_address: String,
}

/// Get the information of a network interface by reading files in the `/sys/class/net/INTERFACE` folder.
///
/// ```rust
/// use mprober_lib::network;
///
/// let network_info = network::get_network_info("lo").unwrap();
///
/// println!("{network_info:#?}");
/// ```
pub fn get_network_info<S: AsRef<str>>(interface: S) -> Result<NetworkInfo, ScannerError> {
    let interface = interface.as_ref();

    // The name must be a single path component.
    if interface.is_empty() || interface == "." || interface == ".." || interface.contains('/') {
        return Err(io::Error::from(ErrorKind::InvalidInput).into());
    }

    let path = Path::new("/sys/class/net").join(interface);

    let operstate = read_sysfs_string(path.join("operstate"))?;

    // Reading `speed` fails for virtual interfaces and gives `-1` when the link is down.
    let speed_mbps = read_sysfs_number::<i64, _>(path.join("speed"))
        .ok()
        .and_then(|speed| u32::try_from(speed).ok());

    let mtu = read_sysfs_number(path.join("mtu"))?;

    let mac_address = read_sysfs_string(path.join("address"))?;

    Ok(NetworkInfo {
        operstate,
        speed_mbps,
        mtu,
        mac_address,
    })
}
