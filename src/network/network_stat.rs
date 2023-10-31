use std::time::Duration;

#[derive(Default, Debug, Clone)]
pub struct NetworkSpeed {
    pub receive:  f64,
    pub transmit: f64,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct NetworkStat {
    pub receive_bytes:  u64,
    pub transmit_bytes: u64,
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
        let d_receive = network_stat_after_this.receive_bytes - self.receive_bytes;
        let d_transmit = network_stat_after_this.transmit_bytes - self.transmit_bytes;

        let receive = d_receive as f64 / seconds;
        let transmit = d_transmit as f64 / seconds;

        NetworkSpeed {
            receive,
            transmit,
        }
    }
}
