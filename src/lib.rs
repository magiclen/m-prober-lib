/*!
# M Prober Lib

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, network interfaces, block devices and processes.

## Examples

```rust
use mprober_lib::*;

println!("{}", hostname::get_hostname().unwrap());
println!("{}", kernel::get_kernel_version().unwrap());
println!("{}", btime::get_btime());
println!("{}", rtc_time::get_rtc_date_time().unwrap());
println!("{:#?}", uptime::get_uptime().unwrap());
println!("{:#?}", load_average::get_load_average().unwrap());
println!("{:#?}", cpu::get_cpus().unwrap());
println!("{:#?}", memory::free().unwrap());
println!("{:#?}", volume::get_volumes().unwrap());
println!("{:#?}", network::get_networks().unwrap());
println!("{:#?}", process::get_processes_with_stat(&process::ProcessFilter::default()).unwrap().into_iter().map(|(process, _)| process).collect::<Vec<process::Process>>());
```

## Benchmark

```bash
cargo bench
```
*/

#[cfg(not(target_os = "linux"))]
compile_error!("mprober-lib reads the `/proc` and `/sys` file systems, so it only supports Linux.");

mod error;
mod functions;
mod utils;

/// The boot time of the system.
pub mod btime;
/// The resource usage, limits and events of a cgroup (v2), e.g. the limits of the container the process runs in.
pub mod cgroup;
/// CPU models, topology, frequencies, per-CPU time counters and system-wide scheduler counters.
pub mod cpu;
/// The hostname of the system.
pub mod hostname;
/// Temperature, fan, voltage, power, current and humidity sensors, like the `sensors` command.
pub mod hwmon;
/// The kernel version, the `uname` fields, the file handle usage, the boot parameters and the taint flags.
pub mod kernel;
/// The load average.
pub mod load_average;
/// Memory and swap usage, like the `free` command, the paging counters of the `vmstat` command, the swap areas and the NUMA nodes.
pub mod memory;
/// Network interfaces, their counters, their link information, their IP addresses, the protocol counters, the sockets and the routing table.
pub mod network;
/// Batteries and power adapters.
pub mod power_supply;
/// PSI (Pressure Stall Information) for CPU, memory and I/O.
pub mod pressure;
/// Running processes, their stats, their memory, their threads and their I/O counters.
pub mod process;
/// The datetime of the hardware real time clock.
pub mod rtc_time;
/// The identity of the machine as the firmware and the kernel report it.
pub mod system;
/// The time since the system booted.
pub mod uptime;
/// Block devices, their attributes, their mounts, their sizes and their I/O counters.
pub mod volume;

pub use error::*;
pub use functions::*;
