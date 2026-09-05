use std::{
    io::{self, ErrorKind},
    str::from_utf8,
};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    process::{ProcessState, SchedulingPolicy},
    utils::{page_size, proc_pid_path, read_single_record_file},
};

/// Fields read from the `/proc/PID/stat` file and the `/proc/PID/statm` file. Time fields are in `USER_HZ` clock ticks and memory fields are in bytes.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessStat {
    /// The state of the process.
    pub state:        ProcessState,
    /// The file name of the executable, without the surrounding parentheses. It may contain spaces.
    pub comm:         String,
    /// The PID of the parent process.
    pub ppid:         u32,
    /// The process group ID.
    pub pgrp:         u32,
    /// The session ID.
    pub session:      u32,
    /// The major number of the controlling terminal.
    pub tty_nr_major: u16,
    /// The minor number of the controlling terminal.
    pub tty_nr_minor: u32,
    /// The process group ID of the foreground process group of the controlling terminal.
    pub tpgid:        Option<u32>,
    /// The number of minor page faults, which did not need disk I/O.
    pub minflt:       u64,
    /// The number of major page faults, which needed disk I/O.
    pub majflt:       u64,
    /// Time spent in user mode, in clock ticks.
    pub utime:        u64,
    /// Time spent in kernel mode, in clock ticks.
    pub stime:        u64,
    /// Time the waited-for children spent in user mode, in clock ticks.
    pub cutime:       u64,
    /// Time the waited-for children spent in kernel mode, in clock ticks.
    pub cstime:       u64,
    /// The scheduling priority.
    pub priority:     i8,
    /// The nice value, from `-20` (high priority) to `19` (low priority).
    pub nice:         i8,
    /// The number of threads in this process.
    pub num_threads:  usize,
    /// The time the process started after system boot, in clock ticks.
    pub starttime:    u64,
    /// The virtual memory size in bytes (`size` in `statm`, `VmSize`).
    pub vsize:        u64,
    /// The resident set size in bytes (`resident` in `statm`, `VmRSS`).
    pub rss:          u64,
    /// The soft limit on the RSS in bytes. It is `u64::MAX` when the limit is unlimited.
    pub rsslim:       u64,
    /// The CPU number last executed on.
    pub processor:    usize,
    /// The real-time scheduling priority. It is `0` for a process not running under a real-time policy.
    pub rt_priority:  u8,
    /// The scheduling policy.
    pub policy:       SchedulingPolicy,
    /// The resident shared size in bytes, which is `RssFile + RssShmem`.
    pub shared:       u64,
    /// The resident anonymous memory in bytes, which is `VmRSS - RssFile - RssShmem = RssAnon`. This is the memory the process occupies by itself.
    pub rss_anon:     u64,
}

/// Read the whole `/proc/PID/stat` file, which is always a single line.
#[inline]
pub(crate) fn read_process_stat_file(pid: u32) -> Result<Vec<u8>, Error> {
    let stat_path = proc_pid_path(pid).join("stat");

    Ok(read_single_record_file(stat_path, 1024)?)
}

/// Split a `/proc/PID/stat` line into the `comm` part and the fields after it.
/// `comm` may contain spaces and parentheses, so the last `)` is the real end of it.
pub(crate) fn split_process_stat_line(line: &[u8]) -> Result<(&[u8], &[u8]), Error> {
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
pub(crate) fn get_process_ppid(pid: u32) -> Result<u32, Error> {
    let line = read_process_stat_file(pid)?;

    let (_, fields) = split_process_stat_line(&line)?;

    let mut sc = ScannerU8SliceAscii::new(fields);

    // Skip the state field.
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?)
}

pub(crate) fn parse_process_stat(line: &[u8]) -> Result<ProcessStat, Error> {
    let (comm, fields) = split_process_stat_line(line)?;

    let mut stat = ProcessStat {
        comm: String::from_utf8_lossy(comm).into_owned(),
        ..ProcessStat::default()
    };

    let mut sc = ScannerU8SliceAscii::new(fields);

    let state = sc.next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // A state letter this crate does not know is not an error, because the kernel has added new letters over time.
    stat.state =
        from_utf8(state).ok().and_then(|s| s.parse().ok()).unwrap_or(ProcessState::Unknown);

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

    // flags
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.minflt = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // cminflt
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.majflt = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // cmajflt
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.utime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.stime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.cutime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.cstime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.priority = sc.next_i8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.nice = sc.next_i8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.num_threads = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.starttime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    // The sizes are in bytes, so they are read as `u64` even on a 32-bit target, where a 64-bit kernel reports processes that do not fit into `usize`.
    stat.vsize = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // the `rss` field is read from the `statm` file later, in order to keep it consistent with the `shared` field
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // This is `RLIM_INFINITY` for most processes, which does not fit into a 32-bit `usize`.
    stat.rsslim = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    for _ in 0..13 {
        sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    stat.processor = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.rt_priority = sc.next_u8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.policy = SchedulingPolicy::from_raw(
        sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?,
    );

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
pub fn get_process_stat(pid: u32) -> Result<ProcessStat, Error> {
    let mut stat = get_process_stat_without_memory(pid)?;

    read_process_statm_file(pid, &mut stat)?;

    Ok(stat)
}

/// Get the stat of a process without the memory fields, which live in a separate file. A caller that may drop the process right away saves one `open` and one `read` this way.
#[inline]
pub(crate) fn get_process_stat_without_memory(pid: u32) -> Result<ProcessStat, Error> {
    let line = read_process_stat_file(pid)?;

    parse_process_stat(&line)
}

/// Fill the memory fields of a stat by reading the `/proc/PID/statm` file. They come from there instead of from the `stat` file, so that `rss` and `shared` are consistent with each other.
pub(crate) fn read_process_statm_file(pid: u32, stat: &mut ProcessStat) -> Result<(), Error> {
    let statm_path = proc_pid_path(pid).join("statm");

    // The file is seven small numbers, so it always fits into one read.
    let statm = read_single_record_file(statm_path, 64)?;

    let mut sc = ScannerU8SliceAscii::new(&statm);

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let page_size = page_size() as u64;

    stat.rss = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * page_size;
    stat.shared = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * page_size;

    stat.rss_anon = stat.rss.saturating_sub(stat.shared);

    Ok(())
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
        assert_eq!(25149, stat.minflt);
        assert_eq!(33, stat.majflt);
        assert_eq!(26, stat.utime);
        assert_eq!(45, stat.stime);
        assert_eq!(17345, stat.cutime);
        assert_eq!(2680, stat.cstime);
        assert_eq!(20, stat.priority);
        assert_eq!(0, stat.nice);
        assert_eq!(1, stat.num_threads);
        assert_eq!(16, stat.starttime);
        assert_eq!(23744512, stat.vsize);
        assert_eq!(u64::MAX, stat.rsslim);
        assert_eq!(2, stat.processor);
        assert_eq!(0, stat.rt_priority);
        assert_eq!(SchedulingPolicy::Other, stat.policy);
    }

    #[test]
    fn parse_unknown_state() {
        let mut line = STAT_LINE.to_vec();

        let index = line.windows(3).position(|w| w == b") S").unwrap() + 2;
        line[index] = b'Q';

        let stat = parse_process_stat(&line).unwrap();

        assert_eq!(ProcessState::Unknown, stat.state);
        assert_eq!(1, stat.ppid);
    }
}
