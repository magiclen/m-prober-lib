use std::{
    fs::File,
    io::{self, ErrorKind, Read},
    time::Duration,
};

use crate::scanner_rust::{ScannerAscii, ScannerError};

/// One line of a PSI (Pressure Stall Information) file.
#[derive(Default, Debug, Clone)]
pub struct PressureStat {
    /// The percentage of time stalled, averaged over the last 10 seconds. `100.0` means `100%`.
    pub avg10:  f64,
    /// The percentage of time stalled, averaged over the last 60 seconds.
    pub avg60:  f64,
    /// The percentage of time stalled, averaged over the last 300 seconds.
    pub avg300: f64,
    /// The total time stalled since boot.
    pub total:  Duration,
}

#[derive(Default, Debug, Clone)]
pub struct Pressure {
    /// The time in which at least one task was stalled on the resource.
    pub some: PressureStat,
    /// The time in which all non-idle tasks were stalled on the resource at the same time. It is `None` for the CPU pressure on kernels older than 5.13.
    pub full: Option<PressureStat>,
}

fn read_pressure_stat<R: Read, const N: usize>(
    sc: &mut ScannerAscii<R, N>,
) -> Result<PressureStat, ScannerError> {
    // The line looks like `some avg10=0.00 avg60=0.00 avg300=0.00 total=2357091`.
    sc.drop_next_until("avg10=")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let avg10 = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next_until("avg60=")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let avg60 = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next_until("avg300=")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let avg300 = sc.next_f64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    sc.drop_next_until("total=")?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;
    let total = sc.next_u64()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    Ok(PressureStat {
        avg10,
        avg60,
        avg300,
        total: Duration::from_micros(total),
    })
}

fn parse_pressure<R: Read>(reader: R) -> Result<Pressure, ScannerError> {
    let mut sc: ScannerAscii<R, 256> = ScannerAscii::new2(reader);

    let label = sc.next_raw()?.ok_or(io::Error::from(ErrorKind::UnexpectedEof))?;

    if label != b"some" {
        return Err(io::Error::from(ErrorKind::InvalidData).into());
    }

    let some = read_pressure_stat(&mut sc)?;

    let full = match sc.next_raw()? {
        Some(label) if label == b"full" => Some(read_pressure_stat(&mut sc)?),
        _ => None,
    };

    Ok(Pressure {
        some,
        full,
    })
}

/// Get the CPU pressure by reading the `/proc/pressure/cpu` file. PSI needs `CONFIG_PSI` and must not be disabled by the `psi=0` kernel parameter, otherwise the file does not exist.
///
/// ```rust,no_run
/// use mprober_lib::pressure;
///
/// let cpu_pressure = pressure::get_cpu_pressure().unwrap();
///
/// println!("{cpu_pressure:#?}");
/// ```
#[inline]
pub fn get_cpu_pressure() -> Result<Pressure, ScannerError> {
    parse_pressure(File::open("/proc/pressure/cpu")?)
}

/// Get the memory pressure by reading the `/proc/pressure/memory` file. PSI needs `CONFIG_PSI` and must not be disabled by the `psi=0` kernel parameter, otherwise the file does not exist.
///
/// ```rust,no_run
/// use mprober_lib::pressure;
///
/// let memory_pressure = pressure::get_memory_pressure().unwrap();
///
/// println!("{memory_pressure:#?}");
/// ```
#[inline]
pub fn get_memory_pressure() -> Result<Pressure, ScannerError> {
    parse_pressure(File::open("/proc/pressure/memory")?)
}

/// Get the I/O pressure by reading the `/proc/pressure/io` file. PSI needs `CONFIG_PSI` and must not be disabled by the `psi=0` kernel parameter, otherwise the file does not exist.
///
/// ```rust,no_run
/// use mprober_lib::pressure;
///
/// let io_pressure = pressure::get_io_pressure().unwrap();
///
/// println!("{io_pressure:#?}");
/// ```
#[inline]
pub fn get_io_pressure() -> Result<Pressure, ScannerError> {
    parse_pressure(File::open("/proc/pressure/io")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_with_full() {
        let pressure = parse_pressure(
            b"some avg10=0.00 avg60=0.01 avg300=0.02 total=3184265\nfull avg10=0.00 avg60=0.01 avg300=0.02 total=3137848\n".as_slice(),
        )
        .unwrap();

        assert_eq!(0.0, pressure.some.avg10);
        assert_eq!(0.01, pressure.some.avg60);
        assert_eq!(0.02, pressure.some.avg300);
        assert_eq!(Duration::from_micros(3184265), pressure.some.total);

        let full = pressure.full.unwrap();

        assert_eq!(Duration::from_micros(3137848), full.total);
    }

    #[test]
    fn parse_without_full() {
        let pressure =
            parse_pressure(b"some avg10=1.50 avg60=0.00 avg300=0.00 total=2357091\n".as_slice())
                .unwrap();

        assert_eq!(1.5, pressure.some.avg10);
        assert_eq!(Duration::from_micros(2357091), pressure.some.total);
        assert!(pressure.full.is_none());
    }
}
