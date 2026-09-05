use regex::Regex;

/// The conditions a process must meet to be returned by `get_processes_with_stat`. A filter that is `None` matches every process.
#[derive(Default, Debug, Clone)]
pub struct ProcessFilter<'a> {
    /// Keep only this process and its descendants.
    pub pid_filter:     Option<u32>,
    /// Keep a process only when any of its four user IDs is this one.
    pub uid_filter:     Option<u32>,
    /// Keep a process only when any of its four group IDs is this one.
    pub gid_filter:     Option<u32>,
    /// Keep a process only when this pattern matches its command line or its program name.
    pub program_filter: Option<&'a Regex>,
    /// Keep a process only when this pattern matches its controlling terminal. A process without one is dropped.
    pub tty_filter:     Option<&'a Regex>,
}
