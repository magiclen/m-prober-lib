use std::fmt::{self, Display, Formatter};

/// The scheduling policy of a process, as it appears in the `policy` field of the `/proc/PID/stat` file.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SchedulingPolicy {
    /// The policy has not been read yet, or the kernel reported a value this crate does not know.
    #[default]
    Unknown,
    /// `SCHED_OTHER` (also called `SCHED_NORMAL`), the default time-sharing policy.
    Other,
    /// `SCHED_FIFO`, a real-time policy without time slicing.
    Fifo,
    /// `SCHED_RR`, a real-time policy with round-robin time slicing.
    RoundRobin,
    /// `SCHED_BATCH`, for CPU-intensive batch jobs.
    Batch,
    /// `SCHED_IDLE`, for very low priority background jobs.
    Idle,
    /// `SCHED_DEADLINE`, a real-time policy based on deadlines.
    Deadline,
    /// `SCHED_EXT`, a policy implemented by a BPF scheduler (Linux 6.12).
    Ext,
}

impl SchedulingPolicy {
    /// Convert the number that the `/proc/PID/stat` file reports.
    #[inline]
    pub fn from_raw(value: u32) -> SchedulingPolicy {
        match value {
            0 => SchedulingPolicy::Other,
            1 => SchedulingPolicy::Fifo,
            2 => SchedulingPolicy::RoundRobin,
            3 => SchedulingPolicy::Batch,
            5 => SchedulingPolicy::Idle,
            6 => SchedulingPolicy::Deadline,
            7 => SchedulingPolicy::Ext,
            _ => SchedulingPolicy::Unknown,
        }
    }

    /// Get the name of this policy as the `ps` command shows it, e.g. `TS` for `SCHED_OTHER`.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            SchedulingPolicy::Unknown => "?",
            SchedulingPolicy::Other => "TS",
            SchedulingPolicy::Fifo => "FF",
            SchedulingPolicy::RoundRobin => "RR",
            SchedulingPolicy::Batch => "B",
            SchedulingPolicy::Idle => "IDL",
            SchedulingPolicy::Deadline => "DLN",
            SchedulingPolicy::Ext => "EXT",
        }
    }
}

impl Display for SchedulingPolicy {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
