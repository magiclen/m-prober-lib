M Prober Lib
====================

[![CI](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/m-prober-lib/actions/workflows/ci.yml)

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

## Documentation

https://docs.rs/mprober-lib

## Official CLI

https://crates.io/crates/mprober

## License

[MIT](LICENSE)