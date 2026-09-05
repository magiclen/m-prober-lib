/*!
# M Prober Lib

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, pressure, sensors, network interfaces, block devices and processes.

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
println!("{:#?}", memory::free().unwrap());
println!("{:#?}", memory::get_vm_stat().unwrap());
println!("{:#?}", pressure::get_cpu_pressure().unwrap());
println!("{:#?}", hwmon::get_hwmon_devices().unwrap());
println!("{:#?}", volume::get_volumes().unwrap());
println!("{:#?}", network::get_networks().unwrap());
println!("{:#?}", network::get_network_info("lo").unwrap());
println!("{:#?}", process::get_processes_with_stat(&process::ProcessFilter::default()).unwrap().into_iter().map(|(process, _)| process).collect::<Vec<process::Process>>());
println!("{:#?}", process::get_process_io(std::process::id()).unwrap());
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

mod functions;
mod utils;

pub mod btime;
pub mod cpu;
pub mod hostname;
pub mod hwmon;
pub mod kernel;
pub mod load_average;
pub mod memory;
pub mod network;
pub mod pressure;
pub mod process;
pub mod rtc_time;
pub mod uptime;
pub mod volume;

pub use functions::*;
pub use scanner_rust::ScannerError;
