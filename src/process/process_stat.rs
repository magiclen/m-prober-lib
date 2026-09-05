use std::{
    io::{self, ErrorKind},
    path::Path,
    str::from_utf8_unchecked,
};

use page_size::get as get_page_size;

use crate::{
    process::ProcessState,
    scanner_rust::{Scanner, ScannerError},
};

#[derive(Default, Debug, Clone)]
pub struct ProcessStat {
    pub state:        ProcessState,
    pub comm:         String,
    pub ppid:         u32,
    pub pgrp:         u32,
    pub session:      u32,
    pub tty_nr_major: u8,
    pub tty_nr_minor: u32,
    pub tpgid:        Option<u32>,
    pub utime:        u32,
    pub stime:        u32,
    pub cutime:       u32,
    pub cstime:       u32,
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
    let mut stat = ProcessStat::default();

    let stat_path = Path::new("/proc").join(pid.to_string()).join("stat");

    let mut sc: Scanner<_, 192> = Scanner::scan_path2(stat_path)?;

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next_until("(")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    loop {
        let comm = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        if comm.ends_with(b")") {
            stat.comm.push_str(unsafe { from_utf8_unchecked(&comm[..(comm.len() - 1)]) });
            break;
        } else {
            stat.comm.push_str(unsafe { from_utf8_unchecked(comm.as_ref()) });
        }
    }

    let state = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.state = ProcessState::from_str(unsafe { from_utf8_unchecked(&state) })
        .ok_or(io::Error::from(ErrorKind::InvalidData))?;

    stat.ppid = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.pgrp = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.session = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    {
        let tty_nr = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        stat.tty_nr_major = (tty_nr >> 8) as u8;
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

    stat.utime = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.stime = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.cutime = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.cstime = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.priority = sc.next_i8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.nice = sc.next_i8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.num_threads = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.starttime = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.vsize = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    // the `rss` field is read from the `statm` file later, in order to keep it consistent with the
    // `shared` field
    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.rsslim = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    for _ in 0..13 {
        sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    }

    stat.processor = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    stat.rt_priority = sc.next_u8()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    drop(sc);

    let statm_path = Path::new("/proc").join(pid.to_string()).join("statm");

    let mut sc: Scanner<_, 32> = Scanner::scan_path2(statm_path)?;

    sc.drop_next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    stat.rss = sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * get_page_size();
    stat.shared =
        sc.next_usize()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))? * get_page_size();

    stat.rss_anon = stat.rss - stat.shared;

    Ok(stat)
}
