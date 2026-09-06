use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
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

/// Extract `N` from a file name like `tempN_input`.
#[inline]
fn parse_sensor_index(file_name: &str, kind: &str, suffix: &str) -> Option<usize> {
    file_name.strip_prefix(kind)?.strip_suffix(suffix)?.parse().ok()
}

#[derive(Default)]
struct SensorIndices {
    temperatures: Vec<usize>,
    fans:         Vec<usize>,
    voltages:     Vec<usize>,
    powers:       Vec<usize>,
    currents:     Vec<usize>,
    humidities:   Vec<usize>,
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

        if let Some(index) = parse_sensor_index(file_name, "temp", "_input") {
            indices.temperatures.push(index);
        } else if let Some(index) = parse_sensor_index(file_name, "fan", "_input") {
            indices.fans.push(index);
        } else if let Some(index) = parse_sensor_index(file_name, "curr", "_input") {
            indices.currents.push(index);
        } else if let Some(index) = parse_sensor_index(file_name, "humidity", "_input") {
            indices.humidities.push(index);
        } else if let Some(index) = parse_sensor_index(file_name, "in", "_input") {
            indices.voltages.push(index);
        } else if let Some(index) = parse_sensor_index(file_name, "power", "_input")
            .or_else(|| parse_sensor_index(file_name, "power", "_average"))
        {
            indices.powers.push(index);
        }
    }

    // The directory order is arbitrary, so the sensors are sorted by their index.
    indices.temperatures.sort_unstable();
    indices.fans.sort_unstable();
    indices.voltages.sort_unstable();
    indices.powers.sort_unstable();
    indices.currents.sort_unstable();
    indices.humidities.sort_unstable();

    // A power sensor may have both an `_input` file and an `_average` file.
    indices.powers.dedup();

    Some(indices)
}

fn read_device(device_path: &Path) -> Option<HwmonDevice> {
    // A device without a name cannot be identified, so it is skipped.
    let name = read_sysfs_string(device_path.join("name")).ok()?;

    let device = read_link_name(device_path.join("device"));

    let indices = scan_sensor_indices(device_path)?;

    let mut temperatures = Vec::with_capacity(indices.temperatures.len());

    for index in indices.temperatures {
        // A sensor whose input cannot be read (e.g. `ENODATA`) is skipped.
        let Some(current) = read_sysfs_milli(device_path.join(format!("temp{index}_input"))) else {
            continue;
        };

        temperatures.push(Temperature {
            label: read_sysfs_string(device_path.join(format!("temp{index}_label"))).ok(),
            current,
            max: read_sysfs_milli(device_path.join(format!("temp{index}_max"))),
            critical: read_sysfs_milli(device_path.join(format!("temp{index}_crit"))),
        });
    }

    let mut fans = Vec::with_capacity(indices.fans.len());

    for index in indices.fans {
        let Ok(rpm) = read_sysfs_number::<u32, _>(device_path.join(format!("fan{index}_input")))
        else {
            continue;
        };

        fans.push(Fan {
            label: read_sysfs_string(device_path.join(format!("fan{index}_label"))).ok(),
            rpm,
        });
    }

    let mut voltages = Vec::with_capacity(indices.voltages.len());

    for index in indices.voltages {
        let Some(current) = read_sysfs_milli(device_path.join(format!("in{index}_input"))) else {
            continue;
        };

        voltages.push(Voltage {
            label: read_sysfs_string(device_path.join(format!("in{index}_label"))).ok(),
            current,
            min: read_sysfs_milli(device_path.join(format!("in{index}_min"))),
            max: read_sysfs_milli(device_path.join(format!("in{index}_max"))),
        });
    }

    let mut powers = Vec::with_capacity(indices.powers.len());

    for index in indices.powers {
        let Some(current) = read_sysfs_micro(device_path.join(format!("power{index}_input")))
            .or_else(|| read_sysfs_micro(device_path.join(format!("power{index}_average"))))
        else {
            continue;
        };

        powers.push(Power {
            label: read_sysfs_string(device_path.join(format!("power{index}_label"))).ok(),
            current,
            cap: read_sysfs_micro(device_path.join(format!("power{index}_cap"))),
        });
    }

    let mut currents = Vec::with_capacity(indices.currents.len());

    for index in indices.currents {
        // The driver reports a current in milliamperes.
        let Some(current) = read_sysfs_milli(device_path.join(format!("curr{index}_input"))) else {
            continue;
        };

        currents.push(Current {
            label: read_sysfs_string(device_path.join(format!("curr{index}_label"))).ok(),
            current,
            min: read_sysfs_milli(device_path.join(format!("curr{index}_min"))),
            max: read_sysfs_milli(device_path.join(format!("curr{index}_max"))),
        });
    }

    let mut humidities = Vec::with_capacity(indices.humidities.len());

    for index in indices.humidities {
        // The driver reports a humidity in thousandths of a percent.
        let Some(current) = read_sysfs_milli(device_path.join(format!("humidity{index}_input")))
        else {
            continue;
        };

        humidities.push(Humidity {
            label: read_sysfs_string(device_path.join(format!("humidity{index}_label"))).ok(),
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

/// Get the sensors of all hardware monitoring devices by reading files in the `/sys/class/hwmon` folder, like the `sensors` command. Thermal zones also appear here through the `thermal_hwmon` bridge. Reading a sensor can be slow on some hardware, so this function should not be called at a high frequency.
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

    let mut device_paths: Vec<(usize, PathBuf)> = Vec::new();

    for entry in read_dir {
        let entry = entry?;

        let file_name = entry.file_name();

        let Some(index) = file_name
            .to_str()
            .and_then(|file_name| file_name.strip_prefix("hwmon"))
            .and_then(|index| index.parse::<usize>().ok())
        else {
            continue;
        };

        device_paths.push((index, entry.path()));
    }

    device_paths.sort_unstable_by_key(|(index, _)| *index);

    let mut devices = Vec::with_capacity(device_paths.len());

    for (_, device_path) in device_paths {
        if let Some(device) = read_device(&device_path) {
            devices.push(device);
        }
    }

    Ok(devices)
}
