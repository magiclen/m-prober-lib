/*!
# M Prober Lib

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, network interfaces, block devices and processes.

## Examples

```rust
extern crate mprober_lib;

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

extern crate libc;

pub extern crate scanner_rust;

mod functions;

pub mod btime;
pub mod cpu;
pub mod hostname;
pub mod kernel;
pub mod load_average;
pub mod memory;
pub mod network;
pub mod process;
pub mod rtc_time;
pub mod uptime;
pub mod volume;

pub use functions::*;

pub use scanner_rust::ScannerError;
