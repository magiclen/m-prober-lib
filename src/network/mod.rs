mod network_stat;

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::thread::sleep;
use std::time::Duration;

use crate::scanner_rust::generic_array::typenum::U1024;
use crate::scanner_rust::{ScannerAscii, ScannerError};

pub use network_stat::*;

#[derive(Default, Debug, Clone, Eq)]
pub struct Network {
    pub interface: String,
    pub stat: NetworkStat,
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
/// extern crate mprober_lib;
///
/// use mprober_lib::network;
///
/// let networks = network::get_networks().unwrap();
///
/// println!("{:#?}", networks);
/// ```
pub fn get_networks() -> Result<Vec<Network>, ScannerError> {
    let mut sc: ScannerAscii<_, U1024> = ScannerAscii::scan_path2("/proc/net/dev")?;

    for _ in 0..2 {
        sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
    }

    let mut networks = Vec::with_capacity(1);

    while let Some(interface) = sc.next_until_raw(":")? {
        let interface = unsafe { String::from_utf8_unchecked(interface) };

        let receive_bytes = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

        for _ in 0..7 {
            sc.drop_next()?.ok_or(ErrorKind::UnexpectedEof)?;
        }

        let transmit_bytes = sc.next_u64()?.ok_or(ErrorKind::UnexpectedEof)?;

        let stat = NetworkStat {
            receive_bytes,
            transmit_bytes,
        };

        let network = Network {
            interface,
            stat,
        };

        networks.push(network);

        sc.drop_next_line()?.ok_or(ErrorKind::UnexpectedEof)?;
    }

    Ok(networks)
}

/// Get network information by reading the `/proc/net/dev` file and measure the speed within a specific time interval.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use std::time::Duration;
///
/// use mprober_lib::network;
///
/// let networks_with_speed = network::get_networks_with_speed(Duration::from_millis(100)).unwrap();
///
/// for (network, network_speed) in networks_with_speed {
///     println!("{}: ", network.interface);
///     println!("    Receive: {:.1} B/s", network_speed.receive);
///     println!("    Transmit: {:.1} B/s", network_speed.transmit);
/// }
/// ```
pub fn get_networks_with_speed(
    interval: Duration,
) -> Result<Vec<(Network, NetworkSpeed)>, ScannerError> {
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
