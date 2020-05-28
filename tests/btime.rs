extern crate mprober_lib;

use mprober_lib::{btime, uptime};

#[test]
fn get_btime() {
    println!("{:?}", btime::get_btime_by_uptime(uptime::get_uptime().unwrap()));
}
