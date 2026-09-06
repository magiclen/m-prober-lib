use std::io::{self, ErrorKind};

use scanner_rust::ScannerU8SliceAscii;

use crate::{Error, cpu::proc_stat_capacity, utils::read_file};

/// System-wide scheduler counters read from the `/proc/stat` file.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPUActivity {
    /// The total number of interrupts serviced since boot.
    pub interrupts:        u64,
    /// The total number of context switches since boot.
    pub context_switches:  u64,
    /// The number of processes and threads created since boot.
    pub processes_created: u64,
    /// The number of processes currently runnable.
    pub procs_running:     u64,
    /// The number of processes currently blocked waiting for I/O.
    pub procs_blocked:     u64,
    /// The total number of softirqs serviced since boot.
    pub softirqs:          u64,
}

/// Get system-wide scheduler counters by reading the `/proc/stat` file.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let cpu_activity = cpu::get_cpu_activity().unwrap();
///
/// println!("{cpu_activity:#?}");
/// ```
pub fn get_cpu_activity() -> Result<CPUActivity, Error> {
    // The `softirq` line this needs is the last one, so the whole file is read.
    let data = read_file("/proc/stat", proc_stat_capacity())?;

    let mut sc = ScannerU8SliceAscii::new(&data);

    let mut activity = CPUActivity::default();

    while let Some(label) = sc.next()? {
        match label {
            b"intr" => {
                activity.interrupts =
                    sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            },
            b"ctxt" => {
                activity.context_switches =
                    sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            },
            b"processes" => {
                activity.processes_created =
                    sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            },
            b"procs_running" => {
                activity.procs_running =
                    sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            },
            b"procs_blocked" => {
                activity.procs_blocked =
                    sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            },
            b"softirq" => {
                activity.softirqs =
                    sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
            },
            _ => (),
        }

        // The `intr` and `softirq` lines have hundreds of per-source counters, which are not needed.
        sc.drop_next_line()?;
    }

    Ok(activity)
}
