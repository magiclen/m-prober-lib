extern crate regex;

use regex::Regex;

#[derive(Default, Debug, Clone)]
pub struct ProcessFilter {
    pub pid_filter: Option<u32>,
    pub uid_filter: Option<u32>,
    pub gid_filter: Option<u32>,
    pub program_filter: Option<Regex>,
    pub tty_filter: Option<Regex>,
}
