use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// The link information of a network interface, read from the `/sys/class/net` folder.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkInfo {
    /// The interface index, which the `ip link` command shows in front of the name.
    pub ifindex:       u32,
    /// The operational state, e.g. `up`, `down` or `unknown`.
    pub operstate:     String,
    /// Whether a physical link is detected. It is `None` when the interface is down, because the kernel does not report it then.
    pub carrier:       Option<bool>,
    /// The duplex mode, `full` or `half`. It is `None` when the link is down, the interface is virtual or the driver does not know.
    pub duplex:        Option<String>,
    /// The link speed in Mbps. It is `None` when the link is down or the interface is virtual.
    pub speed_mbps:    Option<u32>,
    /// The maximum transmission unit in bytes.
    pub mtu:           u32,
    /// The hardware (MAC) address, e.g. `60:cf:84:ac:5a:19`.
    pub mac_address:   String,
    /// The interface flags (`IFF_*` in `netdevice(7)`), e.g. `0x1003` for an ethernet interface that is up. The kernel computes `IFF_RUNNING` and `IFF_LOWER_UP` on the fly, so they are never set here; use `operstate` or `carrier` instead.
    pub flags:         u32,
    /// The name of the driver that owns the interface, e.g. `r8169` or `iwlwifi`. It is `None` for a virtual interface that has no driver.
    pub driver:        Option<String>,
    /// The length of the transmit queue in packets.
    pub tx_queue_len:  u32,
    /// The number of receive queues the interface has. More than one means the driver can spread the load over several CPUs.
    pub num_rx_queues: u32,
    /// The number of transmit queues the interface has.
    pub num_tx_queues: u32,
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

/// Count the `rx-N` and `tx-N` folders the kernel creates for the queues of an interface.
fn count_queues(path: &Path) -> (u32, u32) {
    let Ok(read_dir) = fs::read_dir(path) else {
        return (0, 0);
    };

    let mut rx = 0;
    let mut tx = 0;

    for entry in read_dir.flatten() {
        let file_name = entry.file_name();

        let file_name = file_name.as_encoded_bytes();

        if file_name.starts_with(b"rx-") {
            rx += 1;
        } else if file_name.starts_with(b"tx-") {
            tx += 1;
        }
    }

    (rx, tx)
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

    // The kernel links every interface that has a driver to the folder of that driver.
    let driver = fs::read_link(path.join("device/driver"))
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()));

    let tx_queue_len = read_sysfs_number(path.join("tx_queue_len")).unwrap_or(0);

    let (num_rx_queues, num_tx_queues) = count_queues(&path.join("queues"));

    Ok(NetworkInfo {
        ifindex,
        operstate,
        carrier,
        duplex,
        speed_mbps,
        mtu,
        mac_address,
        flags,
        driver,
        tx_queue_len,
        num_rx_queues,
        num_tx_queues,
    })
}
