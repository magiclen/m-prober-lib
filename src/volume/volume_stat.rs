use std::time::Duration;

/// The rates computed between two `VolumeStat` instances.
#[derive(Default, Debug, Clone)]
pub struct VolumeSpeed {
    /// Bytes read per second.
    pub read:        f64,
    /// Bytes written per second.
    pub write:       f64,
    /// Read operations completed per second.
    pub read_iops:   f64,
    /// Write operations completed per second.
    pub write_iops:  f64,
    /// The fraction of time the device was busy doing I/O, like the `%util` column of `iostat`. If it is `1.0`, means `100%`.
    pub utilization: f64,
}

/// Counters read from the `/proc/diskstats` file.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct VolumeStat {
    /// Read operations completed successfully.
    pub reads_completed:  u64,
    /// Bytes read (the sector count in `/proc/diskstats` multiplied by 512).
    pub read_bytes:       u64,
    /// Time spent reading.
    pub read_time:        Duration,
    /// Write operations completed successfully.
    pub writes_completed: u64,
    /// Bytes written (the sector count in `/proc/diskstats` multiplied by 512).
    pub write_bytes:      u64,
    /// Time spent writing.
    pub write_time:       Duration,
    /// I/O operations currently in progress.
    pub io_in_progress:   u64,
    /// Time spent doing I/O.
    pub io_time:          Duration,
}

impl VolumeStat {
    /// Calculate speed between two `VolumeStat` instances at different time.
    ///
    /// ```rust
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
    ///     println!("Utilization: {:.1}%", volume_speed.utilization * 100.0);
    /// }
    /// ```
    #[inline]
    pub fn compute_speed(
        &self,
        volume_stat_after_this: &VolumeStat,
        interval: Duration,
    ) -> VolumeSpeed {
        let seconds = interval.as_secs_f64();

        if seconds <= 0.0 {
            return VolumeSpeed::default();
        }

        let d_read = volume_stat_after_this.read_bytes.saturating_sub(self.read_bytes);
        let d_write = volume_stat_after_this.write_bytes.saturating_sub(self.write_bytes);
        let d_reads = volume_stat_after_this.reads_completed.saturating_sub(self.reads_completed);
        let d_writes =
            volume_stat_after_this.writes_completed.saturating_sub(self.writes_completed);
        let d_io_time = volume_stat_after_this.io_time.saturating_sub(self.io_time);

        VolumeSpeed {
            read:        d_read as f64 / seconds,
            write:       d_write as f64 / seconds,
            read_iops:   d_reads as f64 / seconds,
            write_iops:  d_writes as f64 / seconds,
            utilization: (d_io_time.as_secs_f64() / seconds).min(1.0),
        }
    }
}
