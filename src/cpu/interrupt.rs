use crate::{
    Error,
    utils::{parse_number, read_file},
};

/// One interrupt source and how often each CPU serviced it, read from the `/proc/interrupts` file or the `/proc/softirqs` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Interrupt {
    /// The name of the source, e.g. `0`, `NMI` or `LOC` for a hardware interrupt, or `TIMER` and `NET_RX` for a softirq.
    pub name:    String,
    /// How often each online CPU serviced this source, in the order of the `cpuN` columns.
    pub counts:  Vec<u64>,
    /// The chip and the device the interrupt belongs to, e.g. `IR-PCI-MSI 1572864-edge nvme0q0`. It is empty for the named counters at the end of the file and for every softirq.
    pub details: String,
}

impl Interrupt {
    /// Add up how often every CPU serviced this source.
    #[inline]
    pub fn total(&self) -> u64 {
        self.counts.iter().sum()
    }
}

/// Parse the content of `/proc/interrupts` or `/proc/softirqs`. Both have a header line of CPU names, and then one line per source that starts with a label ending in a colon.
fn parse_interrupts(data: &[u8], cpu_count: usize) -> Result<Vec<Interrupt>, Error> {
    let mut interrupts = Vec::with_capacity(32);

    // The first line only names the columns.
    for line in data.split(|&b| b == b'\n').skip(1) {
        let mut tokens = line.split(|b| b.is_ascii_whitespace()).filter(|token| !token.is_empty());

        let Some(label) = tokens.next() else {
            continue;
        };

        // Every source line starts with `NAME:`.
        let Some(name) = label.strip_suffix(b":") else {
            continue;
        };

        let name = String::from_utf8_lossy(name).into_owned();

        let mut counts = Vec::with_capacity(cpu_count);

        // The counters run out before the trailing description, which starts at the first token that is not a number.
        let mut details_start = None;

        for token in tokens {
            match parse_number::<u64>(token) {
                Ok(count) if counts.len() < cpu_count => counts.push(count),
                _ => {
                    details_start = Some(token);

                    break;
                },
            }
        }

        // The description is taken from the raw line so that its inner spacing is not lost.
        let details = match details_start {
            Some(token) => {
                let offset = token.as_ptr() as usize - line.as_ptr() as usize;

                String::from_utf8_lossy(line[offset..].trim_ascii_end()).into_owned()
            },
            None => String::new(),
        };

        interrupts.push(Interrupt {
            name,
            counts,
            details,
        });
    }

    Ok(interrupts)
}

/// Count the `cpuN` columns of the header line.
fn parse_cpu_columns(data: &[u8]) -> usize {
    let Some(header) = data.split(|&b| b == b'\n').next() else {
        return 0;
    };

    header.split(|b| b.is_ascii_whitespace()).filter(|token| token.starts_with(b"CPU")).count()
}

/// Get the per-CPU counters of every hardware interrupt by reading the `/proc/interrupts` file. Unlike the `intr` total of [`crate::cpu::get_cpu_activity`], this tells which device is generating the interrupts and whether they are spread over the CPUs.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let interrupts = cpu::get_interrupt_stats().unwrap();
///
/// for interrupt in interrupts {
///     println!("{}: {}", interrupt.name, interrupt.total());
/// }
/// ```
pub fn get_interrupt_stats() -> Result<Vec<Interrupt>, Error> {
    let data = read_file("/proc/interrupts", 64 * 1024)?;

    let cpu_count = parse_cpu_columns(&data);

    parse_interrupts(&data, cpu_count)
}

/// Get the per-CPU counters of every softirq by reading the `/proc/softirqs` file. The sources are the fixed kernel ones, e.g. `TIMER`, `NET_TX`, `NET_RX`, `BLOCK`, `SCHED` and `RCU`.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let softirqs = cpu::get_softirq_stats().unwrap();
///
/// for softirq in softirqs {
///     println!("{}: {}", softirq.name, softirq.total());
/// }
/// ```
pub fn get_softirq_stats() -> Result<Vec<Interrupt>, Error> {
    let data = read_file("/proc/softirqs", 8 * 1024)?;

    let cpu_count = parse_cpu_columns(&data);

    parse_interrupts(&data, cpu_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERRUPTS: &[u8] = b"           CPU0       CPU1       CPU2       CPU3
  0:         24          0          0          0  IR-IO-APIC    2-edge      timer
  9:          0          1          0          2  IR-IO-APIC    9-fasteoi   acpi
133:       1024       2048          0          0  IR-PCI-MSI 1572864-edge      nvme0q0
NMI:          3          4          5          6  Non-maskable interrupts
LOC:     369205     226079     312832     219915  Local timer interrupts
ERR:          0
";

    const SOFTIRQS: &[u8] = b"                    CPU0       CPU1       CPU2       CPU3
          HI:          0          0          3          3
       TIMER:     369205     226079     312832     219915
      NET_RX:         12         34         56         78
";

    #[test]
    fn parse_hardware_interrupts() {
        assert_eq!(4, parse_cpu_columns(INTERRUPTS));

        let interrupts = parse_interrupts(INTERRUPTS, 4).unwrap();

        assert_eq!(6, interrupts.len());

        assert_eq!("0", interrupts[0].name);
        assert_eq!(vec![24, 0, 0, 0], interrupts[0].counts);
        assert_eq!(24, interrupts[0].total());
        assert_eq!("IR-IO-APIC    2-edge      timer", interrupts[0].details);

        assert_eq!("133", interrupts[2].name);
        assert_eq!(3072, interrupts[2].total());
        assert_eq!("IR-PCI-MSI 1572864-edge      nvme0q0", interrupts[2].details);

        // A named counter has a description but no chip.
        assert_eq!("NMI", interrupts[3].name);
        assert_eq!(vec![3, 4, 5, 6], interrupts[3].counts);
        assert_eq!("Non-maskable interrupts", interrupts[3].details);

        // `ERR` has a single counter and no description at all.
        assert_eq!("ERR", interrupts[5].name);
        assert_eq!(vec![0], interrupts[5].counts);
        assert_eq!("", interrupts[5].details);
    }

    #[test]
    fn parse_softirqs() {
        assert_eq!(4, parse_cpu_columns(SOFTIRQS));

        let softirqs = parse_interrupts(SOFTIRQS, 4).unwrap();

        assert_eq!(3, softirqs.len());

        assert_eq!("HI", softirqs[0].name);
        assert_eq!(6, softirqs[0].total());

        assert_eq!("NET_RX", softirqs[2].name);
        assert_eq!(vec![12, 34, 56, 78], softirqs[2].counts);
        assert_eq!("", softirqs[2].details);
    }
}
