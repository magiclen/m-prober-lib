use std::io::{self, ErrorKind};

use crate::{
    Error,
    utils::{parse_number, read_file, unescape_octal},
};

/// One active swap area, read from the `/proc/swaps` file.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwapDevice {
    /// The path of the swap partition or of the swap file, e.g. `/dev/nvme0n1p3` or `/swapfile`.
    pub filename: String,
    /// The kind of the area, which is `partition` or `file`.
    pub kind:     String,
    /// The size of the area in bytes.
    pub size:     u64,
    /// The bytes of the area that are in use.
    pub used:     u64,
    /// The priority of the area. A higher one is used first, and the areas that share a priority are used round-robin.
    pub priority: i32,
}

/// Parse the content of `/proc/swaps`, whose first line only names the columns.
fn parse_swaps(data: &[u8]) -> Result<Vec<SwapDevice>, Error> {
    let mut swaps = Vec::with_capacity(1);

    for line in data.split(|&b| b == b'\n').skip(1) {
        if line.is_empty() {
            continue;
        }

        // The kernel escapes whitespace inside the file name as octal sequences.
        let mut fields = line.split(|b| b.is_ascii_whitespace()).filter(|field| !field.is_empty());

        let mut next_field =
            || fields.next().ok_or_else(|| io::Error::from(ErrorKind::UnexpectedEof));

        let filename = String::from_utf8_lossy(&unescape_octal(next_field()?)).into_owned();
        let kind = String::from_utf8_lossy(next_field()?).into_owned();

        // The sizes are in 1024-byte units.
        let size = parse_number::<u64>(next_field()?)? * 1024;
        let used = parse_number::<u64>(next_field()?)? * 1024;

        let priority = parse_number(next_field()?)?;

        swaps.push(SwapDevice {
            filename,
            kind,
            size,
            used,
            priority,
        });
    }

    Ok(swaps)
}

/// Get every active swap area by reading the `/proc/swaps` file, like the `swapon --show` command. Unlike the swap totals of [`crate::memory::free`], this tells which partitions and files the swap is spread over.
///
/// ```rust
/// use mprober_lib::memory;
///
/// let swaps = memory::get_swaps().unwrap();
///
/// println!("{swaps:#?}");
/// ```
#[inline]
pub fn get_swaps() -> Result<Vec<SwapDevice>, Error> {
    parse_swaps(&read_file("/proc/swaps", 512)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SWAPS: &[u8] = b"Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority
/dev/nvme0n1p3                          partition\t8000508\t\t1024\t\t-2
/swap\\040file                            file\t\t2097148\t\t0\t\t-3
";

    #[test]
    fn parse() {
        let swaps = parse_swaps(SWAPS).unwrap();

        assert_eq!(2, swaps.len());

        assert_eq!("/dev/nvme0n1p3", swaps[0].filename);
        assert_eq!("partition", swaps[0].kind);
        assert_eq!(8000508 * 1024, swaps[0].size);
        assert_eq!(1024 * 1024, swaps[0].used);
        assert_eq!(-2, swaps[0].priority);

        // A swap file whose path contains a space is escaped by the kernel.
        assert_eq!("/swap file", swaps[1].filename);
        assert_eq!("file", swaps[1].kind);
        assert_eq!(-3, swaps[1].priority);
    }

    #[test]
    fn parse_without_swap() {
        assert!(
            parse_swaps(b"Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n").unwrap().is_empty()
        );
    }
}
