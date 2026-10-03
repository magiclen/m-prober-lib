use crate::{
    Error,
    utils::{proc_pid_path, read_sysfs_number},
};

/// How likely the OOM killer is to pick a process, read from the `/proc/PID/oom_*` files.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProcessOOM {
    /// The badness score the OOM killer computes, from `0` to `2000`. The process with the highest one is killed first. Since Linux 5.9 the kernel scales it so that a process without an adjustment scores at least `666`.
    pub score:     u32,
    /// The adjustment an administrator set, from `-1000` to `1000`. It is added to the score, and `-1000` makes the process immune.
    pub score_adj: i32,
}

impl ProcessOOM {
    /// Whether the process can never be picked by the OOM killer, which is what an adjustment of `-1000` means.
    #[inline]
    pub fn is_immune(&self) -> bool {
        self.score_adj <= -1000
    }
}

/// Get the OOM killer score of a specific process found by ID by reading the `/proc/PID/oom_score` file and the `/proc/PID/oom_score_adj` file.
///
/// ```rust
/// use mprober_lib::process;
///
/// let process_oom = process::get_process_oom(std::process::id()).unwrap();
///
/// println!("{process_oom:#?}");
/// ```
pub fn get_process_oom(pid: u32) -> Result<ProcessOOM, Error> {
    let path = proc_pid_path(pid);

    let score = read_sysfs_number(path.join("oom_score"))?;
    let score_adj = read_sysfs_number(path.join("oom_score_adj"))?;

    Ok(ProcessOOM {
        score,
        score_adj,
    })
}
