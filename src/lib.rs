/*!
# M Prober Lib

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, pressure, sensors, batteries, cgroup limits, network interfaces, block devices and processes.

It reads the files provided by the kernel (`/proc` and `/sys`) or calls libc directly, so it has no extra runtime dependencies. Linux 5.10 or later is expected.

## Examples

Some of these depend on optional kernel features, so this example is not run as a test.

```rust,no_run
use mprober_lib::*;

println!("{}", hostname::get_hostname().unwrap());
println!("{}", kernel::get_kernel_version().unwrap());
println!("{:#?}", kernel::get_uname().unwrap());
println!("{:#?}", kernel::get_file_nr().unwrap());
println!("{}", btime::get_btime());
println!("{}", rtc_time::get_rtc_date_time().unwrap());
println!("{:#?}", uptime::get_uptime().unwrap());
println!("{:#?}", load_average::get_load_average().unwrap());
println!("{:#?}", cpu::get_cpus().unwrap());
println!("{:#?}", cpu::get_cpu_activity().unwrap());
println!("{}", cpu::get_online_cpu_count().unwrap());
println!("{:#?}", cpu::get_cpu_frequency(0).unwrap());
println!("{:#?}", memory::free().unwrap());
println!("{:#?}", memory::get_vm_stat().unwrap());
println!("{:#?}", pressure::get_cpu_pressure().unwrap());
println!("{:#?}", hwmon::get_hwmon_devices().unwrap());
println!("{:#?}", power_supply::get_power_supplies().unwrap());
println!("{:#?}", cgroup::get_cgroup_cpu(cgroup::get_cgroup_path().unwrap()).unwrap());
println!("{:#?}", volume::get_volumes().unwrap());
println!("{:#?}", network::get_networks().unwrap());
println!("{:#?}", network::get_network_info("lo").unwrap());
println!("{:#?}", network::get_network_addresses().unwrap());
println!("{:#?}", network::get_socket_stat().unwrap());
println!("{:#?}", process::get_processes_with_stat(&process::ProcessFilter::default()).unwrap().into_iter().map(|(process, _)| process).collect::<Vec<process::Process>>());
println!("{:#?}", process::get_process_io(std::process::id()).unwrap());
println!("{}", process::get_process_fd_count(std::process::id()).unwrap());
```

## Benchmark

The benchmarks are not part of the published package, so they have to be run from a clone of the repository.

```bash
cargo bench
```
*/

#[cfg(not(target_os = "linux"))]
compile_error!("mprober-lib reads the `/proc` and `/sys` file systems, so it only supports Linux.");

pub extern crate scanner_rust;

mod error;
mod functions;
mod utils;

/// The boot time of the system.
pub mod btime;
/// The resource usage and limits of a cgroup (v2), e.g. the limits of the container the process runs in.
pub mod cgroup;
/// CPU models, frequencies, per-CPU time counters and system-wide scheduler counters.
pub mod cpu;
/// The hostname of the system.
pub mod hostname;
/// Temperature and fan sensors, like the `sensors` command.
pub mod hwmon;
/// The kernel version, the `uname` fields and the file handle usage.
pub mod kernel;
/// The load average.
pub mod load_average;
/// Memory and swap usage, like the `free` command, and the paging counters of the `vmstat` command.
pub mod memory;
/// Network interfaces, their counters, their link information, their IP addresses and the socket usage.
pub mod network;
/// Batteries and power adapters.
pub mod power_supply;
/// PSI (Pressure Stall Information) for CPU, memory and I/O.
pub mod pressure;
/// Running processes, their stats and their I/O counters.
pub mod process;
/// The datetime of the hardware real time clock.
pub mod rtc_time;
/// The identity of the machine, and whether it is virtualized.
pub mod system;
/// The time since the system booted.
pub mod uptime;
/// Mounted block devices, their sizes and their I/O counters.
pub mod volume;

pub use error::*;
pub use functions::*;
/// The error type this crate used before v0.2. It is kept for migration; every function now returns [`Error`].
pub use scanner_rust::ScannerError;
