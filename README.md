M Prober Lib
====================

[![CI](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml)

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, pressure, sensors, batteries, cgroup limits, network interfaces, block devices and processes.

It reads the files provided by the kernel (`/proc` and `/sys`) or calls libc directly, so it has no extra runtime dependencies. Linux 5.10 or later is expected.

## Examples

```rust
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

## Documentation

https://docs.rs/mprober-lib

## Official CLI

https://crates.io/crates/mprober

## License

[MIT](LICENSE)
