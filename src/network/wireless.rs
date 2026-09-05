use std::io::ErrorKind;

use crate::{Error, utils::read_file};

/// The signal information of one wireless interface, read from the `/proc/net/wireless` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Wireless {
    /// The name of the interface, e.g. `wlan0`.
    pub interface:       String,
    /// The link quality, which most drivers report from `0` to `70`.
    pub link_quality:    f64,
    /// The signal level in dBm, which is normally between `-90` (bad) and `-30` (excellent).
    pub signal_level:    f64,
    /// The noise level in dBm. A driver that does not measure it reports `-256`.
    pub noise_level:     f64,
    /// The packets discarded because they were for another network.
    pub discarded_nwid:  u64,
    /// The packets discarded because they could not be decrypted.
    pub discarded_crypt: u64,
    /// The packets discarded because a fragment was missing.
    pub discarded_frag:  u64,
    /// The packets that could not be delivered after the maximum number of retries.
    pub discarded_retry: u64,
    /// The packets discarded for any other reason.
    pub discarded_misc:  u64,
    /// The beacons of the access point that were missed.
    pub missed_beacon:   u64,
}

/// Parse the content of `/proc/net/wireless`, whose first two lines only name the columns.
fn parse_wireless(data: &[u8]) -> Result<Vec<Wireless>, Error> {
    let mut wireless = Vec::with_capacity(1);

    for line in data.split(|&b| b == b'\n').skip(2) {
        let mut fields = line.split(|b| b.is_ascii_whitespace()).filter(|f| !f.is_empty());

        let Some(interface) = fields.next() else {
            continue;
        };

        let Some(interface) = interface.strip_suffix(b":") else {
            continue;
        };

        // The status is a hexadecimal value this crate does not report.
        let Some(_status) = fields.next() else {
            continue;
        };

        // The three signal values carry a trailing dot when the driver reports them as updated.
        let mut next_level = || -> f64 {
            fields
                .next()
                .and_then(|field| {
                    std::str::from_utf8(field.strip_suffix(b".").unwrap_or(field))
                        .ok()?
                        .parse()
                        .ok()
                })
                .unwrap_or(0.0)
        };

        let link_quality = next_level();
        let signal_level = next_level();
        let noise_level = next_level();

        let mut next_count = || -> u64 {
            fields
                .next()
                .and_then(|field| std::str::from_utf8(field).ok()?.parse().ok())
                .unwrap_or(0)
        };

        wireless.push(Wireless {
            interface: String::from_utf8_lossy(interface).into_owned(),
            link_quality,
            signal_level,
            noise_level,
            discarded_nwid: next_count(),
            discarded_crypt: next_count(),
            discarded_frag: next_count(),
            discarded_retry: next_count(),
            discarded_misc: next_count(),
            missed_beacon: next_count(),
        });
    }

    Ok(wireless)
}

/// Get the signal information of every wireless interface by reading the `/proc/net/wireless` file, like the `iwconfig` command. An empty result is returned when the kernel has no wireless support, and an interface that is not associated with an access point reports zeros.
///
/// ```rust
/// use mprober_lib::network;
///
/// let wireless = network::get_wireless().unwrap();
///
/// println!("{wireless:#?}");
/// ```
pub fn get_wireless() -> Result<Vec<Wireless>, Error> {
    match read_file("/proc/net/wireless", 1024) {
        Ok(data) => parse_wireless(&data),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIRELESS: &[u8] =
        b"Inter-| sta-|   Quality        |   Discarded packets               | Missed | WE
 face | tus | link level noise |  nwid  crypt   frag  retry   misc | beacon | 22
 wlan0: 0000   70.  -40.  -256        0      1      2      3      4        5
";

    #[test]
    fn parse() {
        let wireless = parse_wireless(WIRELESS).unwrap();

        assert_eq!(1, wireless.len());

        assert_eq!("wlan0", wireless[0].interface);
        assert_eq!(70.0, wireless[0].link_quality);
        assert_eq!(-40.0, wireless[0].signal_level);
        assert_eq!(-256.0, wireless[0].noise_level);
        assert_eq!(0, wireless[0].discarded_nwid);
        assert_eq!(1, wireless[0].discarded_crypt);
        assert_eq!(2, wireless[0].discarded_frag);
        assert_eq!(3, wireless[0].discarded_retry);
        assert_eq!(4, wireless[0].discarded_misc);
        assert_eq!(5, wireless[0].missed_beacon);
    }

    #[test]
    fn parse_without_interface() {
        let wireless = parse_wireless(
            b"Inter-| sta-|   Quality        |   Discarded packets               | Missed | WE\n face | tus | link level noise |  nwid  crypt   frag  retry   misc | beacon | 22\n",
        )
        .unwrap();

        assert!(wireless.is_empty());
    }
}
