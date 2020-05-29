extern crate mprober_lib;

use mprober_lib::memory;

#[test]
fn free() {
    println!("{:?}", memory::free().unwrap());
}
