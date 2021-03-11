use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::str::from_utf8_unchecked;

use crate::scanner_rust::generic_array::typenum::U1024;
use crate::scanner_rust::{ScannerAscii, ScannerError};

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
pub struct CPU {
    pub physical_id: usize,
    pub model_name: String,
    pub cpus_mhz: Vec<f64>,
    pub siblings: usize,
    pub cpu_cores: usize,
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

/// Get CPU information by reading the `/proc/cpuinfo` file.
///
/// ```rust
/// extern crate mprober_lib;
///
/// use mprober_lib::cpu;
///
/// let cpus = cpu::get_cpus().unwrap();
///
/// println!("{:#?}", cpus);
/// ```
pub fn get_cpus() -> Result<Vec<CPU>, ScannerError> {
    const USEFUL_ITEMS: [&[u8]; 5] =
        [b"model name", b"cpu MHz", b"physical id", b"siblings", b"cpu cores"];
    const MODEL_NAME_INDEX: usize = 0;
    const CPU_MHZ_INDEX: usize = 1;
    const PHYSICAL_ID_INDEX: usize = 2;
    const SIBLINGS_INDEX: usize = 3;
    const CPU_CORES: usize = 4;

    let mut sc: ScannerAscii<_, U1024> = ScannerAscii::scan_path2("/proc/cpuinfo")?;

    let mut cpus = Vec::with_capacity(1);
    let mut physical_ids: BTreeSet<usize> = BTreeSet::new();

    let mut physical_id = 0;
    let mut model_name = String::new();
    let mut cpus_mhz = Vec::with_capacity(1);
    let mut siblings = 0;
    let mut cpu_cores = 0;

    'outer: loop {
        'item: for (i, &item) in USEFUL_ITEMS.iter().enumerate() {
            let item_len = item.len();

            loop {
                match sc.next_line_raw()? {
                    Some(line) => {
                        if line.starts_with(item) {
                            let colon_index = line[item_len..]
                                .iter()
                                .copied()
                                .position(|b| b == b':')
                                .ok_or(ErrorKind::InvalidData)?;

                            let value = unsafe {
                                from_utf8_unchecked(&line[(item_len + colon_index + 1)..])
                            }
                            .trim();

                            match i {
                                MODEL_NAME_INDEX => {
                                    if model_name.is_empty() {
                                        model_name.push_str(value);
                                    }
                                }
                                CPU_MHZ_INDEX => {
                                    cpus_mhz.push(value.parse()?);
                                }
                                PHYSICAL_ID_INDEX => {
                                    physical_id = value.parse()?;

                                    if physical_ids.contains(&physical_id) {
                                        break 'item;
                                    }
                                }
                                SIBLINGS_INDEX => {
                                    siblings = value.parse()?;
                                }
                                CPU_CORES => {
                                    cpu_cores = value.parse()?;

                                    break 'item;
                                }
                                _ => unreachable!(),
                            }

                            break;
                        }
                    }
                    None => {
                        if i == MODEL_NAME_INDEX {
                            break 'outer;
                        } else {
                            return Err(ErrorKind::UnexpectedEof.into());
                        }
                    }
                }
            }
        }

        if siblings == cpus_mhz.len() {
            let cpu = CPU {
                physical_id,
                model_name,
                cpus_mhz,
                siblings,
                cpu_cores,
            };

            cpus.push(cpu);
            physical_ids.insert(physical_id);

            physical_id = 0;
            model_name = String::new();
            cpus_mhz = Vec::with_capacity(1);
            siblings = 0;
            cpu_cores = 0;
        }

        loop {
            let line_length = sc.drop_next_line()?;

            match line_length {
                Some(line_length) => {
                    if line_length == 0 {
                        break;
                    }
                }
                None => {
                    break 'outer;
                }
            }
        }
    }

    Ok(cpus)
}
