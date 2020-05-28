extern crate mprober_lib;

use mprober_lib::kernel;

#[test]
fn get_kernel_version() {
    println!("{}", kernel::get_kernel_version().unwrap());
}
