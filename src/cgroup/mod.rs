use std::{
    io::{self, ErrorKind},
    num::ParseIntError,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    Error,
    scanner_rust::ScannerU8SliceAscii,
    utils::{parse_number, read_single_record_file, read_sysfs_number, read_sysfs_string},
};

/// Get the path of the cgroup (v2) that the current process belongs to, by reading the `/proc/self/cgroup` file. The path is inside `/sys/fs/cgroup`, e.g. `/sys/fs/cgroup/user.slice/user-1000.slice` on a host, or `/sys/fs/cgroup` itself in a container. A `NotFound` error is returned when the process is not in a cgroup v2 hierarchy.
///
/// ```rust
/// use mprober_lib::cgroup;
///
/// let cgroup_path = cgroup::get_cgroup_path().unwrap();
///
/// println!("{}", cgroup_path.display());
/// ```
pub fn get_cgroup_path() -> Result<PathBuf, Error> {
    let data = read_single_record_file("/proc/self/cgroup", 512)?;

    // The cgroup v2 line looks like `0::/user.slice/user-1000.slice`, while a cgroup v1 line has a controller list between the colons instead.
    for line in data.split(|&b| b == b'\n') {
        if let Some(path) = line.strip_prefix(b"0::") {
            let path = String::from_utf8_lossy(path.trim_ascii_end());

            return Ok(Path::new("/sys/fs/cgroup").join(path.trim_start_matches('/')));
        }
    }

    Err(io::Error::new(ErrorKind::NotFound, "the process is not in a cgroup v2 hierarchy").into())
}

/// Parse a limit which is either a number or `max`.
#[inline]
fn parse_limit(value: &str) -> Result<Option<u64>, ParseIntError> {
    if value == "max" { Ok(None) } else { value.parse().map(Some) }
}

/// Read a limit file which contains either a number or `max`.
#[inline]
fn read_limit<P: AsRef<Path>>(path: P) -> Result<Option<u64>, Error> {
    Ok(parse_limit(&read_sysfs_string(path)?)?)
}

/// The memory usage and limits of a cgroup, read from the `memory.*` files.
#[derive(Default, Debug, Clone)]
pub struct CgroupMemory {
    /// The memory used by the cgroup and its descendants in bytes (`memory.current`).
    pub current:      u64,
    /// The hard memory limit in bytes (`memory.max`). It is `None` when there is no limit.
    pub max:          Option<u64>,
    /// The swap used by the cgroup and its descendants in bytes (`memory.swap.current`).
    pub swap_current: u64,
    /// The hard swap limit in bytes (`memory.swap.max`). It is `None` when there is no limit.
    pub swap_max:     Option<u64>,
}

/// Get the memory usage and limits of the cgroup at `path` by reading the `memory.*` files in it. The files do not exist in the root cgroup, or when the memory controller is not enabled for the cgroup, so a `NotFound` error is returned then.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let cgroup_memory =
///     cgroup::get_cgroup_memory(cgroup::get_cgroup_path().unwrap()).unwrap();
///
/// println!("{cgroup_memory:#?}");
/// ```
pub fn get_cgroup_memory<P: AsRef<Path>>(path: P) -> Result<CgroupMemory, Error> {
    let path = path.as_ref();

    let current = read_sysfs_number(path.join("memory.current"))?;
    let max = read_limit(path.join("memory.max"))?;

    // The swap files do not exist when swap accounting is disabled.
    let swap_current = read_sysfs_number(path.join("memory.swap.current")).unwrap_or(0);
    let swap_max = read_limit(path.join("memory.swap.max")).unwrap_or(None);

    Ok(CgroupMemory {
        current,
        max,
        swap_current,
        swap_max,
    })
}

/// The CPU usage and limits of a cgroup, read from the `cpu.max` file and the `cpu.stat` file.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
pub struct CgroupCPU {
    /// The CPU time the cgroup may use in every `period` (`cpu.max`). It is `None` when there is no limit.
    pub quota:        Option<Duration>,
    /// The length of one accounting period (`cpu.max`), `100ms` by default.
    pub period:       Duration,
    /// The total CPU time used (`usage_usec`).
    pub usage:        Duration,
    /// The CPU time used in user mode (`user_usec`).
    pub user:         Duration,
    /// The CPU time used in kernel mode (`system_usec`).
    pub system:       Duration,
    /// The number of periods in which the cgroup was throttled because it used up its quota (`nr_throttled`).
    pub nr_throttled: u64,
    /// The total time the cgroup was throttled (`throttled_usec`).
    pub throttled:    Duration,
}

impl CgroupCPU {
    /// Get the number of CPUs the cgroup may use at most, which is `quota / period`. It is `None` when there is no limit.
    ///
    /// ```rust,no_run
    /// use mprober_lib::cgroup;
    ///
    /// let cgroup_cpu =
    ///     cgroup::get_cgroup_cpu(cgroup::get_cgroup_path().unwrap()).unwrap();
    ///
    /// println!("{:?}", cgroup_cpu.effective_cpu_count());
    /// ```
    #[inline]
    pub fn effective_cpu_count(&self) -> Option<f64> {
        let quota = self.quota?;

        if self.period.is_zero() {
            return None;
        }

        // Both values are whole microseconds in `cpu.max`, so dividing them as such keeps the result exact.
        Some(quota.as_micros() as f64 / self.period.as_micros() as f64)
    }
}

/// Parse the content of `cpu.max`, which looks like `max 100000` or `50000 100000`.
fn parse_cpu_max(data: &[u8]) -> Result<(Option<Duration>, Duration), Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let quota = sc.next()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let quota = if quota == b"max" { None } else { Some(parse_number::<u64>(quota)?) };

    let period = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok((quota.map(Duration::from_micros), Duration::from_micros(period)))
}

/// Parse the content of `cpu.stat` into `cpu`.
fn parse_cpu_stat(data: &[u8], cpu: &mut CgroupCPU) -> Result<(), Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    while let Some(key) = sc.next()? {
        let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        match key {
            b"usage_usec" => cpu.usage = Duration::from_micros(value),
            b"user_usec" => cpu.user = Duration::from_micros(value),
            b"system_usec" => cpu.system = Duration::from_micros(value),
            b"nr_throttled" => cpu.nr_throttled = value,
            b"throttled_usec" => cpu.throttled = Duration::from_micros(value),
            _ => (),
        }
    }

    Ok(())
}

/// Get the CPU usage and limits of the cgroup at `path` by reading the `cpu.max` file and the `cpu.stat` file in it. The `cpu.max` file does not exist in the root cgroup, or when the CPU controller is not enabled for the cgroup, which means there is no limit.
///
/// ```rust
/// use mprober_lib::cgroup;
///
/// let cgroup_cpu =
///     cgroup::get_cgroup_cpu(cgroup::get_cgroup_path().unwrap()).unwrap();
///
/// println!("{cgroup_cpu:#?}");
/// ```
pub fn get_cgroup_cpu<P: AsRef<Path>>(path: P) -> Result<CgroupCPU, Error> {
    let path = path.as_ref();

    let mut cpu = CgroupCPU {
        period: Duration::from_millis(100),
        ..CgroupCPU::default()
    };

    match read_single_record_file(path.join("cpu.max"), 64) {
        Ok(data) => (cpu.quota, cpu.period) = parse_cpu_max(&data)?,
        Err(err) if err.kind() == ErrorKind::NotFound => (),
        Err(err) => return Err(err.into()),
    }

    parse_cpu_stat(&read_single_record_file(path.join("cpu.stat"), 512)?, &mut cpu)?;

    Ok(cpu)
}

/// The number of processes in a cgroup, read from the `pids.current` file and the `pids.max` file.
#[derive(Default, Debug, Clone)]
pub struct CgroupPids {
    /// The number of processes (and threads) in the cgroup and its descendants (`pids.current`).
    pub current: u64,
    /// The maximum number of processes (`pids.max`). It is `None` when there is no limit.
    pub max:     Option<u64>,
}

/// Get the number of processes in the cgroup at `path` by reading the `pids.current` file and the `pids.max` file in it. The files do not exist in the root cgroup, or when the PID controller is not enabled for the cgroup, so a `NotFound` error is returned then.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let cgroup_pids =
///     cgroup::get_cgroup_pids(cgroup::get_cgroup_path().unwrap()).unwrap();
///
/// println!("{cgroup_pids:#?}");
/// ```
pub fn get_cgroup_pids<P: AsRef<Path>>(path: P) -> Result<CgroupPids, Error> {
    let path = path.as_ref();

    let current = read_sysfs_number(path.join("pids.current"))?;
    let max = read_limit(path.join("pids.max"))?;

    Ok(CgroupPids {
        current,
        max,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_limits() {
        assert_eq!(None, parse_limit("max").unwrap());
        assert_eq!(Some(1073741824), parse_limit("1073741824").unwrap());

        assert_eq!(
            (None, Duration::from_millis(100)),
            parse_cpu_max(b"max 100000\n".as_slice()).unwrap()
        );
        assert_eq!(
            (Some(Duration::from_millis(50)), Duration::from_millis(100)),
            parse_cpu_max(b"50000 100000\n".as_slice()).unwrap()
        );
    }

    #[test]
    fn parse_stat() {
        const CPU_STAT: &[u8] = b"usage_usec 572351147
user_usec 360521046
system_usec 211830100
nice_usec 31000
nr_periods 0
nr_throttled 3
throttled_usec 1500
";

        let mut cpu = CgroupCPU::default();

        parse_cpu_stat(CPU_STAT, &mut cpu).unwrap();

        assert_eq!(Duration::from_micros(572351147), cpu.usage);
        assert_eq!(Duration::from_micros(360521046), cpu.user);
        assert_eq!(Duration::from_micros(211830100), cpu.system);
        assert_eq!(3, cpu.nr_throttled);
        assert_eq!(Duration::from_micros(1500), cpu.throttled);
    }

    #[test]
    fn effective_cpu_count() {
        let cpu = CgroupCPU {
            quota: Some(Duration::from_millis(150)),
            period: Duration::from_millis(100),
            ..CgroupCPU::default()
        };

        assert_eq!(Some(1.5), cpu.effective_cpu_count());
        assert_eq!(None, CgroupCPU::default().effective_cpu_count());
    }
}
