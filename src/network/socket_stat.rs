use std::io::ErrorKind;

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{page_size, read_single_record_file},
};

/// Socket usage read from the `/proc/net/sockstat` file and the `/proc/net/sockstat6` file, like the `ss -s` command.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SocketStat {
    /// The number of sockets in use, of every protocol family (`sockets: used`).
    pub sockets_used:  u64,
    /// The number of IPv4 TCP sockets in use, which includes the listening ones (`TCP: inuse`).
    pub tcp_in_use:    u64,
    /// The number of IPv6 TCP sockets in use (`TCP6: inuse`).
    pub tcp6_in_use:   u64,
    /// The number of TCP sockets that are no longer attached to a file descriptor (`TCP: orphan`).
    pub tcp_orphan:    u64,
    /// The number of TCP sockets in the `TIME_WAIT` state (`TCP: tw`).
    pub tcp_time_wait: u64,
    /// The number of TCP sockets allocated, including the ones in `TIME_WAIT` (`TCP: alloc`).
    pub tcp_allocated: u64,
    /// The memory used by TCP sockets in bytes (`TCP: mem`, which is in pages).
    pub tcp_memory:    u64,
    /// The number of IPv4 UDP sockets in use (`UDP: inuse`).
    pub udp_in_use:    u64,
    /// The number of IPv6 UDP sockets in use (`UDP6: inuse`).
    pub udp6_in_use:   u64,
    /// The memory used by UDP sockets in bytes (`UDP: mem`, which is in pages).
    pub udp_memory:    u64,
}

/// Parse the lines of `sockstat` or `sockstat6` into `stat`. The memory fields are left in pages.
fn parse_sockstat(data: &[u8], stat: &mut SocketStat) -> Result<(), Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        let Some(label) = sc.next()? else {
            continue;
        };

        // The rest of the line is key-value pairs like `inuse 43 orphan 0`.
        while let Some(key) = sc.next()? {
            let Some(value) = sc.next_u64()? else {
                break;
            };

            match (label, key) {
                (b"sockets:", b"used") => stat.sockets_used = value,
                (b"TCP:", b"inuse") => stat.tcp_in_use = value,
                (b"TCP:", b"orphan") => stat.tcp_orphan = value,
                (b"TCP:", b"tw") => stat.tcp_time_wait = value,
                (b"TCP:", b"alloc") => stat.tcp_allocated = value,
                (b"TCP:", b"mem") => stat.tcp_memory = value,
                (b"UDP:", b"inuse") => stat.udp_in_use = value,
                (b"UDP:", b"mem") => stat.udp_memory = value,
                (b"TCP6:", b"inuse") => stat.tcp6_in_use = value,
                (b"UDP6:", b"inuse") => stat.udp6_in_use = value,
                _ => (),
            }
        }
    }

    Ok(())
}

/// Get the socket usage by reading the `/proc/net/sockstat` file and the `/proc/net/sockstat6` file. The second file is skipped when IPv6 is not available.
///
/// ```rust
/// use mprober_lib::network;
///
/// let socket_stat = network::get_socket_stat().unwrap();
///
/// println!("{socket_stat:#?}");
/// ```
pub fn get_socket_stat() -> Result<SocketStat, Error> {
    let mut stat = SocketStat::default();

    parse_sockstat(&read_single_record_file("/proc/net/sockstat", 512)?, &mut stat)?;

    match read_single_record_file("/proc/net/sockstat6", 256) {
        Ok(data) => parse_sockstat(&data, &mut stat)?,
        Err(err) if err.kind() == ErrorKind::NotFound => (),
        Err(err) => return Err(err.into()),
    }

    let page_size = page_size() as u64;

    stat.tcp_memory *= page_size;
    stat.udp_memory *= page_size;

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        const SOCKSTAT: &[u8] = b"sockets: used 1705
TCP: inuse 43 orphan 1 tw 2 alloc 54 mem 657
UDP: inuse 24 mem 2462
UDPLITE: inuse 0
RAW: inuse 1
FRAG: inuse 0 memory 0
";

        const SOCKSTAT6: &[u8] = b"TCP6: inuse 5
UDP6: inuse 3
UDPLITE6: inuse 0
RAW6: inuse 1
FRAG6: inuse 0 memory 0
";

        let mut stat = SocketStat::default();

        parse_sockstat(SOCKSTAT, &mut stat).unwrap();
        parse_sockstat(SOCKSTAT6, &mut stat).unwrap();

        assert_eq!(1705, stat.sockets_used);
        assert_eq!(43, stat.tcp_in_use);
        assert_eq!(5, stat.tcp6_in_use);
        assert_eq!(1, stat.tcp_orphan);
        assert_eq!(2, stat.tcp_time_wait);
        assert_eq!(54, stat.tcp_allocated);
        assert_eq!(657, stat.tcp_memory);
        assert_eq!(24, stat.udp_in_use);
        assert_eq!(3, stat.udp6_in_use);
        assert_eq!(2462, stat.udp_memory);
    }
}
