use std::{fs, io::ErrorKind, path::Path};

use crate::{
    Error,
    utils::{read_sysfs_bool, read_sysfs_micro, read_sysfs_number, read_sysfs_string},
};

/// One power supply under `/sys/class/power_supply`, e.g. a laptop battery or an AC adapter. Every driver reports a different set of attributes, so most fields are optional.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PowerSupply {
    /// The name of the device, e.g. `BAT0` or `AC`.
    pub name:               String,
    /// The type of the device, e.g. `Battery`, `Mains` or `USB`.
    pub kind:               String,
    /// `System` for a battery powering the computer, or `Device` for the battery of a peripheral like a wireless mouse. It is `None` when the driver does not report it, which means `System`.
    pub scope:              Option<String>,
    /// The charging status of a battery, e.g. `Charging`, `Discharging`, `Not charging` or `Full`.
    pub status:             Option<String>,
    /// Whether the power source is plugged in. It is reported by adapters, not by batteries.
    pub online:             Option<bool>,
    /// The remaining capacity of a battery in percent.
    pub capacity:           Option<u8>,
    /// The coarse remaining capacity of a battery, e.g. `Normal`, `Low` or `Critical`, which some devices report instead of a percentage.
    pub capacity_level:     Option<String>,
    /// The remaining energy in Wh. A battery reports either energy or charge, see `charge_now`.
    pub energy_now:         Option<f64>,
    /// The energy when the battery is full in Wh.
    pub energy_full:        Option<f64>,
    /// The energy when the battery was new in Wh.
    pub energy_full_design: Option<f64>,
    /// The remaining charge in Ah.
    pub charge_now:         Option<f64>,
    /// The charge when the battery is full in Ah.
    pub charge_full:        Option<f64>,
    /// The charge when the battery was new in Ah.
    pub charge_full_design: Option<f64>,
    /// The power being drawn or charged in W.
    pub power_now:          Option<f64>,
    /// The current in A. Some drivers report a negative value while discharging.
    pub current_now:        Option<f64>,
    /// The voltage in V.
    pub voltage_now:        Option<f64>,
    /// The number of charge cycles of a battery.
    pub cycle_count:        Option<u32>,
}

fn read_power_supply(name: String, path: &Path) -> Option<PowerSupply> {
    // A device without a type cannot be interpreted, so it is skipped.
    let kind = read_sysfs_string(path.join("type")).ok()?;

    Some(PowerSupply {
        name,
        kind,
        scope: read_sysfs_string(path.join("scope")).ok(),
        status: read_sysfs_string(path.join("status")).ok(),
        online: read_sysfs_bool(path.join("online")),
        capacity: read_sysfs_number(path.join("capacity")).ok(),
        capacity_level: read_sysfs_string(path.join("capacity_level")).ok(),
        energy_now: read_sysfs_micro(path.join("energy_now")),
        energy_full: read_sysfs_micro(path.join("energy_full")),
        energy_full_design: read_sysfs_micro(path.join("energy_full_design")),
        charge_now: read_sysfs_micro(path.join("charge_now")),
        charge_full: read_sysfs_micro(path.join("charge_full")),
        charge_full_design: read_sysfs_micro(path.join("charge_full_design")),
        power_now: read_sysfs_micro(path.join("power_now")),
        current_now: read_sysfs_micro(path.join("current_now")),
        voltage_now: read_sysfs_micro(path.join("voltage_now")),
        cycle_count: read_sysfs_number(path.join("cycle_count")).ok(),
    })
}

/// Get all power supplies (batteries and adapters) by reading files in the `/sys/class/power_supply` folder. The devices are ordered by their names.
///
/// ```rust
/// use mprober_lib::power_supply;
///
/// let power_supplies = power_supply::get_power_supplies().unwrap();
///
/// println!("{power_supplies:#?}");
/// ```
pub fn get_power_supplies() -> Result<Vec<PowerSupply>, Error> {
    let read_dir = match fs::read_dir("/sys/class/power_supply") {
        Ok(read_dir) => read_dir,
        // A kernel without power supply support simply has no devices.
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let mut entries: Vec<(String, _)> = Vec::new();

    for entry in read_dir {
        let entry = entry?;

        entries.push((entry.file_name().to_string_lossy().into_owned(), entry.path()));
    }

    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let mut power_supplies = Vec::with_capacity(entries.len());

    for (name, path) in entries {
        if let Some(power_supply) = read_power_supply(name, &path) {
            power_supplies.push(power_supply);
        }
    }

    Ok(power_supplies)
}
