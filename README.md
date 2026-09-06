M Prober Lib
====================

[![CI](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml)

This crate aims to quickly collect Linux system information including hostname, kernel version, uptime, RTC time, load average, CPU, memory, pressure, sensors, batteries, cgroup limits, network interfaces, sockets, routes, block devices, mounts and processes.

It reads the files provided by the kernel (`/proc` and `/sys`) or calls libc directly, so it needs no external tool or daemon and never starts a child process. Linux 5.10 or later is expected.

## Examples

```rust
use mprober_lib::*;

println!("{}", hostname::get_hostname().unwrap());
println!("{}", kernel::get_kernel_version().unwrap());
println!("{:#?}", uptime::get_uptime().unwrap());
println!("{:#?}", cpu::get_cpus().unwrap());
println!("{:#?}", memory::free().unwrap());
println!("{:#?}", network::get_networks().unwrap());
println!("{:#?}", volume::get_volumes().unwrap());
println!("{:#?}", process::get_process_stat(1).unwrap());
```

Every function is documented with an example of its own, and the documentation link below lists all of them.

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
