use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{OrEof, read_single_record_file},
};

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

fn parse_load_average(data: &[u8]) -> Result<LoadAverage, Error> {
    let mut sc = ScannerU8SliceAscii::new(data);

    let one = sc.next_f64()?.or_eof()?;
    let five = sc.next_f64()?.or_eof()?;
    let fifteen = sc.next_f64()?.or_eof()?;

    // This field looks like `1/2789`, and reading up to a boundary does not skip the whitespace in front of it.
    sc.skip_whitespaces()?;

    let running_entities = sc.next_u32_until("/")?.or_eof()?;
    let total_entities = sc.next_u32()?.or_eof()?;

    let last_pid = sc.next_u32()?.or_eof()?;

    Ok(LoadAverage {
        one,
        five,
        fifteen,
        running_entities,
        total_entities,
        last_pid,
    })
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
    parse_load_average(&read_single_record_file("/proc/loadavg", 64)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let load_average =
            parse_load_average(b"1.23 0.45 0.06 2/2789 123456\n".as_slice()).unwrap();

        assert_eq!(1.23, load_average.one);
        assert_eq!(0.45, load_average.five);
        assert_eq!(0.06, load_average.fifteen);

        // The fourth field packs two numbers as `running/total`.
        assert_eq!(2, load_average.running_entities);
        assert_eq!(2789, load_average.total_entities);

        assert_eq!(123456, load_average.last_pid);
    }
}
