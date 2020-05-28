extern crate mprober_lib;

use std::time::Duration;

use mprober_lib::functions;

#[test]
fn format_duration() {
    assert_eq!(
        "1 day, 10 hours, 17 minutes, and 36 seconds",
        functions::format_duration(Duration::from_secs(123456))
    );
}
