use std::{
    collections::BTreeMap,
    fs,
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{
        read_link_name, read_sysfs_micro, read_sysfs_milli, read_sysfs_number, read_sysfs_string,
    },
};

/// One temperature sensor of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Temperature {
    /// The label of the sensor, e.g. `Package id 0` or `Composite`.
    pub label:    Option<String>,
    /// The current temperature in degrees Celsius.
    pub current:  f64,
    /// The high threshold in degrees Celsius.
    pub max:      Option<f64>,
    /// The critical threshold in degrees Celsius.
    pub critical: Option<f64>,
}

/// One fan of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fan {
    /// The label of the fan.
    pub label: Option<String>,
    /// The fan speed in RPM.
    pub rpm:   u32,
}

/// One voltage sensor of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Voltage {
    /// The label of the sensor, e.g. `Vcore` or `+12V`.
    pub label:   Option<String>,
    /// The current voltage in volts.
    pub current: f64,
    /// The low threshold in volts.
    pub min:     Option<f64>,
    /// The high threshold in volts.
    pub max:     Option<f64>,
}

/// One power sensor of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Power {
    /// The label of the sensor, e.g. `PPT` or `package`.
    pub label:   Option<String>,
    /// The current power in watts. It comes from `powerN_input`, or from `powerN_average` when the driver only reports an average.
    pub current: f64,
    /// The power cap in watts.
    pub cap:     Option<f64>,
}

/// One current sensor of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Current {
    /// The label of the sensor.
    pub label:   Option<String>,
    /// The current in amperes.
    pub current: f64,
    /// The low threshold in amperes.
    pub min:     Option<f64>,
    /// The high threshold in amperes.
    pub max:     Option<f64>,
}

/// One humidity sensor of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Humidity {
    /// The label of the sensor.
    pub label:   Option<String>,
    /// The relative humidity in percent.
    pub current: f64,
}

/// One hardware monitoring device under `/sys/class/hwmon`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HwmonDevice {
    /// The name of the chip or driver, e.g. `coretemp` or `nvme`.
    pub name:         String,
    /// The name of the underlying device in sysfs, e.g. `nvme0` or `coretemp.0`, which tells devices with the same `name` apart. It is `None` when the driver registers no device.
    pub device:       Option<String>,
    /// The temperature sensors of this device, ordered by their sensor number.
    pub temperatures: Vec<Temperature>,
    /// The fans of this device, ordered by their sensor number.
    pub fans:         Vec<Fan>,
    /// The voltage sensors of this device, ordered by their sensor number.
    pub voltages:     Vec<Voltage>,
    /// The power sensors of this device, ordered by their sensor number.
    pub powers:       Vec<Power>,
    /// The current sensors of this device, ordered by their sensor number.
    pub currents:     Vec<Current>,
    /// The humidity sensors of this device, ordered by their sensor number.
    pub humidities:   Vec<Humidity>,
}

// Which files a sensor has, taken from the folder listing so that an attribute the driver does not provide costs no `open` at all. The kinds do not use the same set of suffixes, so a flag that is meaningless for a kind is simply never set for it.
const HAS_INPUT: u8 = 1 << 0;
const HAS_AVERAGE: u8 = 1 << 1;
const HAS_LABEL: u8 = 1 << 2;
const HAS_MIN: u8 = 1 << 3;
const HAS_MAX: u8 = 1 << 4;
const HAS_CRIT: u8 = 1 << 5;
const HAS_CAP: u8 = 1 << 6;

/// Split a sensor file name like `temp1_input` into its kind, its index and its suffix. A file that is not named this way (e.g. `name` or `update_interval`) returns `None`.
fn split_sensor_file_name(file_name: &str) -> Option<(&str, usize, &str)> {
    let (name, suffix) = file_name.split_at(file_name.find('_')?);

    let (kind, index) = name.split_at(name.find(|c: char| c.is_ascii_digit())?);

    Some((kind, index.parse().ok()?, suffix))
}

/// Map a file name suffix to its flag. A suffix this crate does not read (e.g. `_alarm` or `_crit_alarm`) returns `None`.
#[inline]
fn suffix_flag(suffix: &str) -> Option<u8> {
    let flag = match suffix {
        "_input" => HAS_INPUT,
        "_average" => HAS_AVERAGE,
        "_label" => HAS_LABEL,
        "_min" => HAS_MIN,
        "_max" => HAS_MAX,
        "_crit" => HAS_CRIT,
        "_cap" => HAS_CAP,
        _ => return None,
    };

    Some(flag)
}

/// The sensors of one device with the files each of them has. A `BTreeMap` keeps them in the order of their index, which the folder listing does not have.
#[derive(Default)]
struct SensorIndices {
    temperatures: BTreeMap<usize, u8>,
    fans:         BTreeMap<usize, u8>,
    voltages:     BTreeMap<usize, u8>,
    powers:       BTreeMap<usize, u8>,
    currents:     BTreeMap<usize, u8>,
    humidities:   BTreeMap<usize, u8>,
}

fn scan_sensor_indices(device_path: &Path) -> Option<SensorIndices> {
    let mut indices = SensorIndices::default();

    // The device may be removed while it is being scanned, so an unreadable entry is skipped.
    for entry in fs::read_dir(device_path).ok()? {
        let Ok(entry) = entry else {
            continue;
        };

        let file_name = entry.file_name();

        let Some(file_name) = file_name.to_str() else {
            continue;
        };

        let Some((kind, index, suffix)) = split_sensor_file_name(file_name) else {
            continue;
        };

        let Some(flag) = suffix_flag(suffix) else {
            continue;
        };

        let sensors = match kind {
            "temp" => &mut indices.temperatures,
            "fan" => &mut indices.fans,
            "in" => &mut indices.voltages,
            "power" => &mut indices.powers,
            "curr" => &mut indices.currents,
            "humidity" => &mut indices.humidities,
            _ => continue,
        };

        *sensors.entry(index).or_default() |= flag;
    }

    Some(indices)
}

/// Read an attribute only when the folder listing showed that it exists.
#[inline]
fn read_present<T, F: FnOnce() -> Option<T>>(flags: u8, flag: u8, read: F) -> Option<T> {
    if flags & flag == 0 {
        return None;
    }

    read()
}

fn read_device(device_path: &Path) -> Option<HwmonDevice> {
    // A device without a name cannot be identified, so it is skipped.
    let name = read_sysfs_string(device_path.join("name")).ok()?;

    let device = read_link_name(device_path.join("device"));

    let indices = scan_sensor_indices(device_path)?;

    let mut temperatures = Vec::with_capacity(indices.temperatures.len());

    for (index, flags) in indices.temperatures {
        // A sensor whose input cannot be read (e.g. `ENODATA`) is skipped.
        let Some(current) = read_present(flags, HAS_INPUT, || {
            read_sysfs_milli(device_path.join(format!("temp{index}_input")))
        }) else {
            continue;
        };

        temperatures.push(Temperature {
            label: read_present(flags, HAS_LABEL, || {
                read_sysfs_string(device_path.join(format!("temp{index}_label"))).ok()
            }),
            current,
            max: read_present(flags, HAS_MAX, || {
                read_sysfs_milli(device_path.join(format!("temp{index}_max")))
            }),
            critical: read_present(flags, HAS_CRIT, || {
                read_sysfs_milli(device_path.join(format!("temp{index}_crit")))
            }),
        });
    }

    let mut fans = Vec::with_capacity(indices.fans.len());

    for (index, flags) in indices.fans {
        let Some(rpm) = read_present(flags, HAS_INPUT, || {
            read_sysfs_number::<u32, _>(device_path.join(format!("fan{index}_input"))).ok()
        }) else {
            continue;
        };

        fans.push(Fan {
            label: read_present(flags, HAS_LABEL, || {
                read_sysfs_string(device_path.join(format!("fan{index}_label"))).ok()
            }),
            rpm,
        });
    }

    let mut voltages = Vec::with_capacity(indices.voltages.len());

    for (index, flags) in indices.voltages {
        let Some(current) = read_present(flags, HAS_INPUT, || {
            read_sysfs_milli(device_path.join(format!("in{index}_input")))
        }) else {
            continue;
        };

        voltages.push(Voltage {
            label: read_present(flags, HAS_LABEL, || {
                read_sysfs_string(device_path.join(format!("in{index}_label"))).ok()
            }),
            current,
            min: read_present(flags, HAS_MIN, || {
                read_sysfs_milli(device_path.join(format!("in{index}_min")))
            }),
            max: read_present(flags, HAS_MAX, || {
                read_sysfs_milli(device_path.join(format!("in{index}_max")))
            }),
        });
    }

    let mut powers = Vec::with_capacity(indices.powers.len());

    for (index, flags) in indices.powers {
        let Some(current) = read_present(flags, HAS_INPUT, || {
            read_sysfs_micro(device_path.join(format!("power{index}_input")))
        })
        .or_else(|| {
            read_present(flags, HAS_AVERAGE, || {
                read_sysfs_micro(device_path.join(format!("power{index}_average")))
            })
        }) else {
            continue;
        };

        powers.push(Power {
            label: read_present(flags, HAS_LABEL, || {
                read_sysfs_string(device_path.join(format!("power{index}_label"))).ok()
            }),
            current,
            cap: read_present(flags, HAS_CAP, || {
                read_sysfs_micro(device_path.join(format!("power{index}_cap")))
            }),
        });
    }

    let mut currents = Vec::with_capacity(indices.currents.len());

    for (index, flags) in indices.currents {
        // The driver reports a current in milliamperes.
        let Some(current) = read_present(flags, HAS_INPUT, || {
            read_sysfs_milli(device_path.join(format!("curr{index}_input")))
        }) else {
            continue;
        };

        currents.push(Current {
            label: read_present(flags, HAS_LABEL, || {
                read_sysfs_string(device_path.join(format!("curr{index}_label"))).ok()
            }),
            current,
            min: read_present(flags, HAS_MIN, || {
                read_sysfs_milli(device_path.join(format!("curr{index}_min")))
            }),
            max: read_present(flags, HAS_MAX, || {
                read_sysfs_milli(device_path.join(format!("curr{index}_max")))
            }),
        });
    }

    let mut humidities = Vec::with_capacity(indices.humidities.len());

    for (index, flags) in indices.humidities {
        // The driver reports a humidity in thousandths of a percent.
        let Some(current) = read_present(flags, HAS_INPUT, || {
            read_sysfs_milli(device_path.join(format!("humidity{index}_input")))
        }) else {
            continue;
        };

        humidities.push(Humidity {
            label: read_present(flags, HAS_LABEL, || {
                read_sysfs_string(device_path.join(format!("humidity{index}_label"))).ok()
            }),
            current,
        });
    }

    Some(HwmonDevice {
        name,
        device,
        temperatures,
        fans,
        voltages,
        powers,
        currents,
        humidities,
    })
}

/// Get the sensors of one hardware monitoring device by reading files in the `/sys/class/hwmon/hwmonN` folder, where `N` is `hwmon`. A `NotFound` error is returned when the device does not exist.
///
/// Reading a sensor goes to the hardware, and the devices differ by orders of magnitude: an NVMe disk or a wireless adapter answers in milliseconds, while `coretemp` answers in microseconds. A caller that only wants one device should read that one instead of paying for every device with [`get_hwmon_devices`].
///
/// ```rust,no_run
/// use mprober_lib::hwmon;
///
/// let hwmon_device = hwmon::get_hwmon_device(0).unwrap();
///
/// println!("{hwmon_device:#?}");
/// ```
pub fn get_hwmon_device(hwmon: usize) -> Result<HwmonDevice, Error> {
    let path = Path::new("/sys/class/hwmon").join(format!("hwmon{hwmon}"));

    read_device(&path).ok_or_else(|| io::Error::from(ErrorKind::NotFound).into())
}

/// Get the sensors of all hardware monitoring devices by reading files in the `/sys/class/hwmon` folder, like the `sensors` command. The devices are ordered by their numbers. Thermal zones also appear here through the `thermal_hwmon` bridge. Reading a sensor can be slow on some hardware, so this function should not be called at a high frequency; [`get_hwmon_device`] reads a single device when the others are not needed.
///
/// ```rust
/// use mprober_lib::hwmon;
///
/// let hwmon_devices = hwmon::get_hwmon_devices().unwrap();
///
/// println!("{hwmon_devices:#?}");
/// ```
pub fn get_hwmon_devices() -> Result<Vec<HwmonDevice>, Error> {
    let read_dir = match fs::read_dir("/sys/class/hwmon") {
        Ok(read_dir) => read_dir,
        // A kernel without hwmon support simply has no devices.
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let mut hwmons: Vec<usize> = Vec::new();

    for entry in read_dir {
        let entry = entry?;

        let file_name = entry.file_name();

        let Some(hwmon) = file_name
            .to_str()
            .and_then(|file_name| file_name.strip_prefix("hwmon"))
            .and_then(|hwmon| hwmon.parse::<usize>().ok())
        else {
            continue;
        };

        hwmons.push(hwmon);
    }

    // The directory order is arbitrary, so the devices are sorted by their number.
    hwmons.sort_unstable();

    let mut devices = Vec::with_capacity(hwmons.len());

    for hwmon in hwmons {
        // A device can be removed while the folder is being scanned.
        if let Ok(device) = get_hwmon_device(hwmon) {
            devices.push(device);
        }
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_sensor_file_names() {
        assert_eq!(Some(("temp", 1, "_input")), split_sensor_file_name("temp1_input"));
        assert_eq!(Some(("in", 0, "_label")), split_sensor_file_name("in0_label"));
        assert_eq!(Some(("power", 12, "_average")), split_sensor_file_name("power12_average"));

        // A file that names no sensor, and one whose suffix this crate does not read.
        assert_eq!(None, split_sensor_file_name("name"));
        assert_eq!(None, split_sensor_file_name("update_interval"));
        assert_eq!(Some(("temp", 1, "_crit_alarm")), split_sensor_file_name("temp1_crit_alarm"));
        assert_eq!(None, suffix_flag("_crit_alarm"));
    }
}
