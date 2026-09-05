use std::{fs, io::ErrorKind, path::Path, time::Duration};

use crate::{
    Error,
    utils::{read_sysfs_number, read_sysfs_string},
};

/// One power limit of a power capping zone.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PowerCapConstraint {
    /// The name of the constraint, e.g. `long_term` or `short_term`.
    pub name:            String,
    /// The power limit in watts. It is `None` when the firmware does not report one.
    pub power_limit:     Option<f64>,
    /// The window the limit is averaged over. It is `None` when the firmware does not report one.
    pub time_window:     Option<Duration>,
    /// The highest limit that may be set in watts.
    pub max_power_limit: Option<f64>,
}

/// One power capping zone under `/sys/class/powercap`, which on x86 is an Intel or AMD RAPL domain.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PowerCapZone {
    /// The sysfs name of the zone, e.g. `intel-rapl:0` for a package or `intel-rapl:0:1` for a domain inside it.
    pub id:                  String,
    /// The name of the domain, e.g. `package-0`, `core`, `uncore`, `dram` or `psys`.
    pub name:                String,
    /// The energy the domain consumed since the counter was last reset, in microjoules. It is raw because it wraps around at `max_energy_range_uj`, which [`PowerCapZone::compute_power`] takes care of. It is `None` when the counter could not be read, which is the normal case for an unprivileged process because most distributions restrict it to root.
    pub energy_uj:           Option<u64>,
    /// The value `energy_uj` wraps around at, in microjoules.
    pub max_energy_range_uj: u64,
    /// Whether the capping of this zone is enabled. It is `None` when the driver does not report it.
    pub enabled:             Option<bool>,
    /// The power limits of this zone.
    pub constraints:         Vec<PowerCapConstraint>,
}

impl PowerCapZone {
    /// Get the energy the domain consumed since the counter was last reset, in joules. It is `None` when the counter could not be read.
    #[inline]
    pub fn energy(&self) -> Option<f64> {
        self.energy_uj.map(|uj| uj as f64 / 1_000_000.0)
    }

    /// Compute the average power in watts between two readings of the same zone at different time. The counter wraps around, so a reading that went backwards is treated as one wrap. It is `None` when either reading has no energy counter.
    ///
    /// ```rust,no_run
    /// use std::{thread::sleep, time::Duration};
    ///
    /// use mprober_lib::power_supply;
    ///
    /// let pre_zones = power_supply::get_power_cap_zones().unwrap();
    ///
    /// let interval = Duration::from_millis(500);
    ///
    /// sleep(interval);
    ///
    /// let zones = power_supply::get_power_cap_zones().unwrap();
    ///
    /// if !pre_zones.is_empty() && !zones.is_empty() {
    ///     if let Some(watts) = pre_zones[0].compute_power(&zones[0], interval) {
    ///         println!("{}: {watts:.1} W", zones[0].name);
    ///     }
    /// }
    /// ```
    pub fn compute_power(&self, zone_after_this: &PowerCapZone, interval: Duration) -> Option<f64> {
        let seconds = interval.as_secs_f64();

        if seconds <= 0.0 {
            return None;
        }

        let (before, after) = (self.energy_uj?, zone_after_this.energy_uj?);

        let energy_uj = if after >= before {
            after - before
        } else {
            // The counter wrapped around, which happens every minute or so on a busy package.
            (self.max_energy_range_uj - before).saturating_add(after)
        };

        Some(energy_uj as f64 / 1_000_000.0 / seconds)
    }
}

/// Read the power limits of a zone. The kernel numbers them from zero and stops at the first gap.
fn read_constraints(path: &Path) -> Vec<PowerCapConstraint> {
    let mut constraints = Vec::with_capacity(2);

    for index in 0.. {
        let Ok(name) = read_sysfs_string(path.join(format!("constraint_{index}_name"))) else {
            break;
        };

        // The limits are in microwatts and the window is in microseconds.
        let power_limit =
            read_sysfs_number::<u64, _>(path.join(format!("constraint_{index}_power_limit_uw")))
                .ok()
                .map(|uw| uw as f64 / 1_000_000.0);

        let max_power_limit =
            read_sysfs_number::<u64, _>(path.join(format!("constraint_{index}_max_power_uw")))
                .ok()
                .map(|uw| uw as f64 / 1_000_000.0);

        let time_window =
            read_sysfs_number::<u64, _>(path.join(format!("constraint_{index}_time_window_us")))
                .ok()
                .map(Duration::from_micros);

        constraints.push(PowerCapConstraint {
            name,
            power_limit,
            time_window,
            max_power_limit,
        });
    }

    constraints
}

fn read_power_cap_zone(id: String, path: &Path) -> Option<PowerCapZone> {
    // Only a zone is named, while the folder of a control type (e.g. `intel-rapl` itself) has no name file.
    let name = read_sysfs_string(path.join("name")).ok()?;

    Some(PowerCapZone {
        id,
        name,
        // The counter is root-only on most distributions, so the zone is still reported without it.
        energy_uj: read_sysfs_number(path.join("energy_uj")).ok(),
        max_energy_range_uj: read_sysfs_number(path.join("max_energy_range_uj"))
            .unwrap_or(u64::MAX),
        enabled: read_sysfs_number::<u8, _>(path.join("enabled")).ok().map(|enabled| enabled == 1),
        constraints: read_constraints(path),
    })
}

/// Get every power capping zone by reading files in the `/sys/class/powercap` folder. On x86 these are the RAPL domains, which report the energy the CPU package, its cores, its uncore and the DRAM consumed, and they are the standard way to measure the power draw of a CPU. The zones are ordered by their sysfs names, and an empty result means the platform has no power capping support.
///
/// Reading `energy_uj` needs root on most distributions, because the counter leaks information about what the CPU is doing. An unprivileged process still gets the zones and their power limits, with `energy_uj` set to `None`.
///
/// ```rust
/// use mprober_lib::power_supply;
///
/// let zones = power_supply::get_power_cap_zones().unwrap();
///
/// println!("{zones:#?}");
/// ```
pub fn get_power_cap_zones() -> Result<Vec<PowerCapZone>, Error> {
    let read_dir = match fs::read_dir("/sys/class/powercap") {
        Ok(read_dir) => read_dir,
        // A kernel without power capping support simply has no zones.
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err.into()),
    };

    let mut entries: Vec<(String, _)> = Vec::new();

    for entry in read_dir {
        let entry = entry?;

        entries.push((entry.file_name().to_string_lossy().into_owned(), entry.path()));
    }

    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));

    let mut zones = Vec::with_capacity(entries.len());

    for (id, path) in entries {
        if let Some(zone) = read_power_cap_zone(id, &path) {
            zones.push(zone);
        }
    }

    Ok(zones)
}
