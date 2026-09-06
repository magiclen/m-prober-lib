use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    mem::take,
};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    cpu::cpu_topology::package_topology,
    utils::{parse_number, read_file, read_sysfs_number},
};

/// One physical CPU package, built from the `processor` blocks of the `/proc/cpuinfo` file that share a `physical id`. Two instances are equal when their `physical_id` are equal.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPU {
    /// The `physical id` of this package. It is `0` on platforms that do not report one.
    pub physical_id: usize,
    /// The model name, e.g. `Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz`. It is `None` on a platform whose `/proc/cpuinfo` file has no `model name` line (e.g. ARM, which only reports the implementer and the part number).
    pub model_name:  Option<String>,
    /// The current frequency of each logical processor in MHz. It is empty when neither the `/proc/cpuinfo` file nor cpufreq reports a frequency, e.g. in a virtual machine on ARM.
    pub cpus_mhz:    Vec<f64>,
    /// The number of logical processors in this package.
    pub siblings:    usize,
    /// The number of physical cores in this package.
    pub cpu_cores:   usize,
}

impl Hash for CPU {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.physical_id.hash(state)
    }
}

impl PartialEq for CPU {
    #[inline]
    fn eq(&self, other: &CPU) -> bool {
        self.physical_id.eq(&other.physical_id)
    }
}

// Only the `physical_id` is compared, which is a full equivalence relation. This cannot be derived, because deriving would demand `Eq` from every field, and `cpus_mhz` holds floats.
impl Eq for CPU {}

/// The fields of one `processor` block in `/proc/cpuinfo`. Every field is optional because the file differs between platforms.
#[derive(Default)]
struct ProcessorBlock<'a> {
    processor:   Option<usize>,
    model_name:  Option<&'a [u8]>,
    mhz:         Option<f64>,
    physical_id: Option<usize>,
    siblings:    Option<usize>,
    cpu_cores:   Option<usize>,
    core_id:     Option<usize>,
}

#[derive(Default)]
struct CPUBuilder {
    model_name: Option<String>,
    cpus_mhz:   Vec<f64>,
    processors: Vec<usize>,
    core_ids:   BTreeSet<usize>,
    siblings:   Option<usize>,
    cpu_cores:  Option<usize>,
}

fn flush_processor_block(block: ProcessorBlock, builders: &mut BTreeMap<usize, CPUBuilder>) {
    let Some(processor) = block.processor else {
        return;
    };

    // Platforms without `physical id` are treated as a single package.
    let builder = builders.entry(block.physical_id.unwrap_or(0)).or_default();

    if builder.model_name.is_none()
        && let Some(model_name) = block.model_name
    {
        builder.model_name = Some(String::from_utf8_lossy(model_name).into_owned());
    }

    if let Some(mhz) = block.mhz {
        builder.cpus_mhz.push(mhz);
    }

    builder.processors.push(processor);

    if let Some(core_id) = block.core_id {
        builder.core_ids.insert(core_id);
    }

    if block.siblings.is_some() {
        builder.siblings = block.siblings;
    }

    if block.cpu_cores.is_some() {
        builder.cpu_cores = block.cpu_cores;
    }
}

/// The result of parsing `/proc/cpuinfo`.
struct CPUInfo {
    /// Each CPU with the numbers of its logical processors.
    cpus:            Vec<(CPU, Vec<usize>)>,
    /// Whether the file reported a `physical id`, which platforms like ARM do not.
    has_physical_id: bool,
    /// Whether the file reported a `core id`, which platforms like ARM do not.
    has_core_id:     bool,
}

/// Parse the content of `/proc/cpuinfo`. Each CPU is returned with the numbers of its logical processors.
fn parse_cpuinfo(data: &[u8]) -> Result<CPUInfo, Error> {
    let mut builders: BTreeMap<usize, CPUBuilder> = BTreeMap::new();

    let mut has_physical_id = false;
    let mut has_core_id = false;

    let mut block = ProcessorBlock::default();

    let mut lines = ScannerU8SliceAscii::new(data);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        // A key can hold spaces, e.g. `model name`, so the colon is what ends it.
        let Some(key) = sc.next_until(":")? else {
            continue;
        };

        // The rest of the line is the value, which is taken without scanning it again. A line without a colon leaves nothing here, because reading up to a missing boundary consumes everything.
        let Some(value) = sc.next_bytes(line.len())? else {
            continue;
        };

        let key = key.trim_ascii();
        let value = value.trim_ascii();

        match key {
            b"processor" => {
                // A `processor` line starts a new block, so the previous block is complete now.
                flush_processor_block(take(&mut block), &mut builders);

                block.processor = Some(parse_number(value)?);
            },
            b"model name" => block.model_name = Some(value),
            b"cpu MHz" => block.mhz = Some(parse_number(value)?),
            b"physical id" => {
                has_physical_id = true;

                block.physical_id = Some(parse_number(value)?);
            },
            b"siblings" => block.siblings = Some(parse_number(value)?),
            b"cpu cores" => block.cpu_cores = Some(parse_number(value)?),
            b"core id" => {
                has_core_id = true;

                block.core_id = Some(parse_number(value)?);
            },
            _ => (),
        }
    }

    flush_processor_block(block, &mut builders);

    let cpus = builders
        .into_iter()
        .map(|(physical_id, builder)| {
            let siblings = builder.siblings.unwrap_or(builder.processors.len());

            let cpu_cores = builder
                .cpu_cores
                .or_else(|| (!builder.core_ids.is_empty()).then_some(builder.core_ids.len()))
                .unwrap_or(builder.processors.len());

            let cpu = CPU {
                physical_id,
                model_name: builder.model_name,
                cpus_mhz: builder.cpus_mhz,
                siblings,
                cpu_cores,
            };

            (cpu, builder.processors)
        })
        .collect();

    Ok(CPUInfo {
        cpus,
        has_physical_id,
        has_core_id,
    })
}

/// Rebuild the packages from the sysfs topology, which is the only source on a platform whose `/proc/cpuinfo` reports no `physical id`. The model name and the frequencies of the single package the parsing produced are spread over the real packages.
fn regroup_by_sysfs_topology(cpus: &[(CPU, Vec<usize>)]) -> Option<Vec<(CPU, Vec<usize>)>> {
    let packages = package_topology()?;

    // Only a file without `physical id` reaches this point, so the parsing produced exactly one package.
    let (template, _) = cpus.first()?;

    let mut result = Vec::with_capacity(packages.len());

    for (physical_id, (processors, core_ids)) in packages {
        let cpu = CPU {
            physical_id,
            model_name: template.model_name.clone(),
            cpus_mhz: Vec::new(),
            siblings: processors.len(),
            cpu_cores: core_ids.len(),
        };

        result.push((cpu, processors));
    }

    Some(result)
}

/// Get CPU information by reading the `/proc/cpuinfo` file. If the file does not report `cpu MHz` (e.g. on ARM), the frequencies are read from the `/sys/devices/system/cpu/cpuN/cpufreq/scaling_cur_freq` files instead, and if it reports neither `physical id` nor `core id`, the packages and the cores come from the `/sys/devices/system/cpu/cpuN/topology` folders.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let cpus = cpu::get_cpus().unwrap();
///
/// println!("{cpus:#?}");
/// ```
pub fn get_cpus() -> Result<Vec<CPU>, Error> {
    let data = read_file("/proc/cpuinfo", 64 * 1024)?;

    let cpu_info = parse_cpuinfo(&data)?;

    let mut cpus = cpu_info.cpus;

    if !cpu_info.has_physical_id
        && !cpu_info.has_core_id
        && let Some(regrouped) = regroup_by_sysfs_topology(&cpus)
    {
        cpus = regrouped;
    }

    let mut result = Vec::with_capacity(cpus.len());

    for (mut cpu, processors) in cpus {
        if cpu.cpus_mhz.is_empty() {
            for processor in processors {
                let path =
                    format!("/sys/devices/system/cpu/cpu{processor}/cpufreq/scaling_cur_freq");

                if let Ok(khz) = read_sysfs_number::<u64, _>(path) {
                    cpu.cpus_mhz.push(khz as f64 / 1000.0);
                }
            }
        }

        result.push(cpu);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    const X86_CPUINFO: &[u8] = b"processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
cpu MHz\t\t: 2400.000
physical id\t: 0
siblings\t: 2
core id\t\t: 0
cpu cores\t: 1
flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush
power management:

processor\t: 1
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
cpu MHz\t\t: 2500.000
physical id\t: 0
siblings\t: 2
core id\t\t: 0
cpu cores\t: 1
flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush
power management:

processor\t: 2
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
cpu MHz\t\t: 2600.000
physical id\t: 1
siblings\t: 2
core id\t\t: 0
cpu cores\t: 1
flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush
power management:

processor\t: 3
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
cpu MHz\t\t: 2700.000
physical id\t: 1
siblings\t: 2
core id\t\t: 0
cpu cores\t: 1
flags\t\t: fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush
power management:

";

    const AARCH64_CPUINFO: &[u8] = b"processor\t: 0
BogoMIPS\t: 108.00
Features\t: fp asimd evtstrm crc32 cpuid
CPU implementer\t: 0x41
CPU architecture: 8
CPU variant\t: 0x0
CPU part\t: 0xd08
CPU revision\t: 3

processor\t: 1
BogoMIPS\t: 108.00
Features\t: fp asimd evtstrm crc32 cpuid
CPU implementer\t: 0x41
CPU architecture: 8
CPU variant\t: 0x0
CPU part\t: 0xd08
CPU revision\t: 3

Hardware\t: BCM2835
Revision\t: c03114
Model\t\t: Raspberry Pi 4 Model B Rev 1.4
";

    #[test]
    fn parse_x86() {
        let cpu_info = parse_cpuinfo(X86_CPUINFO).unwrap();

        assert!(cpu_info.has_physical_id);
        assert!(cpu_info.has_core_id);

        let cpus = cpu_info.cpus;

        assert_eq!(2, cpus.len());

        let (cpu, processors) = &cpus[0];

        assert_eq!(0, cpu.physical_id);
        assert_eq!(Some("Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz"), cpu.model_name.as_deref());
        assert_eq!(vec![2400.0, 2500.0], cpu.cpus_mhz);
        assert_eq!(2, cpu.siblings);
        assert_eq!(1, cpu.cpu_cores);
        assert_eq!(&vec![0, 1], processors);

        let (cpu, processors) = &cpus[1];

        assert_eq!(1, cpu.physical_id);
        assert_eq!(vec![2600.0, 2700.0], cpu.cpus_mhz);
        assert_eq!(&vec![2, 3], processors);
    }

    #[test]
    fn parse_aarch64() {
        let cpu_info = parse_cpuinfo(AARCH64_CPUINFO).unwrap();

        // This file reports neither, so `get_cpus` falls back to the sysfs topology.
        assert!(!cpu_info.has_physical_id);
        assert!(!cpu_info.has_core_id);

        let cpus = cpu_info.cpus;

        assert_eq!(1, cpus.len());

        let (cpu, processors) = &cpus[0];

        assert_eq!(0, cpu.physical_id);
        assert_eq!(None, cpu.model_name);
        assert!(cpu.cpus_mhz.is_empty());
        assert_eq!(2, cpu.siblings);
        assert_eq!(2, cpu.cpu_cores);
        assert_eq!(&vec![0, 1], processors);
    }
}
