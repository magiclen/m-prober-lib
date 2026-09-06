use std::time::Duration;

#[test]
fn format_duration() {
    assert_eq!("0 seconds", mprober_lib::format_duration(Duration::from_secs(0)));
    assert_eq!("1 second", mprober_lib::format_duration(Duration::from_secs(1)));
    assert_eq!("2 minutes", mprober_lib::format_duration(Duration::from_secs(120)));
    assert_eq!(
        "1 hour, 0 minutes, and 1 second",
        mprober_lib::format_duration(Duration::from_secs(3601))
    );
    assert_eq!(
        "1 day, 0 hours, 0 minutes, and 5 seconds",
        mprober_lib::format_duration(Duration::from_secs(86405))
    );
    assert_eq!(
        "1 day, 10 hours, 17 minutes, and 36 seconds",
        mprober_lib::format_duration(Duration::from_secs(123456))
    );
}
