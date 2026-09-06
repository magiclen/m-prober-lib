use std::{
    io::{self, ErrorKind},
    num::ParseIntError,
    path::{Path, PathBuf},
    time::Duration,
};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    pressure::{Pressure, parse_pressure},
    utils::{
        is_single_path_component, parse_cpu_list, parse_number, proc_pid_path, read_file,
        read_single_record_file, read_sysfs_number, read_sysfs_string,
    },
};

/// Find the root and the mount point of the cgroup v2 file system in the content of a `mountinfo` file. Only these two fields are extracted, because building every [`crate::volume::MountInfo`] just to find one line would cost far more.
fn find_cgroup2_mount(data: &[u8]) -> Result<Option<(String, String)>, Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        // The mount ID, the parent ID and the device numbers come before the root. A line with too few fields is skipped instead of ending the search.
        let (Some(_), Some(_), Some(_), Some(root), Some(point)) =
            (sc.next()?, sc.next()?, sc.next()?, sc.next()?, sc.next()?)
        else {
            continue;
        };

        // The options and a variable number of optional fields follow, terminated by a single hyphen.
        let mut fs_type = None;

        while let Some(field) = sc.next()? {
            if field == b"-" {
                fs_type = sc.next()?;

                break;
            }
        }

        if fs_type == Some(b"cgroup2".as_slice()) {
            return Ok(Some((
                String::from_utf8_lossy(root).into_owned(),
                String::from_utf8_lossy(point).into_owned(),
            )));
        }
    }

    Ok(None)
}

/// Join a path from a `/proc/PID/cgroup` file to the mount point of the cgroup file system, after stripping the root of the mount from it. It returns `None` when the path is not inside that root.
fn join_cgroup_mount(cgroup_path: &str, root: &str, point: &str) -> Option<PathBuf> {
    let root = root.trim_start_matches('/');

    if root.is_empty() {
        return Some(Path::new(point).join(cgroup_path));
    }

    let rest = cgroup_path.strip_prefix(root)?;

    // The prefix only counts when it ends where a path component ends, so that `docker-abcd` is not taken for a path inside `docker-abc`.
    if !rest.is_empty() && !rest.starts_with('/') {
        return None;
    }

    Some(Path::new(point).join(rest.trim_start_matches('/')))
}

/// Map a path from a `/proc/PID/cgroup` file to the path it has in the mount namespace of the current process.
fn resolve_cgroup_path(cgroup_path: &str) -> PathBuf {
    let cgroup_path = cgroup_path.trim_start_matches('/');

    let direct = Path::new("/sys/fs/cgroup").join(cgroup_path);

    // On a host the cgroup file system is mounted at its own root, so the path is directly below the usual mount point. Checking that with one `stat` keeps the mount table out of the common case.
    if direct.exists() {
        return direct;
    }

    // The path in `/proc/PID/cgroup` is relative to the cgroup namespace of the process, while the mount only exposes the subtree below its own root, so that prefix has to be stripped. They differ when the cgroup namespace is the host one but the mount is not, e.g. in a container started with `--cgroupns=host`.
    if let Ok(data) = read_file("/proc/self/mountinfo", 8 * 1024)
        && let Ok(Some((root, point))) = find_cgroup2_mount(&data)
        && let Some(path) = join_cgroup_mount(cgroup_path, &root, &point)
    {
        return path;
    }

    direct
}

/// Find the cgroup v2 line in the content of a `/proc/PID/cgroup` file and resolve it to a path in the mount namespace of the current process.
fn find_cgroup2_path(data: &[u8]) -> Result<PathBuf, Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    // The cgroup v2 line looks like `0::/user.slice/user-1000.slice`, while a cgroup v1 line has a controller list between the colons instead.
    while let Some(line) = lines.next_line()? {
        if let Some(path) = line.strip_prefix(b"0::") {
            let path = String::from_utf8_lossy(path.trim_ascii_end());

            return Ok(resolve_cgroup_path(&path));
        }
    }

    Err(io::Error::new(ErrorKind::NotFound, "the process is not in a cgroup v2 hierarchy").into())
}

/// Get the path of the cgroup (v2) that a specific process found by ID belongs to, by reading the `/proc/PID/cgroup` file. A `NotFound` error is returned when the process is not in a cgroup v2 hierarchy.
///
/// ```rust
/// use mprober_lib::cgroup;
///
/// let cgroup_path = cgroup::get_process_cgroup_path(1).unwrap();
///
/// println!("{}", cgroup_path.display());
/// ```
pub fn get_process_cgroup_path(pid: u32) -> Result<PathBuf, Error> {
    let cgroup_file = proc_pid_path(pid).join("cgroup");

    find_cgroup2_path(&read_single_record_file(cgroup_file, 512)?)
}

/// Get the path of the cgroup (v2) that the current process belongs to, by reading the `/proc/self/cgroup` file. The path is inside the cgroup v2 mount, e.g. `/sys/fs/cgroup/user.slice/user-1000.slice` on a host, or `/sys/fs/cgroup` itself in a container. A `NotFound` error is returned when the process is not in a cgroup v2 hierarchy.
///
/// ```rust
/// use mprober_lib::cgroup;
///
/// let cgroup_path = cgroup::get_cgroup_path().unwrap();
///
/// println!("{}", cgroup_path.display());
/// ```
pub fn get_cgroup_path() -> Result<PathBuf, Error> {
    find_cgroup2_path(&read_single_record_file("/proc/self/cgroup", 512)?)
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CgroupMemory {
    /// The memory used by the cgroup and its descendants in bytes (`memory.current`).
    pub current:      u64,
    /// The highest memory usage recorded in bytes (`memory.peak`). It is `None` on kernels older than 6.8.
    pub peak:         Option<u64>,
    /// The hard memory limit in bytes (`memory.max`). It is `None` when there is no limit.
    pub max:          Option<u64>,
    /// The throttling limit in bytes (`memory.high`), above which the cgroup is put under heavy reclaim pressure instead of being killed. It is `None` when there is no limit.
    pub high:         Option<u64>,
    /// The best-effort protection in bytes (`memory.low`), below which the memory of the cgroup is not reclaimed while another cgroup can be reclaimed instead. It is `None` when the protection has no bound, which `0` would otherwise be indistinguishable from.
    pub low:          Option<u64>,
    /// The hard protection in bytes (`memory.min`), below which the memory of the cgroup is never reclaimed. It is `None` when the protection has no bound.
    pub min:          Option<u64>,
    /// The swap used by the cgroup and its descendants in bytes (`memory.swap.current`). It is `None` when swap accounting is disabled, which is not the same as no swap being used.
    pub swap_current: Option<u64>,
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

    // `memory.peak` exists since Linux 6.8.
    let peak = read_sysfs_number(path.join("memory.peak")).ok();

    // A protection file holds `max` when everything is protected, which the limit files spell the same way.
    let high = read_limit(path.join("memory.high")).unwrap_or(None);
    let low = read_limit(path.join("memory.low")).unwrap_or(None);
    let min = read_limit(path.join("memory.min")).unwrap_or(None);

    // The swap files do not exist when swap accounting is disabled.
    let swap_current = read_sysfs_number(path.join("memory.swap.current")).ok();
    let swap_max = read_limit(path.join("memory.swap.max")).unwrap_or(None);

    Ok(CgroupMemory {
        current,
        peak,
        max,
        high,
        low,
        min,
        swap_current,
        swap_max,
    })
}

/// The memory events of a cgroup, read from the `memory.events` file. Every field counts how often the event happened since the cgroup was created.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CgroupMemoryEvents {
    /// How often the cgroup was reclaimed below its `memory.low` protection.
    pub low:            u64,
    /// How often the cgroup was throttled because it went over its `memory.high` limit.
    pub high:           u64,
    /// How often the cgroup went over its `memory.max` limit, which usually leads to an OOM kill.
    pub max:            u64,
    /// How often the OOM killer was invoked for the cgroup.
    pub oom:            u64,
    /// How many processes of the cgroup the OOM killer actually killed. This is the authoritative answer to whether a container was killed for using too much memory.
    pub oom_kill:       u64,
    /// How often a whole cgroup was killed at once because `memory.oom.group` was set. It is `None` on kernels older than 5.17, which do not report it.
    pub oom_group_kill: Option<u64>,
}

/// Parse the content of `memory.events`, which is one `key value` pair per line.
fn parse_memory_events(data: &[u8]) -> Result<CgroupMemoryEvents, Error> {
    let mut events = CgroupMemoryEvents::default();

    let mut sc = ScannerU8SliceAscii::new(data);

    while let Some(key) = sc.next()? {
        let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        match key {
            b"low" => events.low = value,
            b"high" => events.high = value,
            b"max" => events.max = value,
            b"oom" => events.oom = value,
            b"oom_kill" => events.oom_kill = value,
            b"oom_group_kill" => events.oom_group_kill = Some(value),
            _ => (),
        }
    }

    Ok(events)
}

/// Get the memory events of the cgroup at `path` by reading the `memory.events` file in it. The file does not exist in the root cgroup, or when the memory controller is not enabled for the cgroup, so a `NotFound` error is returned then.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let memory_events =
///     cgroup::get_cgroup_memory_events(cgroup::get_cgroup_path().unwrap())
///         .unwrap();
///
/// println!("{memory_events:#?}");
/// ```
#[inline]
pub fn get_cgroup_memory_events<P: AsRef<Path>>(path: P) -> Result<CgroupMemoryEvents, Error> {
    parse_memory_events(&read_single_record_file(path.as_ref().join("memory.events"), 256)?)
}

/// The memory of a cgroup broken down by kind, read from the `memory.stat` file. Every field is in bytes unless its documentation says otherwise, and every field counts the cgroup and its descendants together.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CgroupMemoryStat {
    /// The anonymous memory, which is the heap and the stacks of the processes (`anon`).
    pub anon:               u64,
    /// The page cache, which holds file contents (`file`). The kernel can reclaim most of it, so it is not memory the processes really need.
    pub file:               u64,
    /// The memory the kernel uses for the cgroup in total (`kernel`), which includes the stacks, the page tables, the slab and the socket buffers. It is `None` on kernels older than 5.18.
    pub kernel:             Option<u64>,
    /// The memory of the kernel stacks of the tasks (`kernel_stack`).
    pub kernel_stack:       u64,
    /// The memory of the page tables (`pagetables`). It is `None` on kernels older than 5.13.
    pub pagetables:         Option<u64>,
    /// The memory the per-CPU allocator uses (`percpu`).
    pub percpu:             u64,
    /// The memory of the socket buffers (`sock`).
    pub sock:               u64,
    /// The memory of `tmpfs` and of shared anonymous mappings (`shmem`), which is part of `file`.
    pub shmem:              u64,
    /// The page cache that is mapped into page tables (`file_mapped`).
    pub file_mapped:        u64,
    /// The page cache that is waiting to be written back (`file_dirty`).
    pub file_dirty:         u64,
    /// The page cache that is being written back right now (`file_writeback`).
    pub file_writeback:     u64,
    /// The anonymous memory that is backed by transparent huge pages (`anon_thp`).
    pub anon_thp:           u64,
    /// The anonymous memory that has not been used recently (`inactive_anon`).
    pub inactive_anon:      u64,
    /// The anonymous memory that has been used recently (`active_anon`).
    pub active_anon:        u64,
    /// The page cache that has not been used recently (`inactive_file`), which the kernel reclaims first. `memory.current - inactive_file` is what the `docker stats` command shows as the memory usage of a container.
    pub inactive_file:      u64,
    /// The page cache that has been used recently (`active_file`).
    pub active_file:        u64,
    /// The memory that cannot be reclaimed, e.g. because it is locked (`unevictable`).
    pub unevictable:        u64,
    /// The part of the slab that can be reclaimed (`slab_reclaimable`).
    pub slab_reclaimable:   u64,
    /// The part of the slab that cannot be reclaimed (`slab_unreclaimable`).
    pub slab_unreclaimable: u64,
    /// The memory the kernel uses for its own data structures (`slab`).
    pub slab:               u64,
    /// The number of page faults (`pgfault`). This is a count, not a size.
    pub page_faults:        u64,
    /// The number of major page faults, which needed disk I/O (`pgmajfault`). This is a count, not a size.
    pub major_page_faults:  u64,
}

/// Parse the content of `memory.stat`, which is one `key value` pair per line.
fn parse_memory_stat(data: &[u8]) -> Result<CgroupMemoryStat, Error> {
    let mut stat = CgroupMemoryStat::default();

    let mut sc = ScannerU8SliceAscii::new(data);

    while let Some(key) = sc.next()? {
        let value = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        match key {
            b"anon" => stat.anon = value,
            b"file" => stat.file = value,
            b"kernel" => stat.kernel = Some(value),
            b"kernel_stack" => stat.kernel_stack = value,
            b"pagetables" => stat.pagetables = Some(value),
            b"percpu" => stat.percpu = value,
            b"sock" => stat.sock = value,
            b"shmem" => stat.shmem = value,
            b"file_mapped" => stat.file_mapped = value,
            b"file_dirty" => stat.file_dirty = value,
            b"file_writeback" => stat.file_writeback = value,
            b"anon_thp" => stat.anon_thp = value,
            b"inactive_anon" => stat.inactive_anon = value,
            b"active_anon" => stat.active_anon = value,
            b"inactive_file" => stat.inactive_file = value,
            b"active_file" => stat.active_file = value,
            b"unevictable" => stat.unevictable = value,
            b"slab_reclaimable" => stat.slab_reclaimable = value,
            b"slab_unreclaimable" => stat.slab_unreclaimable = value,
            b"slab" => stat.slab = value,
            b"pgfault" => stat.page_faults = value,
            b"pgmajfault" => stat.major_page_faults = value,
            _ => (),
        }
    }

    Ok(stat)
}

/// Get the memory of the cgroup at `path` broken down by kind, by reading the `memory.stat` file in it. The `current` of [`get_cgroup_memory`] includes the page cache, so this is the way to tell how much of the memory of a container is anonymous memory and how much is cache the kernel could drop. The file does not exist when the memory controller is not enabled for the cgroup, so a `NotFound` error is returned then.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let memory_stat =
///     cgroup::get_cgroup_memory_stat(cgroup::get_cgroup_path().unwrap())
///         .unwrap();
///
/// println!("{memory_stat:#?}");
/// ```
#[inline]
pub fn get_cgroup_memory_stat<P: AsRef<Path>>(path: P) -> Result<CgroupMemoryStat, Error> {
    parse_memory_stat(&read_single_record_file(path.as_ref().join("memory.stat"), 2048)?)
}

/// The CPU usage and limits of a cgroup, read from the `cpu.max` file and the `cpu.stat` file.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CgroupCPU {
    /// The CPU time the cgroup may use in every `period` (`cpu.max`). It is `None` when there is no limit.
    pub quota:        Option<Duration>,
    /// The length of one accounting period (`cpu.max`), which the kernel defaults to `100ms`. It is `None` when the `cpu.max` file does not exist, so this reports the period the kernel actually holds rather than assuming the default.
    pub period:       Option<Duration>,
    /// The total CPU time used (`usage_usec`).
    pub usage:        Duration,
    /// The CPU time used in user mode (`user_usec`).
    pub user:         Duration,
    /// The CPU time used in kernel mode (`system_usec`).
    pub system:       Duration,
    /// The number of accounting periods that have passed (`nr_periods`). Its ratio to `nr_throttled` tells how often the quota ran out. It is `0` when the CPU controller is not enabled for the cgroup, in which case nothing can be throttled.
    pub nr_periods:   u64,
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
        let period = self.period?;

        if period.is_zero() {
            return None;
        }

        // Both values are whole microseconds in `cpu.max`, so dividing them as such keeps the result exact.
        Some(quota.as_micros() as f64 / period.as_micros() as f64)
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
            b"nr_periods" => cpu.nr_periods = value,
            b"nr_throttled" => cpu.nr_throttled = value,
            b"throttled_usec" => cpu.throttled = Duration::from_micros(value),
            _ => (),
        }
    }

    Ok(())
}

/// Get the CPU usage and limits of the cgroup at `path` by reading the `cpu.max` file and the `cpu.stat` file in it. The `cpu.max` file does not exist in the root cgroup, or when the CPU controller is not enabled for the cgroup, which means there is no limit and leaves both `quota` and `period` as `None`.
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

    let mut cpu = CgroupCPU::default();

    match read_single_record_file(path.join("cpu.max"), 64) {
        Ok(data) => {
            let (quota, period) = parse_cpu_max(&data)?;

            cpu.quota = quota;
            cpu.period = Some(period);
        },
        Err(err) if err.kind() == ErrorKind::NotFound => (),
        Err(err) => return Err(err.into()),
    }

    parse_cpu_stat(&read_single_record_file(path.join("cpu.stat"), 512)?, &mut cpu)?;

    Ok(cpu)
}

/// The number of processes in a cgroup, read from the `pids.current` file and the `pids.max` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// The I/O counters of one block device inside a cgroup, read from the `io.stat` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CgroupIO {
    /// The major number of the device.
    pub major:         u32,
    /// The minor number of the device.
    pub minor:         u32,
    /// Bytes read (`rbytes`).
    pub read_bytes:    u64,
    /// Bytes written (`wbytes`).
    pub write_bytes:   u64,
    /// Read operations completed (`rios`).
    pub read_ios:      u64,
    /// Write operations completed (`wios`).
    pub write_ios:     u64,
    /// Bytes discarded (`dbytes`).
    pub discard_bytes: u64,
    /// Discard operations completed (`dios`).
    pub discard_ios:   u64,
}

/// Parse the content of `io.stat`, whose lines look like `259:0 rbytes=1024 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0`.
fn parse_io_stat(data: &[u8]) -> Result<Vec<CgroupIO>, Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    let mut ios = Vec::with_capacity(1);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        let Some(device) = sc.next()? else {
            continue;
        };

        // This field looks like `259:0`.
        let mut device = ScannerU8SliceAscii::new(device);

        let mut io = CgroupIO {
            major: device.next_u32_until(":")?.ok_or(io::Error::from(ErrorKind::InvalidData))?,
            minor: device.next_u32()?.ok_or(io::Error::from(ErrorKind::InvalidData))?,
            ..CgroupIO::default()
        };

        // The rest of the line is `key=value` pairs, and a device the kernel has no counter for has none at all.
        while let Some(token) = sc.next()? {
            let mut token = ScannerU8SliceAscii::new(token);

            let (Some(key), Some(value)) = (token.next_until("=")?, token.next_u64()?) else {
                continue;
            };

            match key {
                b"rbytes" => io.read_bytes = value,
                b"wbytes" => io.write_bytes = value,
                b"rios" => io.read_ios = value,
                b"wios" => io.write_ios = value,
                b"dbytes" => io.discard_bytes = value,
                b"dios" => io.discard_ios = value,
                _ => (),
            }
        }

        ios.push(io);
    }

    Ok(ios)
}

/// Get the per-device I/O counters of the cgroup at `path` by reading the `io.stat` file in it. The file does not exist when the I/O controller is not enabled for the cgroup, so a `NotFound` error is returned then. This is the only way to measure the disk I/O of a container, because `/proc/diskstats` counts the whole host.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let cgroup_io =
///     cgroup::get_cgroup_io(cgroup::get_cgroup_path().unwrap()).unwrap();
///
/// println!("{cgroup_io:#?}");
/// ```
#[inline]
pub fn get_cgroup_io<P: AsRef<Path>>(path: P) -> Result<Vec<CgroupIO>, Error> {
    parse_io_stat(&read_single_record_file(path.as_ref().join("io.stat"), 1024)?)
}

/// Get the PSI (Pressure Stall Information) of one resource of the cgroup at `path` by reading the `RESOURCE.pressure` file in it, e.g. `cpu`, `memory` or `io`. The file needs `CONFIG_PSI`, and it does not exist when the matching controller is not enabled for the cgroup, so a `NotFound` error is returned then.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let cpu_pressure =
///     cgroup::get_cgroup_pressure(cgroup::get_cgroup_path().unwrap(), "cpu")
///         .unwrap();
///
/// println!("{cpu_pressure:#?}");
/// ```
pub fn get_cgroup_pressure<P: AsRef<Path>, S: AsRef<str>>(
    path: P,
    resource: S,
) -> Result<Pressure, Error> {
    let resource = resource.as_ref();

    // The name must be a single path component, and a dot in it would name another file in the same folder.
    if !is_single_path_component(resource) || resource.contains('.') {
        return Err(io::Error::from(ErrorKind::InvalidInput).into());
    }

    parse_pressure(&read_single_record_file(
        path.as_ref().join(format!("{resource}.pressure")),
        256,
    )?)
}

/// The CPUs and the NUMA nodes a cgroup may use, read from the `cpuset.*` files.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CgroupCpuset {
    /// The CPUs requested for the cgroup (`cpuset.cpus`). It is empty when nothing is requested, which means the cgroup inherits from its parent.
    pub cpus:           Vec<usize>,
    /// The CPUs the cgroup may actually run on (`cpuset.cpus.effective`), which is what the parent cgroups leave it.
    pub cpus_effective: Vec<usize>,
    /// The NUMA nodes requested for the cgroup (`cpuset.mems`).
    pub mems:           Vec<usize>,
    /// The NUMA nodes the cgroup may actually allocate from (`cpuset.mems.effective`).
    pub mems_effective: Vec<usize>,
}

/// Get the CPUs and the NUMA nodes of the cgroup at `path` by reading the `cpuset.*` files in it. The files do not exist when the cpuset controller is not enabled for the cgroup, so a `NotFound` error is returned then. Unlike the quota of [`get_cgroup_cpu`], this is a hard restriction on which CPUs the cgroup runs on, e.g. what `docker run --cpuset-cpus` sets.
///
/// ```rust,no_run
/// use mprober_lib::cgroup;
///
/// let cpuset =
///     cgroup::get_cgroup_cpuset(cgroup::get_cgroup_path().unwrap()).unwrap();
///
/// println!("{cpuset:#?}");
/// ```
pub fn get_cgroup_cpuset<P: AsRef<Path>>(path: P) -> Result<CgroupCpuset, Error> {
    let path = path.as_ref();

    let cpus_effective = parse_cpu_list(&read_sysfs_string(path.join("cpuset.cpus.effective"))?);

    // Only the effective files are mandatory, because a cgroup that requests nothing has empty ones.
    let cpus = read_sysfs_string(path.join("cpuset.cpus"))
        .map(|list| parse_cpu_list(&list))
        .unwrap_or_default();

    let mems_effective = read_sysfs_string(path.join("cpuset.mems.effective"))
        .map(|list| parse_cpu_list(&list))
        .unwrap_or_default();

    let mems = read_sysfs_string(path.join("cpuset.mems"))
        .map(|list| parse_cpu_list(&list))
        .unwrap_or_default();

    Ok(CgroupCpuset {
        cpus,
        cpus_effective,
        mems,
        mems_effective,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_cgroup2_mount_after_a_short_line() {
        // The cgroup2 line comes after a truncated one, which must not end the search.
        const MOUNTINFO: &[u8] =
            b"23 28 0:22 / /proc rw,nosuid,nodev,noexec,relatime - proc proc rw
truncated
30 28 0:26 /subtree /sys/fs/cgroup rw,nosuid,nodev,noexec,relatime - cgroup2 cgroup2 rw,nsdelegate
";

        assert_eq!(
            Some((String::from("/subtree"), String::from("/sys/fs/cgroup"))),
            find_cgroup2_mount(MOUNTINFO).unwrap()
        );

        assert_eq!(None, find_cgroup2_mount(b"truncated\n").unwrap());
    }

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
nr_periods 120
nr_throttled 3
throttled_usec 1500
";

        let mut cpu = CgroupCPU::default();

        parse_cpu_stat(CPU_STAT, &mut cpu).unwrap();

        assert_eq!(Duration::from_micros(572351147), cpu.usage);
        assert_eq!(Duration::from_micros(360521046), cpu.user);
        assert_eq!(Duration::from_micros(211830100), cpu.system);
        assert_eq!(120, cpu.nr_periods);
        assert_eq!(3, cpu.nr_throttled);
        assert_eq!(Duration::from_micros(1500), cpu.throttled);
    }

    #[test]
    fn find_cgroup2_mount_point() {
        const MOUNT_INFO: &[u8] =
            b"27 34 0:24 / /sys rw,nosuid,nodev,noexec,relatime shared:7 - sysfs sysfs rw
35 27 0:28 / /sys/fs/cgroup rw,nosuid,nodev,noexec,relatime shared:9 - cgroup2 cgroup2 rw,nsdelegate
44 34 0:41 / /proc/sys/fs/binfmt_misc rw,relatime - autofs systemd-1 rw
";

        assert_eq!(
            Some((String::from("/"), String::from("/sys/fs/cgroup"))),
            find_cgroup2_mount(MOUNT_INFO).unwrap()
        );

        // A container started with `--cgroupns=host` only gets its own subtree mounted.
        const CONTAINER_MOUNT_INFO: &[u8] = b"1234 1233 0:28 /system.slice/docker-abc.scope /sys/fs/cgroup ro,nosuid,nodev,noexec,relatime - cgroup2 cgroup rw
";

        assert_eq!(
            Some((String::from("/system.slice/docker-abc.scope"), String::from("/sys/fs/cgroup"))),
            find_cgroup2_mount(CONTAINER_MOUNT_INFO).unwrap()
        );

        assert_eq!(None, find_cgroup2_mount(b"27 34 0:24 / /sys rw - sysfs sysfs rw\n").unwrap());
    }

    #[test]
    fn join_cgroup_mount_paths() {
        // A mount at its own root exposes every cgroup, so the path is joined as it is.
        assert_eq!(
            Some(PathBuf::from("/sys/fs/cgroup/user.slice")),
            join_cgroup_mount("user.slice", "/", "/sys/fs/cgroup")
        );

        // A container started with `--cgroupns=host` only gets its own subtree mounted, so that prefix is stripped.
        assert_eq!(
            Some(PathBuf::from("/sys/fs/cgroup")),
            join_cgroup_mount(
                "system.slice/docker-abc.scope",
                "/system.slice/docker-abc.scope",
                "/sys/fs/cgroup",
            )
        );
        assert_eq!(
            Some(PathBuf::from("/sys/fs/cgroup/init.scope")),
            join_cgroup_mount(
                "system.slice/docker-abc.scope/init.scope",
                "/system.slice/docker-abc.scope",
                "/sys/fs/cgroup",
            )
        );

        // A cgroup whose name only starts with the root is not inside it.
        assert_eq!(
            None,
            join_cgroup_mount(
                "system.slice/docker-abcd.scope",
                "/system.slice/docker-abc.scope",
                "/sys/fs/cgroup",
            )
        );
    }

    #[test]
    fn parse_events() {
        const MEMORY_EVENTS: &[u8] = b"low 0
high 12
max 3
oom 2
oom_kill 1
oom_group_kill 0
";

        let events = parse_memory_events(MEMORY_EVENTS).unwrap();

        assert_eq!(0, events.low);
        assert_eq!(12, events.high);
        assert_eq!(3, events.max);
        assert_eq!(2, events.oom);
        assert_eq!(1, events.oom_kill);
        assert_eq!(Some(0), events.oom_group_kill);

        // A kernel older than 5.17 does not write the last line.
        let events = parse_memory_events(b"low 0\nhigh 0\nmax 0\noom 0\noom_kill 0\n").unwrap();

        assert_eq!(None, events.oom_group_kill);
    }

    #[test]
    fn parse_memory_stats() {
        const MEMORY_STAT: &[u8] = b"anon 6118477824
file 4079632384
kernel 149110784
kernel_stack 12091392
pagetables 48627712
sec_pagetables 0
percpu 17472
sock 0
vmalloc 0
shmem 787709952
zswap 0
zswapped 0
file_mapped 131088384
file_dirty 8372224
file_writeback 0
swapcached 0
anon_thp 0
file_thp 0
shmem_thp 0
inactive_anon 0
active_anon 6905892864
inactive_file 1486483456
active_file 1805438976
unevictable 16384
slab_reclaimable 67204024
slab_unreclaimable 18989408
slab 86193432
workingset_refault_anon 0
workingset_refault_file 506542
pgfault 12345
pgmajfault 67
";

        let stat = parse_memory_stat(MEMORY_STAT).unwrap();

        assert_eq!(6118477824, stat.anon);
        assert_eq!(4079632384, stat.file);
        assert_eq!(Some(149110784), stat.kernel);
        assert_eq!(12091392, stat.kernel_stack);
        assert_eq!(Some(48627712), stat.pagetables);
        assert_eq!(787709952, stat.shmem);
        assert_eq!(8372224, stat.file_dirty);
        assert_eq!(1486483456, stat.inactive_file);
        assert_eq!(1805438976, stat.active_file);
        assert_eq!(16384, stat.unevictable);
        assert_eq!(86193432, stat.slab);
        assert_eq!(12345, stat.page_faults);
        assert_eq!(67, stat.major_page_faults);

        // A kernel older than 5.13 reports neither the page tables nor the kernel total.
        let stat = parse_memory_stat(b"anon 4096\nfile 8192\nkernel_stack 0\n").unwrap();

        assert_eq!(4096, stat.anon);
        assert_eq!(None, stat.kernel);
        assert_eq!(None, stat.pagetables);
    }

    #[test]
    fn parse_io() {
        const IO_STAT: &[u8] =
            b"259:1 rbytes=40202240 wbytes=10305536 rios=2368 wios=634 dbytes=0 dios=0
259:0 rbytes=8756880896 wbytes=25748984832 rios=141481 wios=1293837 dbytes=68519366656 dios=392811
7:7
";

        let ios = parse_io_stat(IO_STAT).unwrap();

        assert_eq!(3, ios.len());

        assert_eq!(259, ios[0].major);
        assert_eq!(1, ios[0].minor);
        assert_eq!(40202240, ios[0].read_bytes);
        assert_eq!(10305536, ios[0].write_bytes);
        assert_eq!(2368, ios[0].read_ios);
        assert_eq!(634, ios[0].write_ios);

        assert_eq!(68519366656, ios[1].discard_bytes);
        assert_eq!(392811, ios[1].discard_ios);

        // A device the kernel has no counter for is listed without any pair.
        assert_eq!(7, ios[2].major);
        assert_eq!(0, ios[2].read_bytes);
    }

    #[test]
    fn effective_cpu_count() {
        let cpu = CgroupCPU {
            quota: Some(Duration::from_millis(150)),
            period: Some(Duration::from_millis(100)),
            ..CgroupCPU::default()
        };

        assert_eq!(Some(1.5), cpu.effective_cpu_count());
        assert_eq!(None, CgroupCPU::default().effective_cpu_count());

        // Without a `cpu.max` file there is no period to divide by, so no count can be computed.
        let without_period = CgroupCPU {
            quota: Some(Duration::from_millis(150)),
            ..CgroupCPU::default()
        };

        assert_eq!(None, without_period.effective_cpu_count());
    }
}
