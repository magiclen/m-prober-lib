extern crate mprober_lib;

#[macro_use]
extern crate bencher;

use bencher::Bencher;
use mprober_lib::*;

fn get_btime(bencher: &mut Bencher) {
    bencher.iter(btime::get_btime);
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

fn get_cpu_activity(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_cpu_activity().unwrap());
}

fn get_online_cpu_count(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_online_cpu_count().unwrap());
}

fn get_hostname(bencher: &mut Bencher) {
    bencher.iter(|| hostname::get_hostname().unwrap());
}

fn get_hwmon_devices(bencher: &mut Bencher) {
    bencher.iter(|| hwmon::get_hwmon_devices().unwrap());
}

fn get_kernel_version(bencher: &mut Bencher) {
    bencher.iter(|| kernel::get_kernel_version().unwrap());
}

fn get_uname(bencher: &mut Bencher) {
    bencher.iter(|| kernel::get_uname().unwrap());
}

fn get_file_nr(bencher: &mut Bencher) {
    bencher.iter(|| kernel::get_file_nr().unwrap());
}

fn get_load_average(bencher: &mut Bencher) {
    bencher.iter(|| load_average::get_load_average().unwrap());
}

fn free(bencher: &mut Bencher) {
    bencher.iter(|| memory::free().unwrap());
}

fn get_vm_stat(bencher: &mut Bencher) {
    bencher.iter(|| memory::get_vm_stat().unwrap());
}

fn get_networks(bencher: &mut Bencher) {
    bencher.iter(|| network::get_networks().unwrap());
}

fn get_network_info(bencher: &mut Bencher) {
    bencher.iter(|| network::get_network_info("lo").unwrap());
}

fn get_cpu_pressure(bencher: &mut Bencher) {
    bencher.iter(|| pressure::get_cpu_pressure().unwrap());
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

fn get_process_io(bencher: &mut Bencher) {
    let pid = std::process::id();

    bencher.iter(|| process::get_process_io(pid).unwrap());
}

fn get_rtc_date_time(bencher: &mut Bencher) {
    bencher.iter(|| rtc_time::get_rtc_date_time().unwrap());
}

fn get_uptime(bencher: &mut Bencher) {
    bencher.iter(|| uptime::get_uptime().unwrap());
}

fn get_mounts(bencher: &mut Bencher) {
    bencher.iter(|| volume::get_mounts().unwrap());
}

fn get_volumes(bencher: &mut Bencher) {
    bencher.iter(|| volume::get_volumes().unwrap());
}

benchmark_group!(btime, get_btime);

benchmark_group!(
    cpu,
    get_cpus,
    get_average_cpu_stat,
    get_all_cpus_stat_with_average,
    get_all_cpus_stat_without_average,
    get_cpu_activity,
    get_online_cpu_count
);
benchmark_group!(hostname, get_hostname);
benchmark_group!(hwmon, get_hwmon_devices);
benchmark_group!(kernel, get_kernel_version, get_uname, get_file_nr);
benchmark_group!(load_average, get_load_average);
benchmark_group!(memory, free, get_vm_stat);
benchmark_group!(network, get_networks, get_network_info);
benchmark_group!(pressure, get_cpu_pressure);
benchmark_group!(
    process,
    get_process_status,
    get_process_time_stat,
    get_process_stat,
    get_process_with_stat,
    get_process_io
);
benchmark_group!(rtc_time, get_rtc_date_time);
benchmark_group!(uptime, get_uptime);
benchmark_group!(volume, get_mounts, get_volumes);

benchmark_main!(
    btime,
    cpu,
    hostname,
    hwmon,
    kernel,
    load_average,
    memory,
    network,
    pressure,
    process,
    rtc_time,
    uptime,
    volume
);
