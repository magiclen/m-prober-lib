use std::{
    io::{self, ErrorKind},
    path::Path,
};

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// The identity of the machine as the firmware reports it, read from the `/sys/class/dmi/id` folder. Only x86 and a few ARM servers have DMI, and a virtual machine reports the identity of its hypervisor.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DmiInfo {
    /// The manufacturer of the system, e.g. `ASUS` or `Dell Inc.`.
    pub sys_vendor:      Option<String>,
    /// The model of the system, e.g. `System Product Name` or `XPS 13 9310`.
    pub product_name:    Option<String>,
    /// The version of the system.
    pub product_version: Option<String>,
    /// The product family, e.g. `ThinkPad X1 Carbon`.
    pub product_family:  Option<String>,
    /// The manufacturer of the motherboard.
    pub board_vendor:    Option<String>,
    /// The model of the motherboard, e.g. `PRIME X670-P`.
    pub board_name:      Option<String>,
    /// The manufacturer of the firmware, e.g. `American Megatrends Inc.`.
    pub bios_vendor:     Option<String>,
    /// The version of the firmware.
    pub bios_version:    Option<String>,
    /// The release date of the firmware, e.g. `04/25/2024`.
    pub bios_date:       Option<String>,
    /// The SMBIOS chassis type, e.g. `3` for a desktop and `10` for a notebook. Use [`DmiInfo::chassis_type_name`] to get its name.
    pub chassis_type:    Option<u8>,
}

impl DmiInfo {
    /// Get the name of the SMBIOS chassis type, e.g. `Notebook`. It is `None` when the firmware reports a type this crate does not know.
    pub fn chassis_type_name(&self) -> Option<&'static str> {
        let name = match self.chassis_type? {
            1 => "Other",
            2 => "Unknown",
            3 => "Desktop",
            4 => "Low Profile Desktop",
            5 => "Pizza Box",
            6 => "Mini Tower",
            7 => "Tower",
            8 => "Portable",
            9 => "Laptop",
            10 => "Notebook",
            11 => "Hand Held",
            12 => "Docking Station",
            13 => "All In One",
            14 => "Sub Notebook",
            15 => "Space-saving",
            16 => "Lunch Box",
            17 => "Main Server Chassis",
            18 => "Expansion Chassis",
            19 => "Sub Chassis",
            20 => "Bus Expansion Chassis",
            21 => "Peripheral Chassis",
            22 => "RAID Chassis",
            23 => "Rack Mount Chassis",
            24 => "Sealed-case PC",
            25 => "Multi-system Chassis",
            26 => "Compact PCI",
            27 => "Advanced TCA",
            28 => "Blade",
            29 => "Blade Enclosure",
            30 => "Tablet",
            31 => "Convertible",
            32 => "Detachable",
            33 => "IoT Gateway",
            34 => "Embedded PC",
            35 => "Mini PC",
            36 => "Stick PC",
            _ => return None,
        };

        Some(name)
    }
}

/// Read a DMI attribute, treating the placeholder strings the firmware writes when it has nothing as absent.
fn read_dmi<P: AsRef<Path>>(path: P) -> Option<String> {
    let value = read_sysfs_string(path).ok()?;

    let value = value.trim();

    if value.is_empty()
        || value.eq_ignore_ascii_case("To Be Filled By O.E.M.")
        || value.eq_ignore_ascii_case("Default string")
        || value.eq_ignore_ascii_case("Not Specified")
        || value.eq_ignore_ascii_case("Not Applicable")
    {
        return None;
    }

    Some(value.to_owned())
}

/// Get the identity of the machine by reading files in the `/sys/class/dmi/id` folder. A `NotFound` error is returned on a platform without DMI (e.g. most ARM boards), a field the firmware left empty is `None`, and the fields that carry a serial number are left out because reading them needs root.
///
/// ```rust,no_run
/// use mprober_lib::system;
///
/// let dmi_info = system::get_dmi_info().unwrap();
///
/// println!("{dmi_info:#?}");
/// ```
pub fn get_dmi_info() -> Result<DmiInfo, Error> {
    let path = Path::new("/sys/class/dmi/id");

    // Without the folder there is no DMI at all, which is reported like every other missing kernel feature instead of as a struct full of `None`.
    if !path.is_dir() {
        return Err(io::Error::from(ErrorKind::NotFound).into());
    }

    Ok(DmiInfo {
        sys_vendor:      read_dmi(path.join("sys_vendor")),
        product_name:    read_dmi(path.join("product_name")),
        product_version: read_dmi(path.join("product_version")),
        product_family:  read_dmi(path.join("product_family")),
        board_vendor:    read_dmi(path.join("board_vendor")),
        board_name:      read_dmi(path.join("board_name")),
        bios_vendor:     read_dmi(path.join("bios_vendor")),
        bios_version:    read_dmi(path.join("bios_version")),
        bios_date:       read_dmi(path.join("bios_date")),
        chassis_type:    read_sysfs_number(path.join("chassis_type")).ok(),
    })
}

/// Get the machine ID, which is a stable identifier of this installation, by reading the `/etc/machine-id` file. The `/var/lib/dbus/machine-id` file is used as a fallback on a system that only has the D-Bus one. A `NotFound` error is returned when neither exists.
///
/// ```rust,no_run
/// use mprober_lib::system;
///
/// let machine_id = system::get_machine_id().unwrap();
///
/// println!("{machine_id}");
/// ```
pub fn get_machine_id() -> Result<String, Error> {
    match read_sysfs_string("/etc/machine-id") {
        Ok(machine_id) if !machine_id.is_empty() => Ok(machine_id),
        // A system that predates systemd only has the D-Bus one, and a freshly imaged one can have an empty file.
        _ => Ok(read_sysfs_string("/var/lib/dbus/machine-id")?),
    }
}

/// Get the boot ID, which is a random identifier the kernel generates on every boot, by reading the `/proc/sys/kernel/random/boot_id` file. It changes on every reboot, so it tells whether two samples came from the same boot.
///
/// ```rust
/// use mprober_lib::system;
///
/// let boot_id = system::get_boot_id().unwrap();
///
/// println!("{boot_id}");
/// ```
#[inline]
pub fn get_boot_id() -> Result<String, Error> {
    Ok(read_sysfs_string("/proc/sys/kernel/random/boot_id")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chassis_type_names() {
        let notebook = DmiInfo {
            chassis_type: Some(10),
            ..DmiInfo::default()
        };

        assert_eq!(Some("Notebook"), notebook.chassis_type_name());

        let unknown = DmiInfo {
            chassis_type: Some(200),
            ..DmiInfo::default()
        };

        assert_eq!(None, unknown.chassis_type_name());
        assert_eq!(None, DmiInfo::default().chassis_type_name());
    }
}
