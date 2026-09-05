use std::{fmt::Write, time::Duration};

/// Format a `Duration` to a string. The string would be like `4 hours, 39 minutes, and 25 seconds`.
///
/// ```rust
/// use mprober_lib::uptime;
///
/// let uptime = uptime::get_uptime().unwrap();
///
/// println!("{}", mprober_lib::format_duration(uptime.total_uptime))
/// ```
pub fn format_duration(duration: Duration) -> String {
    let sec = duration.as_secs();
    let days = sec / 86400;
    let sec = sec % 86400;
    let hours = sec / 3600;
    let sec = sec % 3600;
    let minutes = sec / 60;
    let seconds = sec % 60;

    let mut s = String::with_capacity(48);

    if days > 0 {
        write!(s, "{days} day").unwrap();

        if days != 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    // A zero unit is still shown when a bigger unit and a smaller unit are both shown, so the chain has no gap.
    if hours > 0 || (days > 0 && (minutes > 0 || seconds > 0)) {
        write!(s, "{hours} hour").unwrap();

        if hours != 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if minutes > 0 || ((days > 0 || hours > 0) && seconds > 0) {
        write!(s, "{minutes} minute").unwrap();

        if minutes != 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if seconds > 0 {
        write!(s, "{seconds} second").unwrap();

        if seconds != 1 {
            s.push('s');
        }

        s.push_str(", ");
    }

    if s.is_empty() {
        return String::from("0 seconds");
    }

    s.truncate(s.len() - 2);

    if let Some(index) = s.rfind(", ") {
        s.insert_str(index + 2, "and ");
    }

    s
}
