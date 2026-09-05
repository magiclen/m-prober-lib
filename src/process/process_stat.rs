use std::{
    io::{self, ErrorKind},
    path::Path,
    str::from_utf8,
};

use crate::{
    process::ProcessState,
    scanner_rust::{Scanner, ScannerError, ScannerU8SliceAscii},
    utils::{page_size, read_file},
};

#[derive(Default, Debug, Clone)]
pub struct ProcessStat {
    pub state:        ProcessState,
    pub comm:         String,
    pub ppid:         u32,
    pub pgrp:         u32,
    pub session:      u32,
    pub tty_nr_major: u16,
    pub tty_nr_minor: u32,
    pub tpgid:        Option<u32>,
    pub utime:        u64,
    pub stime:        u64,
    pub cutime:       u64,
    pub cstime:       u64,
    pub priority:     i8,
    pub nice:         i8,
    pub num_threads:  usize,
    pub starttime:    u64,
    /// size, VmSize (total program size)
    pub vsize:        usize,
    /// resident, VmRSS (resident set size)
    pub rss:          usize,
    pub rsslim:       usize,
    pub processor:    usize,
    pub rt_priority:  u8,
    /// RssFile + RssShmem (resident shared size)
    pub shared:       usize,
    /// VmRSS - RssFile - RssShmem = RssAnon (resident anonymous memory, process occupied memory)
    pub rss_anon:     usize,
}

/// Read the whole `/proc/PID/stat` file, which is always a single line.
#[inline]
pub(crate) fn read_process_stat_file(pid: u32) -> Result<Vec<u8>, ScannerError> {
    let stat_path = Path::new("/proc").join(pid.to_string()).join("stat");

    Ok(read_file(stat_path, 512)?)
}

/// Split a `/proc/PID/stat` line into the `comm` part and the fields after it.
/// `comm` may contain spaces and parentheses, so the last `)` is the real end of it.
pub(crate) fn split_process_stat_line(line: &[u8]) -> Result<(&[u8], &[u8]), ScannerError> {
    let start =
        line.iter().position(|&b| b == b'(').ok_or(io::Error::from(ErrorKind::InvalidData))?;
    let end =
        line.iter().rposition(|&b| b == b')').ok_or(io::Error::from(ErrorKind::InvalidData))?;

    if end < start {
        return Err(io::Error::from(ErrorKind::InvalidData).into());
    }

    Ok((&line[(start + 1)..end], &line[(end + 1)..]))
}

/// Get only the parent PID of a process by reading the `/proc/PID/stat` file.
pub(crate) fn get_process_ppid(pid: u32) -> Result<u32, ScannerError> {
    let line = read_process_stat_file(pid)?;

    let (_, fields) = split_process_stat_line(&line)?;

    let mut sc = ScannerU8SliceAscii::new(fields);

    // Skip the state field.
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?)
}

fn parse_process_stat(line: &[u8]) -> Result<ProcessStat, ScannerError> {
    let (comm, fields) = split_process_stat_line(line)?;

    let mut stat = ProcessStat {
        comm: String::from_utf8_lossy(comm).into_owned(),
        ..ProcessStat::default()
    };

    let mut sc = ScannerU8SliceAscii::new(fields);

    let state = sc.next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.state = from_utf8(state)
        .ok()
        .and_then(ProcessState::from_str)
        .ok_or(io::Error::from(ErrorKind::InvalidData))?;

    stat.ppid = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.pgrp = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.session = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    {
        let tty_nr = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        // This is how the kernel encodes `dev_t`: 12-bit major, 20-bit minor split into two parts.
        stat.tty_nr_major = ((tty_nr >> 8) & 0xFFF) as u16;
        stat.tty_nr_minor = ((tty_nr >> 20) << 8) | (tty_nr & 0xFF);
    }

    {
        let tpgid = sc.next_i32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        if tpgid >= 0 {
            stat.tpgid = Some(tpgid as u32);
        }
    }

    for _ in 0..5 {
        sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    stat.utime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.stime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.cutime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.cstime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.priority = sc.next_i8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.nice = sc.next_i8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.num_threads = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.starttime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.vsize = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // the `rss` field is read from the `statm` file later, in order to keep it consistent with the `shared` field
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.rsslim = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    for _ in 0..13 {
        sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    stat.processor = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.rt_priority = sc.next_u8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(stat)
}

/// Get the stat of a specific process found by ID by reading the `/proc/PID/stat` file and the `/proc/PID/statm` file.
///
/// ```rust
/// use mprober_lib::process;
///
/// let process_stat = process::get_process_stat(1).unwrap();
///
/// println!("{process_stat:#?}");
/// ```
pub fn get_process_stat(pid: u32) -> Result<ProcessStat, ScannerError> {
    let line = read_process_stat_file(pid)?;

    let mut stat = parse_process_stat(&line)?;

    let statm_path = Path::new("/proc").join(pid.to_string()).join("statm");

    let mut sc: Scanner<_, 32> = Scanner::scan_path2(statm_path)?;

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let page_size = page_size();

    stat.rss = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * page_size;
    stat.shared = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * page_size;

    stat.rss_anon = stat.rss.saturating_sub(stat.shared);

    Ok(stat)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const STAT_LINE: &[u8] = b"1234 (Web (Content) x) S 1 1234 1234 34816 1234 4194560 25149 10479099 33 1321 26 45 17345 2680 20 0 1 0 16 23744512 3387 18446744073709551615 1 1 0 0 0 0 671173123 4096 1260 0 0 0 17 2 0 0 0 0 0 0 0 0 0 0 0 0 0\n";

    #[test]
    fn parse_stat_line() {
        let stat = parse_process_stat(STAT_LINE).unwrap();

        assert_eq!("Web (Content) x", stat.comm);
        assert_eq!(ProcessState::Sleeping, stat.state);
        assert_eq!(1, stat.ppid);
        assert_eq!(1234, stat.pgrp);
        assert_eq!(1234, stat.session);
        assert_eq!(136, stat.tty_nr_major);
        assert_eq!(0, stat.tty_nr_minor);
        assert_eq!(Some(1234), stat.tpgid);
        assert_eq!(26, stat.utime);
        assert_eq!(45, stat.stime);
        assert_eq!(17345, stat.cutime);
        assert_eq!(2680, stat.cstime);
        assert_eq!(20, stat.priority);
        assert_eq!(0, stat.nice);
        assert_eq!(1, stat.num_threads);
        assert_eq!(16, stat.starttime);
        assert_eq!(23744512, stat.vsize);
        assert_eq!(usize::MAX, stat.rsslim);
        assert_eq!(2, stat.processor);
        assert_eq!(0, stat.rt_priority);
    }
}
