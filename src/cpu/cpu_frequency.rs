use std::path::Path;

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// The frequency and the governor of one logical processor, read from the `/sys/devices/system/cpu/cpuN/cpufreq` folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
pub struct CPUFrequency {
    /// The current frequency in MHz (`scaling_cur_freq`).
    pub current_mhz: f64,
    /// The lowest frequency the hardware supports in MHz (`cpuinfo_min_freq`).
    pub min_mhz:     f64,
    /// The highest frequency the hardware supports in MHz (`cpuinfo_max_freq`).
    pub max_mhz:     f64,
    /// The governor in use, e.g. `powersave`, `performance` or `schedutil` (`scaling_governor`).
    pub governor:    String,
}

/// Get the frequency information of a logical processor by reading files in the `/sys/devices/system/cpu/cpuN/cpufreq` folder. The folder does not exist when the kernel has no cpufreq driver for the platform (e.g. in many virtual machines), so a `NotFound` error is returned then.
///
/// ```rust,no_run
/// use mprober_lib::cpu;
///
/// let cpu_frequency = cpu::get_cpu_frequency(0).unwrap();
///
/// println!("{cpu_frequency:#?}");
/// ```
pub fn get_cpu_frequency(cpu: usize) -> Result<CPUFrequency, Error> {
    let path = Path::new("/sys/devices/system/cpu").join(format!("cpu{cpu}/cpufreq"));

    // The frequencies are in kHz.
    let current_mhz = read_sysfs_number::<u64, _>(path.join("scaling_cur_freq"))? as f64 / 1000.0;
    let min_mhz = read_sysfs_number::<u64, _>(path.join("cpuinfo_min_freq"))? as f64 / 1000.0;
    let max_mhz = read_sysfs_number::<u64, _>(path.join("cpuinfo_max_freq"))? as f64 / 1000.0;

    let governor = read_sysfs_string(path.join("scaling_governor"))?;

    Ok(CPUFrequency {
        current_mhz,
        min_mhz,
        max_mhz,
        governor,
    })
}
