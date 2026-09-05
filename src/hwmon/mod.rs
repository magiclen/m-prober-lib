use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// One temperature sensor of a hardware monitoring device.
#[derive(Default, Debug, Clone)]
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
pub struct Fan {
    /// The label of the fan.
    pub label: Option<String>,
    /// The fan speed in RPM.
    pub rpm:   u32,
}

/// One hardware monitoring device under `/sys/class/hwmon`.
#[derive(Default, Debug, Clone)]
pub struct HwmonDevice {
    /// The name of the chip or driver, e.g. `coretemp` or `nvme`.
    pub name:         String,
    /// The temperature sensors of this device, ordered by their sensor number.
    pub temperatures: Vec<Temperature>,
    /// The fans of this device, ordered by their sensor number.
    pub fans:         Vec<Fan>,
}

/// Extract `N` from a file name like `tempN_input`.
#[inline]
fn parse_sensor_index(file_name: &str, kind: &str) -> Option<usize> {
    file_name.strip_prefix(kind)?.strip_suffix("_input")?.parse().ok()
}

#[inline]
fn read_millidegree<P: AsRef<Path>>(path: P) -> Option<f64> {
    read_sysfs_number::<i64, _>(path).ok().map(|value| value as f64 / 1000.0)
}

fn read_sensors(device_path: &Path) -> Option<(Vec<Temperature>, Vec<Fan>)> {
    let mut temperature_indices = Vec::new();
    let mut fan_indices = Vec::new();

    // The device may be removed while it is being scanned, so an unreadable entry is skipped.
    for entry in fs::read_dir(device_path).ok()? {
        let Ok(entry) = entry else {
            continue;
        };

        let file_name = entry.file_name();

        let Some(file_name) = file_name.to_str() else {
            continue;
        };

        if let Some(index) = parse_sensor_index(file_name, "temp") {
            temperature_indices.push(index);
        } else if let Some(index) = parse_sensor_index(file_name, "fan") {
            fan_indices.push(index);
        }
    }

    // The directory order is arbitrary, so the sensors are sorted by their index.
    temperature_indices.sort_unstable();
    fan_indices.sort_unstable();

    let mut temperatures = Vec::with_capacity(temperature_indices.len());

    for index in temperature_indices {
        // A sensor whose input cannot be read (e.g. `ENODATA`) is skipped.
        let Ok(current) =
            read_sysfs_number::<i64, _>(device_path.join(format!("temp{index}_input")))
        else {
            continue;
        };

        temperatures.push(Temperature {
            label:    read_sysfs_string(device_path.join(format!("temp{index}_label"))).ok(),
            current:  current as f64 / 1000.0,
            max:      read_millidegree(device_path.join(format!("temp{index}_max"))),
            critical: read_millidegree(device_path.join(format!("temp{index}_crit"))),
        });
    }

    let mut fans = Vec::with_capacity(fan_indices.len());

    for index in fan_indices {
        let Ok(rpm) = read_sysfs_number::<u32, _>(device_path.join(format!("fan{index}_input")))
        else {
            continue;
        };

        fans.push(Fan {
            label: read_sysfs_string(device_path.join(format!("fan{index}_label"))).ok(),
            rpm,
        });
    }

    Some((temperatures, fans))
}

/// Get temperature and fan sensors of all hardware monitoring devices by reading files in the `/sys/class/hwmon` folder, like the `sensors` command. Thermal zones also appear here through the `thermal_hwmon` bridge. Reading a sensor can be slow on some hardware, so this function should not be called at a high frequency.
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
        // A device without a name cannot be identified, so it is skipped.
        let Ok(name) = read_sysfs_string(device_path.join("name")) else {
            continue;
        };

        let Some((temperatures, fans)) = read_sensors(&device_path) else {
            continue;
        };

        devices.push(HwmonDevice {
            name,
            temperatures,
            fans,
        });
    }

    Ok(devices)
}
