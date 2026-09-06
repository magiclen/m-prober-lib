use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{parse_cpu_list, read_cpu_list, read_sysfs_number, read_sysfs_string},
};

/// The topology of one logical processor, read from the `/sys/devices/system/cpu/cpuN/topology` folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPUTopology {
    /// The number of this logical processor, which is the `N` of `cpuN`.
    pub cpu:                 usize,
    /// The physical package (socket) this processor belongs to.
    pub physical_package_id: usize,
    /// The physical core this processor belongs to, which is only unique within a package.
    pub core_id:             usize,
    /// The die this processor belongs to. It is `None` on a platform that does not report one.
    pub die_id:              Option<usize>,
    /// The cluster this processor belongs to, e.g. one group of an ARM big.LITTLE design. It is `None` on a platform that does not report one.
    pub cluster_id:          Option<usize>,
    /// Every processor that shares the physical core with this one, including this one. More than one means the core is hyper-threaded.
    pub thread_siblings:     Vec<usize>,
    /// Every processor in the same package, including this one.
    pub core_siblings:       Vec<usize>,
}

/// Read a topology ID that the kernel writes as `-1` when it does not know it.
#[inline]
fn read_optional_id<P: AsRef<Path>>(path: P) -> Option<usize> {
    read_sysfs_number::<i64, _>(path).ok().and_then(|id| usize::try_from(id).ok())
}

/// Get the numbers of the logical processors that are currently online, by reading the `/sys/devices/system/cpu/online` file.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let online_cpus = cpu::get_online_cpus().unwrap();
///
/// println!("{online_cpus:?}");
/// ```
#[inline]
pub fn get_online_cpus() -> Result<Vec<usize>, Error> {
    Ok(parse_cpu_list(&read_sysfs_string("/sys/devices/system/cpu/online")?))
}

/// Get the topology of a logical processor by reading files in the `/sys/devices/system/cpu/cpuN/topology` folder. This is the authoritative source for the core and package layout, unlike the `/proc/cpuinfo` file, which reports neither `physical id` nor `core id` on many platforms (e.g. ARM).
///
/// ```rust,no_run
/// use mprober_lib::cpu;
///
/// let cpu_topology = cpu::get_cpu_topology(0).unwrap();
///
/// println!("{cpu_topology:#?}");
/// ```
pub fn get_cpu_topology(cpu: usize) -> Result<CPUTopology, Error> {
    let path = Path::new("/sys/devices/system/cpu").join(format!("cpu{cpu}/topology"));

    let physical_package_id = read_optional_id(path.join("physical_package_id"))
        .ok_or(io::Error::from(ErrorKind::NotFound))?;

    let core_id = read_optional_id(path.join("core_id")).unwrap_or(0);

    Ok(CPUTopology {
        cpu,
        physical_package_id,
        core_id,
        die_id: read_optional_id(path.join("die_id")),
        cluster_id: read_optional_id(path.join("cluster_id")),
        thread_siblings: read_cpu_list(path.join("thread_siblings_list")),
        core_siblings: read_cpu_list(path.join("core_siblings_list")),
    })
}

/// Get the topology of every online logical processor by reading files in the `/sys/devices/system/cpu` folder. A processor whose topology the kernel does not report is not included.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let cpu_topologies = cpu::get_all_cpu_topologies().unwrap();
///
/// println!("{cpu_topologies:#?}");
/// ```
pub fn get_all_cpu_topologies() -> Result<Vec<CPUTopology>, Error> {
    let cpus = get_online_cpus()?;

    let mut topologies = Vec::with_capacity(cpus.len());

    for cpu in cpus {
        // A processor can go offline while the folder is being scanned, and a virtual machine may expose no topology at all.
        if let Ok(topology) = get_cpu_topology(cpu) {
            topologies.push(topology);
        }
    }

    Ok(topologies)
}

/// The processor numbers and the core IDs of one physical package.
pub(crate) type Package = (Vec<usize>, BTreeSet<usize>);

/// Group the online logical processors by their physical package, using sysfs. It returns `None` when sysfs reports no topology at all.
pub(crate) fn package_topology() -> Option<BTreeMap<usize, Package>> {
    let cpus = get_online_cpus().ok()?;

    let mut packages: BTreeMap<usize, Package> = BTreeMap::new();

    for cpu in cpus {
        let path = Path::new("/sys/devices/system/cpu").join(format!("cpu{cpu}/topology"));

        // A package is made of these two attributes alone, so the four other files of the folder are left unread. A processor that reports no package (it can go offline while the folder is being scanned, and a virtual machine may expose no topology at all) is skipped, like in `get_all_cpu_topologies`.
        let Some(physical_package_id) = read_optional_id(path.join("physical_package_id")) else {
            continue;
        };

        let core_id = read_optional_id(path.join("core_id")).unwrap_or(0);

        let package = packages.entry(physical_package_id).or_default();

        package.0.push(cpu);
        package.1.insert(core_id);
    }

    if packages.is_empty() {
        return None;
    }

    Some(packages)
}
