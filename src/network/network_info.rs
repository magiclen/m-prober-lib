use std::{
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// The link information of a network interface, read from the `/sys/class/net` folder.
#[derive(Default, Debug, Clone)]
pub struct NetworkInfo {
    /// The interface index, which the `ip link` command shows in front of the name.
    pub ifindex:     u32,
    /// The operational state, e.g. `up`, `down` or `unknown`.
    pub operstate:   String,
    /// Whether a physical link is detected. It is `None` when the interface is down, because the kernel does not report it then.
    pub carrier:     Option<bool>,
    /// The duplex mode, `full` or `half`. It is `None` when the link is down, the interface is virtual or the driver does not know.
    pub duplex:      Option<String>,
    /// The link speed in Mbps. It is `None` when the link is down or the interface is virtual.
    pub speed_mbps:  Option<u32>,
    /// The maximum transmission unit in bytes.
    pub mtu:         u32,
    /// The hardware (MAC) address, e.g. `60:cf:84:ac:5a:19`.
    pub mac_address: String,
    /// The interface flags (`IFF_*` in `netdevice(7)`), e.g. `0x1003` for an ethernet interface that is up. The kernel computes `IFF_RUNNING` and `IFF_LOWER_UP` on the fly, so they are never set here; use `operstate` or `carrier` instead.
    pub flags:       u32,
}

impl NetworkInfo {
    /// Whether the interface is enabled by the administrator (`IFF_UP`). Use `operstate` or `carrier` to know whether the link is actually up.
    #[inline]
    pub fn is_up(&self) -> bool {
        self.flags & (libc::IFF_UP as u32) != 0
    }

    /// Whether the interface is a loopback interface (`IFF_LOOPBACK`).
    #[inline]
    pub fn is_loopback(&self) -> bool {
        self.flags & (libc::IFF_LOOPBACK as u32) != 0
    }
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
pub fn get_network_info<S: AsRef<str>>(interface: S) -> Result<NetworkInfo, Error> {
    let interface = interface.as_ref();

    // The name must be a single path component.
    if interface.is_empty() || interface == "." || interface == ".." || interface.contains('/') {
        return Err(io::Error::from(ErrorKind::InvalidInput).into());
    }

    let path = Path::new("/sys/class/net").join(interface);

    let ifindex = read_sysfs_number(path.join("ifindex"))?;

    let operstate = read_sysfs_string(path.join("operstate"))?;

    // Reading `carrier`, `duplex` and `speed` fails with `EINVAL` when the interface is down, and `speed` is `-1` when the driver does not know it.
    let carrier = read_sysfs_number::<u8, _>(path.join("carrier")).ok().map(|carrier| carrier == 1);

    let duplex = read_sysfs_string(path.join("duplex")).ok().filter(|duplex| duplex != "unknown");

    let speed_mbps = read_sysfs_number::<i64, _>(path.join("speed"))
        .ok()
        .and_then(|speed| u32::try_from(speed).ok());

    let mtu = read_sysfs_number(path.join("mtu"))?;

    let mac_address = read_sysfs_string(path.join("address"))?;

    // The flags are written in hexadecimal, e.g. `0x1003`.
    let flags = {
        let flags = read_sysfs_string(path.join("flags"))?;

        u32::from_str_radix(flags.strip_prefix("0x").unwrap_or(&flags), 16)?
    };

    Ok(NetworkInfo {
        ifindex,
        operstate,
        carrier,
        duplex,
        speed_mbps,
        mtu,
        mac_address,
        flags,
    })
}
