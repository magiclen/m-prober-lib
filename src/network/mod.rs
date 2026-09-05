mod network_info;
mod network_stat;

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    io::{self, ErrorKind},
    thread::sleep,
    time::Duration,
};

pub use network_info::*;
pub use network_stat::*;

use crate::{Error, scanner_rust::ScannerAscii};

/// One network interface and its counters. Two instances are equal when their interface names are equal.
#[derive(Default, Debug, Clone, Eq)]
pub struct Network {
    /// The name of the interface, e.g. `lo` or `eth0`.
    pub interface: String,
    /// The counters of the interface.
    pub stat:      NetworkStat,
}

impl Hash for Network {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.interface.hash(state)
    }
}

impl PartialEq for Network {
    #[inline]
    fn eq(&self, other: &Network) -> bool {
        self.interface.eq(&other.interface)
    }
}

/// Get network information by reading the `/proc/net/dev` file.
///
/// ```rust
/// use mprober_lib::network;
///
/// let networks = network::get_networks().unwrap();
///
/// println!("{networks:#?}");
/// ```
pub fn get_networks() -> Result<Vec<Network>, Error> {
    let mut sc: ScannerAscii<_, 1024> = ScannerAscii::scan_path2("/proc/net/dev")?;

    for _ in 0..2 {
        sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    let mut networks = Vec::with_capacity(1);

    loop {
        // Interface names are right-aligned in this file, so the padding must be skipped before reading up to the colon.
        if !sc.skip_whitespaces()? {
            break;
        }

        let Some(interface) = sc.next_until_raw(":")? else {
            break;
        };

        // The kernel only rejects `/`, `:` and whitespace in an interface name, so it may not be valid UTF-8.
        let interface = String::from_utf8_lossy(&interface).into_owned();

        let receive_bytes = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        let receive_packets = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        let receive_errors = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        let receive_dropped = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        // fifo, frame, compressed, multicast
        for _ in 0..4 {
            sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        }

        let transmit_bytes = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        let transmit_packets = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        let transmit_errors = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
        let transmit_dropped = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        let stat = NetworkStat {
            receive_bytes,
            receive_packets,
            receive_errors,
            receive_dropped,
            transmit_bytes,
            transmit_packets,
            transmit_errors,
            transmit_dropped,
        };

        let network = Network {
            interface,
            stat,
        };

        networks.push(network);

        sc.drop_next_line()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    Ok(networks)
}

/// Get network information by reading the `/proc/net/dev` file and measure the speed within a specific time interval.
///
/// ```rust
/// use std::time::Duration;
///
/// use mprober_lib::network;
///
/// let networks_with_speed =
///     network::get_networks_with_speed(Duration::from_millis(100)).unwrap();
///
/// for (network, network_speed) in networks_with_speed {
///     println!("{}: ", network.interface);
///     println!("    Receive: {:.1} B/s", network_speed.receive);
///     println!("    Transmit: {:.1} B/s", network_speed.transmit);
/// }
/// ```
pub fn get_networks_with_speed(interval: Duration) -> Result<Vec<(Network, NetworkSpeed)>, Error> {
    let pre_networks = get_networks()?;

    let pre_networks_length = pre_networks.len();

    let mut pre_networks_hashset = HashSet::with_capacity(pre_networks_length);

    for pre_network in pre_networks {
        pre_networks_hashset.insert(pre_network);
    }

    sleep(interval);

    let networks = get_networks()?;

    let mut networks_with_speed = Vec::with_capacity(networks.len().min(pre_networks_length));

    for network in networks {
        if let Some(pre_network) = pre_networks_hashset.get(&network) {
            let network_speed = pre_network.stat.compute_speed(&network.stat, interval);

            networks_with_speed.push((network, network_speed));
        }
    }

    Ok(networks_with_speed)
}
