use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::Path,
};

use crate::{
    Error,
    utils::{read_file, read_sysfs_number, read_sysfs_string},
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

/// Get the identity of the machine by reading files in the `/sys/class/dmi/id` folder. Every field is `None` on a platform without DMI, and the fields that carry a serial number are left out because reading them needs root.
///
/// ```rust
/// use mprober_lib::system;
///
/// let dmi_info = system::get_dmi_info().unwrap();
///
/// println!("{dmi_info:#?}");
/// ```
pub fn get_dmi_info() -> Result<DmiInfo, Error> {
    let path = Path::new("/sys/class/dmi/id");

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

/// The hypervisor this system runs under.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VirtualMachine {
    /// KVM, which the firmware reports as `KVM` or `Standard PC`.
    Kvm,
    /// QEMU without KVM acceleration.
    Qemu,
    /// VMware Workstation, Player or ESXi.
    VMware,
    /// Oracle VirtualBox.
    VirtualBox,
    /// Xen.
    Xen,
    /// Microsoft Hyper-V.
    HyperV,
    /// Parallels Desktop.
    Parallels,
    /// Bochs.
    Bochs,
    /// Amazon EC2, whose firmware does not say which hypervisor is underneath.
    AmazonEc2,
    /// The CPU reports a hypervisor, but the firmware does not say which one.
    Other,
}

impl VirtualMachine {
    /// Get the name of this hypervisor, e.g. `kvm`, as the `systemd-detect-virt` command reports it.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            VirtualMachine::Kvm => "kvm",
            VirtualMachine::Qemu => "qemu",
            VirtualMachine::VMware => "vmware",
            VirtualMachine::VirtualBox => "oracle",
            VirtualMachine::Xen => "xen",
            VirtualMachine::HyperV => "microsoft",
            VirtualMachine::Parallels => "parallels",
            VirtualMachine::Bochs => "bochs",
            VirtualMachine::AmazonEc2 => "amazon",
            VirtualMachine::Other => "vm-other",
        }
    }
}

impl Display for VirtualMachine {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The container this process runs in.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Container {
    /// Docker.
    Docker,
    /// Podman.
    Podman,
    /// LXC or LXD.
    Lxc,
    /// `systemd-nspawn`.
    SystemdNspawn,
    /// OpenVZ.
    OpenVz,
    /// The Windows Subsystem for Linux.
    Wsl,
    /// A container runtime this crate does not know by name.
    Other,
}

impl Container {
    /// Get the name of this container runtime, e.g. `docker`, as the `systemd-detect-virt` command reports it.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            Container::Docker => "docker",
            Container::Podman => "podman",
            Container::Lxc => "lxc",
            Container::SystemdNspawn => "systemd-nspawn",
            Container::OpenVz => "openvz",
            Container::Wsl => "wsl",
            Container::Other => "container-other",
        }
    }
}

impl Display for Container {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What this system is virtualized by. Both fields can be set at once, e.g. for a container inside a virtual machine.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Virtualization {
    /// The hypervisor, or `None` on bare metal.
    pub virtual_machine: Option<VirtualMachine>,
    /// The container runtime, or `None` outside a container.
    pub container:       Option<Container>,
}

impl Virtualization {
    /// Whether the system is virtualized in any way.
    #[inline]
    pub fn is_virtualized(&self) -> bool {
        self.virtual_machine.is_some() || self.container.is_some()
    }
}

/// Detect the hypervisor from what the firmware reports about the machine.
fn detect_virtual_machine(dmi_info: &DmiInfo) -> Option<VirtualMachine> {
    let vendor = dmi_info.sys_vendor.as_deref().unwrap_or_default();
    let product = dmi_info.product_name.as_deref().unwrap_or_default();

    // The vendor is the reliable half, because several hypervisors let the product name be configured.
    let machine = if vendor.contains("VMware") {
        VirtualMachine::VMware
    } else if vendor.contains("innotek") || product.contains("VirtualBox") {
        VirtualMachine::VirtualBox
    } else if vendor.contains("Xen") || product.contains("HVM domU") {
        VirtualMachine::Xen
    } else if vendor.contains("Parallels") {
        VirtualMachine::Parallels
    } else if vendor.contains("Amazon EC2") {
        VirtualMachine::AmazonEc2
    } else if vendor.contains("Microsoft Corporation") && product.contains("Virtual Machine") {
        VirtualMachine::HyperV
    } else if vendor.contains("Bochs") {
        VirtualMachine::Bochs
    } else if product.contains("KVM") || vendor.contains("KVM") {
        VirtualMachine::Kvm
    } else if vendor.contains("QEMU") {
        VirtualMachine::Qemu
    } else if has_hypervisor_flag() {
        // The CPU says it is virtualized even though the firmware pretends otherwise.
        VirtualMachine::Other
    } else {
        return None;
    };

    Some(machine)
}

/// Check whether the CPU reports the `hypervisor` flag, which every hypervisor sets.
fn has_hypervisor_flag() -> bool {
    let Ok(data) = read_file("/proc/cpuinfo", 64 * 1024) else {
        return false;
    };

    for line in data.split(|&b| b == b'\n') {
        if !line.starts_with(b"flags") {
            continue;
        }

        return line.split(|b| b.is_ascii_whitespace()).any(|token| token == b"hypervisor");
    }

    false
}

/// Detect the container runtime from the files and the environment a runtime leaves behind.
fn detect_container() -> Option<Container> {
    // The Windows Subsystem for Linux marks itself in the kernel release.
    if let Ok(release) = read_sysfs_string("/proc/sys/kernel/osrelease")
        && (release.contains("microsoft")
            || release.contains("Microsoft")
            || release.contains("WSL"))
    {
        return Some(Container::Wsl);
    }

    if Path::new("/.dockerenv").exists() {
        return Some(Container::Docker);
    }

    if Path::new("/run/.containerenv").exists() {
        return Some(Container::Podman);
    }

    if Path::new("/proc/vz").exists() && !Path::new("/proc/bc").exists() {
        return Some(Container::OpenVz);
    }

    // The runtime sets this variable for the init process of the container, but only root may read it.
    if let Ok(environ) = fs::read("/proc/1/environ") {
        for entry in environ.split(|&b| b == 0) {
            if let Some(value) = entry.strip_prefix(b"container=") {
                let container = match value {
                    b"docker" => Container::Docker,
                    b"podman" => Container::Podman,
                    b"lxc" | b"lxc-libvirt" => Container::Lxc,
                    b"systemd-nspawn" => Container::SystemdNspawn,
                    _ => Container::Other,
                };

                return Some(container);
            }
        }
    }

    // The cgroup path of a container keeps the name of the runtime that created it.
    if let Ok(cgroup) = read_file("/proc/self/cgroup", 512) {
        let cgroup = String::from_utf8_lossy(&cgroup);

        if cgroup.contains("/docker") {
            return Some(Container::Docker);
        }

        if cgroup.contains("/lxc") {
            return Some(Container::Lxc);
        }
    }

    None
}

/// Detect what this system is virtualized by, using the DMI information, the `hypervisor` CPU flag and the marks a container runtime leaves behind, like the `systemd-detect-virt` command. A container inside a virtual machine reports both.
///
/// The container detection is a best effort, because a runtime is free to leave no mark at all, and reading the environment of the init process needs root.
///
/// ```rust
/// use mprober_lib::system;
///
/// let virtualization = system::detect_virtualization().unwrap();
///
/// println!("{virtualization:#?}");
/// ```
pub fn detect_virtualization() -> Result<Virtualization, Error> {
    let dmi_info = get_dmi_info()?;

    Ok(Virtualization {
        virtual_machine: detect_virtual_machine(&dmi_info),
        container:       detect_container(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dmi(vendor: &str, product: &str) -> DmiInfo {
        DmiInfo {
            sys_vendor: Some(vendor.to_string()),
            product_name: Some(product.to_string()),
            ..DmiInfo::default()
        }
    }

    #[test]
    fn detect_hypervisors() {
        assert_eq!(
            Some(VirtualMachine::VMware),
            detect_virtual_machine(&dmi("VMware, Inc.", "VMware Virtual Platform"))
        );
        assert_eq!(
            Some(VirtualMachine::VirtualBox),
            detect_virtual_machine(&dmi("innotek GmbH", "VirtualBox"))
        );
        assert_eq!(
            Some(VirtualMachine::HyperV),
            detect_virtual_machine(&dmi("Microsoft Corporation", "Virtual Machine"))
        );
        assert_eq!(
            Some(VirtualMachine::Kvm),
            detect_virtual_machine(&dmi("QEMU", "Standard PC (Q35 + ICH9, 2009)  KVM"))
        );
        assert_eq!(
            Some(VirtualMachine::Qemu),
            detect_virtual_machine(&dmi("QEMU", "Standard PC (Q35 + ICH9, 2009)"))
        );
        assert_eq!(
            Some(VirtualMachine::AmazonEc2),
            detect_virtual_machine(&dmi("Amazon EC2", "t3.micro"))
        );
    }

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
