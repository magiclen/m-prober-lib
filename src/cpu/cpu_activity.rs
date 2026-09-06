use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    cpu::proc_stat_capacity,
    utils::{OrEof, read_file},
};

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

fn parse_cpu_activity(data: &[u8]) -> Result<CPUActivity, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let mut activity = CPUActivity::default();

    while let Some(label) = sc.next()? {
        match label {
            b"intr" => {
                activity.interrupts = sc.next_u64()?.or_eof()?;
            },
            b"ctxt" => {
                activity.context_switches = sc.next_u64()?.or_eof()?;
            },
            b"processes" => {
                activity.processes_created = sc.next_u64()?.or_eof()?;
            },
            b"procs_running" => {
                activity.procs_running = sc.next_u64()?.or_eof()?;
            },
            b"procs_blocked" => {
                activity.procs_blocked = sc.next_u64()?.or_eof()?;
            },
            b"softirq" => {
                activity.softirqs = sc.next_u64()?.or_eof()?;
            },
            _ => (),
        }

        // The `intr` and `softirq` lines have hundreds of per-source counters, which are not needed.
        sc.drop_next_line()?;
    }

    Ok(activity)
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
#[inline]
pub fn get_cpu_activity() -> Result<CPUActivity, Error> {
    // The `softirq` line this needs is the last one, so the whole file is read.
    parse_cpu_activity(&read_file("/proc/stat", proc_stat_capacity())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &[u8] = b"cpu  228791 17 138764 57861882 19268 0 624 0 0 0
cpu0 9459 0 5828 2410479 850 0 116 0 0 0
cpu1 9107 1 5710 2411431 761 0 40 0 0 0
intr 68724016 22 1055 0 0 0 0 0 0 0 60 0 0 138 0 0 0
ctxt 174031029
btime 1757132817
processes 320157
procs_running 2
procs_blocked 0
softirq 44861827 1355717 3743832 12 289093 116414 0 6414 20574618 0 18775727
";

    #[test]
    fn parse() {
        let activity = parse_cpu_activity(STAT).unwrap();

        assert_eq!(68724016, activity.interrupts);
        assert_eq!(174031029, activity.context_switches);
        assert_eq!(320157, activity.processes_created);
        assert_eq!(2, activity.procs_running);
        assert_eq!(0, activity.procs_blocked);
        assert_eq!(44861827, activity.softirqs);
    }
}
