extern crate mprober_lib;

#[macro_use]
extern crate bencher;

use bencher::Bencher;

use mprober_lib::*;

fn get_btime(bencher: &mut Bencher) {
    bencher.iter(|| btime::get_btime_by_uptime(uptime::get_uptime().unwrap()));
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

fn get_rtc_date_time(bencher: &mut Bencher) {
    bencher.iter(|| rtc_time::get_rtc_date_time().unwrap());
}

fn get_uptime(bencher: &mut Bencher) {
    bencher.iter(|| uptime::get_uptime().unwrap());
}

benchmark_group!(btime, get_btime);
benchmark_group!(hostname, get_hostname);
benchmark_group!(kernel, get_kernel_version);
benchmark_group!(load_average, get_load_average);
benchmark_group!(rtc_time, get_rtc_date_time);
benchmark_group!(uptime, get_uptime);

benchmark_main!(btime, hostname, kernel, load_average, rtc_time, uptime);
