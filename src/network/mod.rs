mod network_address;
mod network_info;
mod network_stat;
mod protocol_stat;
mod route;
mod socket_connection;
mod socket_stat;

use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    thread::sleep,
    time::Duration,
};

pub use network_address::*;
pub use network_info::*;
pub use network_stat::*;
pub use protocol_stat::*;
pub use route::*;
use scanner_rust::ScannerU8SliceAscii;
pub use socket_connection::*;
pub use socket_stat::*;

use crate::{
    Error,
    utils::{OrEof, read_file},
};

/// One network interface and its counters. Two instances are equal when their interface names are equal.
#[derive(Default, Debug, Clone, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Parse the content of `/proc/net/dev`, whose first two lines only name the columns.
fn parse_networks(data: &[u8]) -> Result<Vec<Network>, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    for _ in 0..2 {
        sc.drop_next_line()?.or_eof()?;
    }

    let mut networks = Vec::with_capacity(8);

    loop {
        // Interface names are right-aligned in this file, so the padding must be skipped before reading up to the colon.
        if !sc.skip_whitespaces()? {
            break;
        }

        let Some(interface) = sc.next_until(":")? else {
            break;
        };

        // The kernel only rejects `/`, `:` and whitespace in an interface name, so it may not be valid UTF-8.
        let interface = String::from_utf8_lossy(interface).into_owned();

        let receive_bytes = sc.next_u64()?.or_eof()?;
        let receive_packets = sc.next_u64()?.or_eof()?;
        let receive_errors = sc.next_u64()?.or_eof()?;
        let receive_dropped = sc.next_u64()?.or_eof()?;

        // fifo, frame, compressed, multicast
        for _ in 0..4 {
            sc.drop_next()?.or_eof()?;
        }

        let transmit_bytes = sc.next_u64()?.or_eof()?;
        let transmit_packets = sc.next_u64()?.or_eof()?;
        let transmit_errors = sc.next_u64()?.or_eof()?;
        let transmit_dropped = sc.next_u64()?.or_eof()?;

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

        // A `None` here only means the file ended without a trailing newline, which the loop condition handles.
        sc.drop_next_line()?;
    }

    Ok(networks)
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
#[inline]
pub fn get_networks() -> Result<Vec<Network>, Error> {
    parse_networks(&read_file("/proc/net/dev", 4096)?)
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

#[cfg(test)]
mod tests {
    use super::*;

    const NET_DEV: &[u8] = b"Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo:  771658    6454    0    0    0     0          0         0   771658    6454    0    0    0     0       0          0
enp131s0: 337732427  462493    1    2    0     0          0      3765 247091393  344755    3    4    0     0       0          0
";

    #[test]
    fn parse() {
        let networks = parse_networks(NET_DEV).unwrap();

        assert_eq!(2, networks.len());

        assert_eq!("lo", networks[0].interface);
        assert_eq!(771658, networks[0].stat.receive_bytes);
        assert_eq!(6454, networks[0].stat.receive_packets);
        assert_eq!(771658, networks[0].stat.transmit_bytes);

        // A name of six characters or more leaves no padding in front of it, so the colon is what ends it.
        assert_eq!("enp131s0", networks[1].interface);
        assert_eq!(337732427, networks[1].stat.receive_bytes);
        assert_eq!(1, networks[1].stat.receive_errors);
        assert_eq!(2, networks[1].stat.receive_dropped);
        assert_eq!(247091393, networks[1].stat.transmit_bytes);
        assert_eq!(344755, networks[1].stat.transmit_packets);
        assert_eq!(3, networks[1].stat.transmit_errors);
        assert_eq!(4, networks[1].stat.transmit_dropped);
    }
}
