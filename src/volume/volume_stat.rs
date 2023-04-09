use std::time::Duration;

#[derive(Default, Debug, Clone)]
pub struct VolumeSpeed {
    pub read:  f64,
    pub write: f64,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct VolumeStat {
    pub read_bytes:  u64,
    pub write_bytes: u64,
}

impl VolumeStat {
    /// Calculate speed between two `VolumeStat` instances at different time.
    ///
    /// ```rust
    /// extern crate mprober_lib;
    ///
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::volume;
    ///
    /// let pre_volumes = volume::get_volumes().unwrap();
    ///
    /// let interval = Duration::from_millis(100);
    ///
    /// sleep(interval);
    ///
    /// let volumes = volume::get_volumes().unwrap();
    ///
    /// if !pre_volumes.is_empty() && !volumes.is_empty() {
    ///     let volume_speed =
    ///         pre_volumes[0].stat.compute_speed(&volumes[0].stat, interval);
    ///
    ///     println!("Read: {:.1} B/s", volume_speed.read);
    ///     println!("Write: {:.1} B/s", volume_speed.write);
    /// }
    /// ```
    #[inline]
    pub fn compute_speed(
        &self,
        volume_stat_after_this: &VolumeStat,
        interval: Duration,
    ) -> VolumeSpeed {
        let seconds = interval.as_secs_f64();
        let d_read = volume_stat_after_this.read_bytes - self.read_bytes;
        let d_write = volume_stat_after_this.write_bytes - self.write_bytes;

        let read = d_read as f64 / seconds;
        let write = d_write as f64 / seconds;

        VolumeSpeed {
            read,
            write,
        }
    }
}
