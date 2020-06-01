extern crate regex;

use regex::Regex;

#[derive(Default, Debug, Clone)]
pub struct ProcessFilter<'a> {
    pub pid_filter: Option<u32>,
    pub uid_filter: Option<u32>,
    pub gid_filter: Option<u32>,
    pub program_filter: Option<&'a Regex>,
    pub tty_filter: Option<&'a Regex>,
}
