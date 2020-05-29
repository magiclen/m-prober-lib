extern crate mprober_lib;

use mprober_lib::rtc_time;

#[test]
fn get_rtc_date_time() {
    println!("{:?}", rtc_time::get_rtc_date_time().unwrap());
}
