extern crate mprober_lib;

#[macro_use]
extern crate bencher;

use bencher::Bencher;
use mprober_lib::*;

/// Run a benchmark only when the data source exists on this machine. A virtual machine has no cpufreq driver, a container has no RTC, and a kernel can be built without PSI, so those benchmarks would otherwise panic instead of being skipped.
fn bench_if_available<T, F: Fn() -> Result<T, Error>>(bencher: &mut Bencher, f: F) {
    if f().is_err() {
        bencher.iter(|| ());

        return;
    }

    bencher.iter(|| f().unwrap());
}

fn get_btime(bencher: &mut Bencher) {
    bencher.iter(btime::get_btime);
}

fn get_cgroup_path(bencher: &mut Bencher) {
    bench_if_available(bencher, cgroup::get_cgroup_path);
}

fn get_cgroup_cpu(bencher: &mut Bencher) {
    let Ok(path) = cgroup::get_cgroup_path() else {
        bencher.iter(|| ());

        return;
    };

    bench_if_available(bencher, || cgroup::get_cgroup_cpu(&path));
}

fn get_cgroup_memory(bencher: &mut Bencher) {
    let Ok(path) = cgroup::get_cgroup_path() else {
        bencher.iter(|| ());

        return;
    };

    bench_if_available(bencher, || cgroup::get_cgroup_memory(&path));
}

fn get_cgroup_memory_stat(bencher: &mut Bencher) {
    let Ok(path) = cgroup::get_cgroup_path() else {
        bencher.iter(|| ());

        return;
    };

    bench_if_available(bencher, || cgroup::get_cgroup_memory_stat(&path));
}

fn get_cgroup_io(bencher: &mut Bencher) {
    let Ok(path) = cgroup::get_cgroup_path() else {
        bencher.iter(|| ());

        return;
    };

    bench_if_available(bencher, || cgroup::get_cgroup_io(&path));
}

fn get_cgroup_pressure(bencher: &mut Bencher) {
    let Ok(path) = cgroup::get_cgroup_path() else {
        bencher.iter(|| ());

        return;
    };

    bench_if_available(bencher, || cgroup::get_cgroup_pressure(&path, "cpu"));
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

fn get_available_cpu_count(bencher: &mut Bencher) {
    bencher.iter(|| cpu::get_available_cpu_count().unwrap());
}

fn get_cpu_frequency(bencher: &mut Bencher) {
    bench_if_available(bencher, || cpu::get_cpu_frequency(0));
}

fn get_all_cpu_frequencies(bencher: &mut Bencher) {
    bench_if_available(bencher, cpu::get_all_cpu_frequencies);
}

fn get_all_cpu_topologies(bencher: &mut Bencher) {
    bench_if_available(bencher, cpu::get_all_cpu_topologies);
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

fn get_kernel_cmdline(bencher: &mut Bencher) {
    bench_if_available(bencher, kernel::get_kernel_cmdline);
}

fn get_load_average(bencher: &mut Bencher) {
    bencher.iter(|| load_average::get_load_average().unwrap());
}

fn free(bencher: &mut Bencher) {
    bencher.iter(|| memory::free().unwrap());
}

fn get_mem_info(bencher: &mut Bencher) {
    bencher.iter(|| memory::get_mem_info().unwrap());
}

fn get_vm_stat(bencher: &mut Bencher) {
    bencher.iter(|| memory::get_vm_stat().unwrap());
}

fn get_swaps(bencher: &mut Bencher) {
    bench_if_available(bencher, memory::get_swaps);
}

fn get_numa_nodes(bencher: &mut Bencher) {
    bencher.iter(|| memory::get_numa_nodes().unwrap());
}

fn get_networks(bencher: &mut Bencher) {
    bencher.iter(|| network::get_networks().unwrap());
}

fn get_network_info(bencher: &mut Bencher) {
    bench_if_available(bencher, || network::get_network_info("lo"));
}

fn get_network_addresses(bencher: &mut Bencher) {
    bencher.iter(|| network::get_network_addresses().unwrap());
}

fn get_socket_stat(bencher: &mut Bencher) {
    bench_if_available(bencher, network::get_socket_stat);
}

fn get_protocol_stat(bencher: &mut Bencher) {
    bench_if_available(bencher, network::get_protocol_stat);
}

fn get_all_socket_connections(bencher: &mut Bencher) {
    bench_if_available(bencher, network::get_all_socket_connections);
}

fn get_routes(bencher: &mut Bencher) {
    bench_if_available(bencher, network::get_routes);
}

fn get_power_supplies(bencher: &mut Bencher) {
    bencher.iter(|| power_supply::get_power_supplies().unwrap());
}

fn get_cpu_pressure(bencher: &mut Bencher) {
    bench_if_available(bencher, pressure::get_cpu_pressure);
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

fn get_processes_with_stat(bencher: &mut Bencher) {
    bencher.iter(|| process::get_processes_with_stat(&process::ProcessFilter::default()).unwrap());
}

fn get_process_io(bencher: &mut Bencher) {
    let pid = std::process::id();

    bench_if_available(bencher, || process::get_process_io(pid));
}

fn get_process_fd_count(bencher: &mut Bencher) {
    let pid = std::process::id();

    bencher.iter(|| process::get_process_fd_count(pid).unwrap());
}

fn get_process_memory(bencher: &mut Bencher) {
    let pid = std::process::id();

    bench_if_available(bencher, || process::get_process_memory(pid));
}

fn get_process_threads(bencher: &mut Bencher) {
    let pid = std::process::id();

    bencher.iter(|| process::get_process_threads(pid).unwrap());
}

fn get_process_oom(bencher: &mut Bencher) {
    let pid = std::process::id();

    bencher.iter(|| process::get_process_oom(pid).unwrap());
}

fn get_process_limits(bencher: &mut Bencher) {
    let pid = std::process::id();

    bencher.iter(|| process::get_process_limits(pid).unwrap());
}

fn get_rtc_date_time(bencher: &mut Bencher) {
    bench_if_available(bencher, rtc_time::get_rtc_date_time);
}

fn get_dmi_info(bencher: &mut Bencher) {
    bench_if_available(bencher, system::get_dmi_info);
}

fn get_boot_id(bencher: &mut Bencher) {
    bench_if_available(bencher, system::get_boot_id);
}

fn get_uptime(bencher: &mut Bencher) {
    bencher.iter(|| uptime::get_uptime().unwrap());
}

fn get_mount_infos(bencher: &mut Bencher) {
    bencher.iter(|| volume::get_mount_infos().unwrap());
}

fn get_volumes(bencher: &mut Bencher) {
    bencher.iter(|| volume::get_volumes().unwrap());
}

fn get_disk_stats(bencher: &mut Bencher) {
    bencher.iter(|| volume::get_disk_stats().unwrap());
}

fn get_block_devices(bencher: &mut Bencher) {
    bencher.iter(|| volume::get_block_devices().unwrap());
}

fn get_block_device_info(bencher: &mut Bencher) {
    // A machine without a mounted block device (e.g. a container on an overlay file system) has nothing to look up.
    let Some(device) = volume::get_volumes().unwrap().into_iter().next().map(|v| v.device) else {
        bencher.iter(|| ());

        return;
    };

    bench_if_available(bencher, || volume::get_block_device_info(&device));
}

benchmark_group!(btime, get_btime);
benchmark_group!(
    cgroup,
    get_cgroup_path,
    get_cgroup_cpu,
    get_cgroup_memory,
    get_cgroup_memory_stat,
    get_cgroup_io,
    get_cgroup_pressure
);
benchmark_group!(
    cpu,
    get_cpus,
    get_average_cpu_stat,
    get_all_cpus_stat_with_average,
    get_all_cpus_stat_without_average,
    get_cpu_activity,
    get_online_cpu_count,
    get_available_cpu_count,
    get_cpu_frequency,
    get_all_cpu_frequencies,
    get_all_cpu_topologies
);
benchmark_group!(hostname, get_hostname);
benchmark_group!(hwmon, get_hwmon_devices);
benchmark_group!(kernel, get_kernel_version, get_uname, get_file_nr, get_kernel_cmdline);
benchmark_group!(load_average, get_load_average);
benchmark_group!(memory, free, get_mem_info, get_vm_stat, get_swaps, get_numa_nodes);
benchmark_group!(
    network,
    get_networks,
    get_network_info,
    get_network_addresses,
    get_socket_stat,
    get_protocol_stat,
    get_all_socket_connections,
    get_routes
);
benchmark_group!(power_supply, get_power_supplies);
benchmark_group!(pressure, get_cpu_pressure);
benchmark_group!(
    process,
    get_process_status,
    get_process_time_stat,
    get_process_stat,
    get_process_with_stat,
    get_processes_with_stat,
    get_process_io,
    get_process_fd_count,
    get_process_memory,
    get_process_threads,
    get_process_oom,
    get_process_limits
);
benchmark_group!(rtc_time, get_rtc_date_time);
benchmark_group!(system, get_dmi_info, get_boot_id);
benchmark_group!(uptime, get_uptime);
benchmark_group!(
    volume,
    get_mount_infos,
    get_volumes,
    get_disk_stats,
    get_block_devices,
    get_block_device_info
);

benchmark_main!(
    btime,
    cgroup,
    cpu,
    hostname,
    hwmon,
    kernel,
    load_average,
    memory,
    network,
    power_supply,
    pressure,
    process,
    rtc_time,
    system,
    uptime,
    volume
);
