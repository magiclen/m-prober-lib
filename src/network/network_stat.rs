use std::time::Duration;

/// The rates computed between two `NetworkStat` instances.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkSpeed {
    /// Bytes received per second.
    pub receive:          f64,
    /// Bytes transmitted per second.
    pub transmit:         f64,
    /// Packets received per second.
    pub receive_packets:  f64,
    /// Packets transmitted per second.
    pub transmit_packets: f64,
}

/// Counters read from the `/proc/net/dev` file.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkStat {
    /// Bytes received.
    pub receive_bytes:    u64,
    /// Packets received.
    pub receive_packets:  u64,
    /// Receive errors.
    pub receive_errors:   u64,
    /// Packets dropped while receiving.
    pub receive_dropped:  u64,
    /// Bytes transmitted.
    pub transmit_bytes:   u64,
    /// Packets transmitted.
    pub transmit_packets: u64,
    /// Transmit errors.
    pub transmit_errors:  u64,
    /// Packets dropped while transmitting.
    pub transmit_dropped: u64,
}

impl NetworkStat {
    /// Calculate speed between two `NetworkStat` instances at different time.
    ///
    /// ```rust
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::network;
    ///
    /// let pre_networks = network::get_networks().unwrap();
    ///
    /// let interval = Duration::from_millis(100);
    ///
    /// sleep(interval);
    ///
    /// let networks = network::get_networks().unwrap();
    ///
    /// if !pre_networks.is_empty() && !networks.is_empty() {
    ///     let network_speed =
    ///         pre_networks[0].stat.compute_speed(&networks[0].stat, interval);
    ///
    ///     println!("Receive: {:.1} B/s", network_speed.receive);
    ///     println!("Transmit: {:.1} B/s", network_speed.transmit);
    /// }
    /// ```
    #[inline]
    pub fn compute_speed(
        &self,
        network_stat_after_this: &NetworkStat,
        interval: Duration,
    ) -> NetworkSpeed {
        let seconds = interval.as_secs_f64();

        if seconds <= 0.0 {
            return NetworkSpeed::default();
        }

        let d_receive = network_stat_after_this.receive_bytes.saturating_sub(self.receive_bytes);
        let d_transmit = network_stat_after_this.transmit_bytes.saturating_sub(self.transmit_bytes);
        let d_receive_packets =
            network_stat_after_this.receive_packets.saturating_sub(self.receive_packets);
        let d_transmit_packets =
            network_stat_after_this.transmit_packets.saturating_sub(self.transmit_packets);

        NetworkSpeed {
            receive:          d_receive as f64 / seconds,
            transmit:         d_transmit as f64 / seconds,
            receive_packets:  d_receive_packets as f64 / seconds,
            transmit_packets: d_transmit_packets as f64 / seconds,
        }
    }
}
