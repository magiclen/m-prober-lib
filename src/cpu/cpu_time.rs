/// CPU times in `USER_HZ` clock ticks, summed from a `CPUStat`.
#[derive(Default, Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPUTime {
    /// `user + nice + system + irq + softirq + steal`
    pub non_idle: u64,
    /// `idle + iowait`
    pub idle:     u64,
}

impl CPUTime {
    /// Get the total CPU time.
    ///
    /// ```rust
    /// use mprober_lib::cpu;
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    /// let cpu_time = average_cpu_stat.compute_cpu_time();
    /// let total_cpu_time = cpu_time.get_total_time();
    ///
    /// println!("{total_cpu_time}");
    /// ```
    #[inline]
    pub fn get_total_time(self) -> u64 {
        self.idle + self.non_idle
    }
}
