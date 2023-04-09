#[derive(Default, Debug, Clone, Copy)]
pub struct CPUTime {
    pub non_idle: u64,
    pub idle:     u64,
}

impl CPUTime {
    /// Get the total CPU time.
    ///
    /// ```rust
    /// extern crate mprober_lib;
    ///
    /// use mprober_lib::cpu;
    ///
    /// let average_cpu_stat = cpu::get_average_cpu_stat().unwrap();
    /// let cpu_time = average_cpu_stat.compute_cpu_time();
    /// let total_cpu_time = cpu_time.get_total_time();
    ///
    /// println!("{}", total_cpu_time);
    /// ```
    #[inline]
    pub fn get_total_time(self) -> u64 {
        self.idle + self.non_idle
    }
}
