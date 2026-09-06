use std::sync::OnceLock;

mod cpu_activity;
mod cpu_count;
mod cpu_frequency;
mod cpu_info;
mod cpu_stat;
mod cpu_time;
mod cpu_topology;

pub use cpu_activity::*;
pub use cpu_count::*;
pub use cpu_frequency::*;
pub use cpu_info::*;
pub use cpu_stat::*;
pub use cpu_time::*;
pub use cpu_topology::*;

/// A capacity hint for the whole `/proc/stat` file, whose size grows with the number of processors. The count is read only once, because this is a hint and not a limit: a processor coming online later only makes the buffer grow.
pub(crate) fn proc_stat_capacity() -> usize {
    static CAPACITY: OnceLock<usize> = OnceLock::new();

    // The kernel sizes its own buffer in `stat_open` the same way, with 128 bytes per processor plus two bytes per interrupt counter, which the constant covers here.
    *CAPACITY.get_or_init(|| 4096 + 128 * (get_online_cpu_count().unwrap_or(0) + 1))
}
