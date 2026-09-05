use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{Hash, Hasher},
    mem::take,
};

use crate::{
    Error,
    utils::{parse_number, read_file, read_sysfs_number},
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
pub struct CPU {
    pub physical_id: usize,
    /// The model name, which is empty on platforms that do not report it (e.g. ARM).
    pub model_name:  String,
    /// The current frequency of each logical processor in MHz.
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
    model_name: String,
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

    if builder.model_name.is_empty()
        && let Some(model_name) = block.model_name
    {
        builder.model_name = String::from_utf8_lossy(model_name).into_owned();
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

/// Parse the content of `/proc/cpuinfo`. Each CPU is returned with the numbers of its logical processors.
fn parse_cpuinfo(data: &[u8]) -> Result<Vec<(CPU, Vec<usize>)>, Error> {
    let mut builders: BTreeMap<usize, CPUBuilder> = BTreeMap::new();

    let mut block = ProcessorBlock::default();

    for line in data.split(|&b| b == b'\n') {
        let Some(colon_index) = line.iter().position(|&b| b == b':') else {
            continue;
        };

        let key = line[..colon_index].trim_ascii();
        let value = line[(colon_index + 1)..].trim_ascii();

        match key {
            b"processor" => {
                // A `processor` line starts a new block, so the previous block is complete now.
                flush_processor_block(take(&mut block), &mut builders);

                block.processor = Some(parse_number(value)?);
            },
            b"model name" => block.model_name = Some(value),
            b"cpu MHz" => block.mhz = Some(parse_number(value)?),
            b"physical id" => block.physical_id = Some(parse_number(value)?),
            b"siblings" => block.siblings = Some(parse_number(value)?),
            b"cpu cores" => block.cpu_cores = Some(parse_number(value)?),
            b"core id" => block.core_id = Some(parse_number(value)?),
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

    Ok(cpus)
}

/// Get CPU information by reading the `/proc/cpuinfo` file. If the file does not report `cpu MHz` (e.g. on ARM), the frequencies are read from the `/sys/devices/system/cpu/cpuN/cpufreq/scaling_cur_freq` files instead.
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

    let cpus = parse_cpuinfo(&data)?;

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
        let cpus = parse_cpuinfo(X86_CPUINFO).unwrap();

        assert_eq!(2, cpus.len());

        let (cpu, processors) = &cpus[0];

        assert_eq!(0, cpu.physical_id);
        assert_eq!("Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz", cpu.model_name);
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
        let cpus = parse_cpuinfo(AARCH64_CPUINFO).unwrap();

        assert_eq!(1, cpus.len());

        let (cpu, processors) = &cpus[0];

        assert_eq!(0, cpu.physical_id);
        assert_eq!("", cpu.model_name);
        assert!(cpu.cpus_mhz.is_empty());
        assert_eq!(2, cpu.siblings);
        assert_eq!(2, cpu.cpu_cores);
        assert_eq!(&vec![0, 1], processors);
    }
}
