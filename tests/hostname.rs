extern crate mprober_lib;

use mprober_lib::hostname;

#[test]
fn get_hostname() {
    println!("{}", hostname::get_hostname().unwrap());
}
