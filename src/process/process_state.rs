use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

/// The error returned when a string is not a known process state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ParseProcessStateError;

impl Display for ParseProcessStateError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("unknown process state")
    }
}

impl Error for ParseProcessStateError {}

/// The single-character state of a process, as it appears in the `/proc/PID/stat` file.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ProcessState {
    /// The state has not been read yet, or the kernel reported a character this crate does not know.
    #[default]
    Unknown,
    /// `R`
    Running,
    /// `S`
    Sleeping,
    /// `D`, uninterruptible sleep
    Waiting,
    /// `Z`
    Zombie,
    /// `T`
    Stopped,
    /// `t`
    TracingStop,
    /// `W`, paging on Linux 2.4 and waking on Linux 2.6.33 to 3.13
    PagingOrWaking,
    /// `X` or `x`
    Dead,
    /// `K`
    Wakekill,
    /// `P`
    Parked,
    /// `I`
    Idle,
}

impl ProcessState {
    /// Get the name of this state, e.g. `Sleeping`.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            ProcessState::Unknown => "Unknown",
            ProcessState::Running => "Running",
            ProcessState::Sleeping => "Sleeping",
            ProcessState::Waiting => "Waiting",
            ProcessState::Zombie => "Zombie",
            ProcessState::Stopped => "Stopped",
            ProcessState::TracingStop => "TracingStop",
            ProcessState::PagingOrWaking => "PagingOrWaking",
            ProcessState::Dead => "Dead",
            ProcessState::Wakekill => "Wakekill",
            ProcessState::Parked => "Parked",
            ProcessState::Idle => "Idle",
        }
    }
}

impl Display for ProcessState {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProcessState {
    type Err = ParseProcessStateError;

    /// Parse the single-character state that the `/proc/PID/stat` file reports.
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(ProcessState::Running),
            "S" => Ok(ProcessState::Sleeping),
            "D" => Ok(ProcessState::Waiting),
            "Z" => Ok(ProcessState::Zombie),
            "T" => Ok(ProcessState::Stopped),
            "t" => Ok(ProcessState::TracingStop),
            "W" => Ok(ProcessState::PagingOrWaking),
            "X" | "x" => Ok(ProcessState::Dead),
            "K" => Ok(ProcessState::Wakekill),
            "P" => Ok(ProcessState::Parked),
            "I" => Ok(ProcessState::Idle),
            _ => Err(ParseProcessStateError),
        }
    }
}
