extern crate mprober_lib;

use mprober_lib::uptime;

#[test]
fn get_uptime() {
    println!("{:?}", uptime::get_uptime().unwrap());
}
