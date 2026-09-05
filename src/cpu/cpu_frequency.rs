use std::path::Path;

use crate::{
    Error,
    cpu::get_online_cpus,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// The frequency and the governor of one logical processor, read from the `/sys/devices/system/cpu/cpuN/cpufreq` folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPUFrequency {
    /// The current frequency in MHz (`scaling_cur_freq`).
    pub current_mhz:     f64,
    /// The lowest frequency the hardware supports in MHz (`cpuinfo_min_freq`).
    pub min_mhz:         f64,
    /// The highest frequency the hardware supports in MHz (`cpuinfo_max_freq`).
    pub max_mhz:         f64,
    /// The lowest frequency the governor may pick in MHz (`scaling_min_freq`), which the administrator can raise above `min_mhz`.
    pub scaling_min_mhz: f64,
    /// The highest frequency the governor may pick in MHz (`scaling_max_freq`), which the administrator can lower below `max_mhz`.
    pub scaling_max_mhz: f64,
    /// The governor in use, e.g. `powersave`, `performance` or `schedutil` (`scaling_governor`).
    pub governor:        String,
    /// The cpufreq driver in use, e.g. `intel_pstate`, `acpi-cpufreq` or `cppc_cpufreq` (`scaling_driver`).
    pub driver:          String,
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

    // A driver without a policy range reports only the hardware limits.
    let scaling_min_mhz = read_sysfs_number::<u64, _>(path.join("scaling_min_freq"))
        .map(|khz| khz as f64 / 1000.0)
        .unwrap_or(min_mhz);
    let scaling_max_mhz = read_sysfs_number::<u64, _>(path.join("scaling_max_freq"))
        .map(|khz| khz as f64 / 1000.0)
        .unwrap_or(max_mhz);

    let governor = read_sysfs_string(path.join("scaling_governor"))?;

    let driver = read_sysfs_string(path.join("scaling_driver")).unwrap_or_default();

    Ok(CPUFrequency {
        current_mhz,
        min_mhz,
        max_mhz,
        scaling_min_mhz,
        scaling_max_mhz,
        governor,
        driver,
    })
}

/// Get the frequency information of every online logical processor by reading files in the `/sys/devices/system/cpu` folder. A processor without a cpufreq folder is not included, so the result is empty on a platform without a cpufreq driver.
///
/// ```rust
/// use mprober_lib::cpu;
///
/// let cpu_frequencies = cpu::get_all_cpu_frequencies().unwrap();
///
/// println!("{cpu_frequencies:#?}");
/// ```
pub fn get_all_cpu_frequencies() -> Result<Vec<(usize, CPUFrequency)>, Error> {
    let cpus = get_online_cpus()?;

    let mut frequencies = Vec::with_capacity(cpus.len());

    for cpu in cpus {
        // A processor can go offline while the folder is being scanned.
        if let Ok(frequency) = get_cpu_frequency(cpu) {
            frequencies.push((cpu, frequency));
        }
    }

    Ok(frequencies)
}

/// How often a logical processor was forced to slow down, read from the `/sys/devices/system/cpu/cpuN/thermal_throttle` folder.
#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CPUThermalThrottle {
    /// How often this core was throttled because it got too hot.
    pub core_throttle_count:    u64,
    /// How often the whole package was throttled because it got too hot. It is `None` when the platform reports no package counter.
    pub package_throttle_count: Option<u64>,
}

/// Get the thermal throttling counters of a logical processor by reading files in the `/sys/devices/system/cpu/cpuN/thermal_throttle` folder. Only x86 reports these, so a `NotFound` error is returned on other platforms. A counter that keeps growing means the cooling cannot keep up, which shows up as a frequency far below `max_mhz`.
///
/// ```rust,no_run
/// use mprober_lib::cpu;
///
/// let throttle = cpu::get_cpu_thermal_throttle(0).unwrap();
///
/// println!("{throttle:#?}");
/// ```
pub fn get_cpu_thermal_throttle(cpu: usize) -> Result<CPUThermalThrottle, Error> {
    let path = Path::new("/sys/devices/system/cpu").join(format!("cpu{cpu}/thermal_throttle"));

    let core_throttle_count = read_sysfs_number(path.join("core_throttle_count"))?;

    let package_throttle_count = read_sysfs_number(path.join("package_throttle_count")).ok();

    Ok(CPUThermalThrottle {
        core_throttle_count,
        package_throttle_count,
    })
}
