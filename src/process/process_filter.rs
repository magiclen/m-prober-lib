use std::fmt::{self, Debug, Formatter};

/// The conditions a process must meet to be returned by `get_processes_with_stat`. A filter that is `None` matches every process.
///
/// The text filters are plain predicates, so any matcher can be used, e.g. a `regex::Regex` through `Some(&|s| regex.is_match(s))`.
///
/// ```rust
/// use mprober_lib::process::{self, ProcessFilter};
///
/// let is_shell = |s: &str| s.contains("sh");
///
/// let filter = ProcessFilter {
///     program_filter: Some(&is_shell),
///     ..ProcessFilter::default()
/// };
///
/// let processes = process::get_processes_with_stat(&filter).unwrap();
///
/// println!("{processes:#?}");
/// ```
#[derive(Default, Clone)]
pub struct ProcessFilter<'a> {
    /// Keep only this process and its descendants. A descendant is found by walking up the chain of parents, so one whose ancestor could not be read is left out, which happens when `/proc` is mounted with `hidepid` or when the ancestor exits during the scan.
    pub pid_filter:     Option<u32>,
    /// Keep a process only when any of its four user IDs is this one.
    pub uid_filter:     Option<u32>,
    /// Keep a process only when any of its four group IDs is this one.
    pub gid_filter:     Option<u32>,
    /// Keep a process only when this predicate accepts its command line or its program name.
    pub program_filter: Option<&'a dyn Fn(&str) -> bool>,
    /// Keep a process only when this predicate accepts the name of its controlling terminal, e.g. `pts/0`. A process without one is dropped.
    pub tty_filter:     Option<&'a dyn Fn(&str) -> bool>,
}

impl Debug for ProcessFilter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // A predicate has no useful `Debug` output, so only its presence is shown.
        f.debug_struct("ProcessFilter")
            .field("pid_filter", &self.pid_filter)
            .field("uid_filter", &self.uid_filter)
            .field("gid_filter", &self.gid_filter)
            .field("program_filter", &self.program_filter.map(|_| ".."))
            .field("tty_filter", &self.tty_filter.map(|_| ".."))
            .finish()
    }
}
