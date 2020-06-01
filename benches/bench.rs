extern crate mprober_lib;

#[macro_use]
extern crate bencher;

use bencher::Bencher;

use mprober_lib::*;

fn get_btime(bencher: &mut Bencher) {
    bencher.iter(|| btime::get_btime());
}

fn get_cpus(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_cpus().unwrap());
}

fn get_average_cpu_stat(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_average_cpu_stat().unwrap());
}

fn get_all_cpus_stat_with_average(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_all_cpus_stat(true).unwrap());
}

fn get_all_cpus_stat_without_average(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_all_cpus_stat(false).unwrap());
}

fn get_hostname(bencher: &mut Bencher) {
    bencher.iter(|| hostname::get_hostname().unwrap());
}

fn get_kernel_version(bencher: &mut Bencher) {
    bencher.iter(|| kernel::get_kernel_version().unwrap());
}

fn get_load_average(bencher: &mut Bencher) {
    bencher.iter(|| load_average::get_load_average().unwrap());
}

fn free(bencher: &mut Bencher) {
    bencher.iter(|| memory::free().unwrap());
}

fn get_networks(bencher: &mut Bencher) {
    bencher.iter(|| network::get_networks().unwrap());
}

fn get_process_status(bencher: &mut Bencher) {
    bencher.iter(|| process::get_process_status(1).unwrap());
}

fn get_process_time_stat(bencher: &mut Bencher) {
    bencher.iter(|| process::get_process_time_stat(1).unwrap());
}

fn get_process_stat(bencher: &mut Bencher) {
    bencher.iter(|| process::get_process_stat(1).unwrap());
}

fn get_process_with_stat(bencher: &mut Bencher) {
    bencher.iter(|| process::get_process_with_stat(1).unwrap());
}

fn get_rtc_date_time(bencher: &mut Bencher) {
    bencher.iter(|| rtc_time::get_rtc_date_time().unwrap());
}

fn get_uptime(bencher: &mut Bencher) {
    bencher.iter(|| uptime::get_uptime().unwrap());
}

benchmark_group!(btime, get_btime);

benchmark_group!(
    cpu,
    get_cpus,
    get_average_cpu_stat,
    get_all_cpus_stat_with_average,
    get_all_cpus_stat_without_average
);
benchmark_group!(hostname, get_hostname);
benchmark_group!(kernel, get_kernel_version);
benchmark_group!(load_average, get_load_average);
benchmark_group!(memory, free);
benchmark_group!(network, get_networks);
benchmark_group!(
    process,
    get_process_status,
    get_process_time_stat,
    get_process_stat,
    get_process_with_stat
);
benchmark_group!(rtc_time, get_rtc_date_time);
benchmark_group!(uptime, get_uptime);

benchmark_main!(
    btime,
    cpu,
    hostname,
    kernel,
    load_average,
    memory,
    network,
    process,
    rtc_time,
    uptime
);
