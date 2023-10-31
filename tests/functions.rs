use std::time::Duration;

#[test]
fn format_duration() {
    assert_eq!(
        "1 day, 10 hours, 17 minutes, and 36 seconds",
        mprober_lib::format_duration(Duration::from_secs(123456))
    );
}
