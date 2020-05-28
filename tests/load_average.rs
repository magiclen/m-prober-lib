extern crate mprober_lib;

use mprober_lib::load_average;

#[test]
fn get_load_average() {
    println!("{:?}", load_average::get_load_average().unwrap());
}
