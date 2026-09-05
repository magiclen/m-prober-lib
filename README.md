M Prober Lib
====================

[![CI](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml)

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, pressure, sensors, batteries, cgroup limits, network interfaces, sockets, routes, block devices, mounts and processes.

It reads the files provided by the kernel (`/proc` and `/sys`) or calls libc directly, so it has no extra runtime dependencies. Linux 5.10 or later is expected.

## Examples

Some of these depend on optional kernel features, so they can fail on a system that does not provide them.

```rust
use mprober_lib::*;

// System
println!("{}", hostname::get_hostname().unwrap());
println!("{:#?}", system::get_dmi_info().unwrap());
println!("{}", system::get_machine_id().unwrap());
println!("{}", system::get_boot_id().unwrap());

// Kernel
println!("{}", kernel::get_kernel_version().unwrap());
println!("{:#?}", kernel::get_uname().unwrap());
println!("{:#?}", kernel::get_file_nr().unwrap());
println!("{}", kernel::get_pid_max().unwrap());
println!("{}", kernel::get_threads_max().unwrap());
println!("{:?}", kernel::get_kernel_cmdline().unwrap());
println!("{:?}", kernel::get_kernel_taint_reasons(kernel::get_kernel_taint().unwrap()));

// Time
println!("{}", btime::get_btime());
println!("{}", rtc_time::get_rtc_date_time().unwrap());
println!("{:#?}", uptime::get_uptime().unwrap());
println!("{:#?}", load_average::get_load_average().unwrap());

// CPU
println!("{:#?}", cpu::get_cpus().unwrap());
println!("{:#?}", cpu::get_cpu_activity().unwrap());
println!("{}", cpu::get_online_cpu_count().unwrap());
println!("{}", cpu::get_available_cpu_count().unwrap());
println!("{:?}", cpu::get_online_cpus().unwrap());
println!("{:#?}", cpu::get_all_cpu_topologies().unwrap());
println!("{:#?}", cpu::get_cpu_frequency(0).unwrap());
println!("{:#?}", cpu::get_all_cpu_frequencies().unwrap());
println!("{:#?}", cpu::get_cpu_thermal_throttle(0).unwrap());

// Memory
println!("{:#?}", memory::free().unwrap());
println!("{:#?}", memory::get_mem_info().unwrap());
println!("{:#?}", memory::get_vm_stat().unwrap());
println!("{:#?}", memory::get_swaps().unwrap());
println!("{:#?}", memory::get_numa_nodes().unwrap());

// Pressure and sensors
println!("{:#?}", pressure::get_cpu_pressure().unwrap());
println!("{:#?}", hwmon::get_hwmon_devices().unwrap());
println!("{:#?}", power_supply::get_power_supplies().unwrap());

// cgroup
let cgroup_path = cgroup::get_cgroup_path().unwrap();
println!("{:#?}", cgroup::get_cgroup_cpu(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_memory(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_memory_stat(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_memory_events(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_pids(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_io(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_cpuset(&cgroup_path).unwrap());
println!("{:#?}", cgroup::get_cgroup_pressure(&cgroup_path, "cpu").unwrap());

// Storage
println!("{:#?}", volume::get_volumes().unwrap());
println!("{:#?}", volume::get_disk_stats().unwrap());
println!("{:#?}", volume::get_block_devices().unwrap());
println!("{:#?}", volume::get_mount_infos().unwrap());

// Network
println!("{:#?}", network::get_networks().unwrap());
println!("{:#?}", network::get_network_info("lo").unwrap());
println!("{:#?}", network::get_network_addresses().unwrap());
println!("{:#?}", network::get_socket_stat().unwrap());
println!("{:#?}", network::get_protocol_stat().unwrap());
println!("{:#?}", network::get_all_socket_connections().unwrap());
println!("{:#?}", network::get_default_routes().unwrap());

// Processes
let pid = std::process::id();
println!("{:#?}", process::get_processes_with_stat(&process::ProcessFilter::default()).unwrap().into_iter().map(|(process, _)| process).collect::<Vec<process::Process>>());
println!("{:#?}", process::get_process_io(pid).unwrap());
println!("{}", process::get_process_fd_count(pid).unwrap());
println!("{:#?}", process::get_process_memory(pid).unwrap());
println!("{:#?}", process::get_process_threads(pid).unwrap());
println!("{:#?}", process::get_process_oom(pid).unwrap());
println!("{:#?}", process::get_process_limits(pid).unwrap());
println!("{}", process::get_process_cwd(pid).unwrap().display());
println!("{}", cgroup::get_process_cgroup_path(pid).unwrap().display());
```

## Optional Features

* `serde`: derive `Serialize` and `Deserialize` for every data type this crate returns.

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
