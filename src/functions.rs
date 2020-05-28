use std::fmt::Write;
use std::time::Duration;

/// Format a `Duration` to a string. The string would be like `4 hours, 39 minutes, and 25 seconds`.
pub fn format_duration(duration: Duration) -> String {
    let sec = duration.as_secs();
    let days = sec / 86400;
    let sec = sec % 86400;
    let hours = sec / 3600;
    let sec = sec % 3600;
    let minutes = sec / 60;
    let seconds = sec % 60;

    let mut s = String::with_capacity(10);

    if days > 0 {
        s.write_fmt(format_args!("{} day", days)).unwrap();

        if days > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if hours > 0 || (days > 0) && (minutes > 0 || seconds > 0) {
        s.write_fmt(format_args!("{} hour", hours)).unwrap();

        if hours > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if minutes > 0 || (hours > 0 && seconds > 0) {
        s.write_fmt(format_args!("{} minute", minutes)).unwrap();

        if minutes > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if seconds > 0 {
        s.write_fmt(format_args!("{} second", seconds)).unwrap();

        if seconds > 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    debug_assert!(s.len() >= 2);

    if let Some(index) = s.as_str()[..(s.len() - 2)].rfind(", ") {
        s.insert_str(index + 2, "and ");
    }

    let mut v = s.into_bytes();

    unsafe {
        v.set_len(v.len() - 2);

        String::from_utf8_unchecked(v)
    }
}
