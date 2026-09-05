use std::io::{self, ErrorKind};

use scanner_rust::ScannerAscii;

use crate::{Error, utils::parse_number};

/// The load average read from the `/proc/loadavg` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LoadAverage {
    /// The load average over the last minute.
    pub one:              f64,
    /// The load average over the last five minutes.
    pub five:             f64,
    /// The load average over the last fifteen minutes.
    pub fifteen:          f64,
    /// The number of currently runnable kernel scheduling entities (processes and threads).
    pub running_entities: u32,
    /// The number of kernel scheduling entities that currently exist.
    pub total_entities:   u32,
    /// The PID of the process that was most recently created.
    pub last_pid:         u32,
}

/// Get the load average by reading the `/proc/loadavg` file.
///
/// ```rust
/// use mprober_lib::load_average;
///
/// let load_average = load_average::get_load_average().unwrap();
///
/// println!("{load_average:#?}");
/// ```
#[inline]
pub fn get_load_average() -> Result<LoadAverage, Error> {
    let mut sc: ScannerAscii<_, 32> = ScannerAscii::scan_path2("/proc/loadavg")?;

    let one = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let five = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let fifteen = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    let (running_entities, total_entities) = {
        // This field looks like `1/2789`.
        let entities = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

        let slash_index = entities
            .iter()
            .position(|&b| b == b'/')
            .ok_or(io::Error::from(ErrorKind::InvalidData))?;

        (parse_number(&entities[..slash_index])?, parse_number(&entities[(slash_index + 1)..])?)
    };

    let last_pid = sc.next_u32()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(LoadAverage {
        one,
        five,
        fifteen,
        running_entities,
        total_entities,
        last_pid,
    })
}
