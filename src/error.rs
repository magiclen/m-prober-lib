use std::{
    error::Error as StdError,
    fmt::{self, Display, Formatter},
    io,
    num::{ParseFloatError, ParseIntError},
};

use crate::scanner_rust::ScannerError;

/// The error type of this crate. The variants match those of `ScannerError`, which this crate used before v0.2.
#[derive(Debug)]
pub enum Error {
    /// A file could not be opened or read, or a function in libc failed.
    IOError(io::Error),
    /// An integer field could not be parsed.
    ParseIntError(ParseIntError),
    /// A floating point field could not be parsed.
    ParseFloatError(ParseFloatError),
}

impl Error {
    /// Whether this error means that the kernel does not provide the data at all, e.g. because the feature was not built into it or the hardware has no such device. It is a `NotFound` I/O error, which every function of this crate returns for a missing `/proc` or `/sys` file, so a probe of an optional feature can be skipped with one call instead of matching on the inner error.
    ///
    /// ```rust
    /// use mprober_lib::pressure;
    ///
    /// match pressure::get_cpu_pressure() {
    ///     Ok(pressure) => println!("{pressure:#?}"),
    ///     // A kernel built without PSI has no `/proc/pressure` folder.
    ///     Err(err) if err.is_not_supported() => println!("PSI is not available"),
    ///     Err(err) => panic!("{err}"),
    /// }
    /// ```
    #[inline]
    pub fn is_not_supported(&self) -> bool {
        matches!(self, Error::IOError(err) if err.kind() == io::ErrorKind::NotFound)
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::IOError(err) => Display::fmt(err, f),
            Error::ParseIntError(err) => Display::fmt(err, f),
            Error::ParseFloatError(err) => Display::fmt(err, f),
        }
    }
}

impl StdError for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::IOError(err) => Some(err),
            Error::ParseIntError(err) => Some(err),
            Error::ParseFloatError(err) => Some(err),
        }
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(err: io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<ParseIntError> for Error {
    #[inline]
    fn from(err: ParseIntError) -> Error {
        Error::ParseIntError(err)
    }
}

impl From<ParseFloatError> for Error {
    #[inline]
    fn from(err: ParseFloatError) -> Error {
        Error::ParseFloatError(err)
    }
}

impl From<ScannerError> for Error {
    #[inline]
    fn from(err: ScannerError) -> Error {
        match err {
            ScannerError::IOError(err) => Error::IOError(err),
            ScannerError::ParseIntError(err) => Error::ParseIntError(err),
            ScannerError::ParseFloatError(err) => Error::ParseFloatError(err),
        }
    }
}

impl From<Error> for io::Error {
    #[inline]
    fn from(err: Error) -> io::Error {
        match err {
            Error::IOError(err) => err,
            err => io::Error::new(io::ErrorKind::InvalidData, err),
        }
    }
}
