use std::{
    fmt::{self, Display, Formatter},
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    str::from_utf8,
};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{parse_number, read_file},
};

/// The transport protocol of a socket.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SocketProtocol {
    /// TCP over IPv4, read from the `/proc/net/tcp` file.
    Tcp,
    /// TCP over IPv6, read from the `/proc/net/tcp6` file.
    Tcp6,
    /// UDP over IPv4, read from the `/proc/net/udp` file.
    Udp,
    /// UDP over IPv6, read from the `/proc/net/udp6` file.
    Udp6,
}

impl SocketProtocol {
    /// Get the name of this protocol as the `ss` command shows it, e.g. `tcp`.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            SocketProtocol::Tcp => "tcp",
            SocketProtocol::Tcp6 => "tcp6",
            SocketProtocol::Udp => "udp",
            SocketProtocol::Udp6 => "udp6",
        }
    }

    /// Whether this protocol uses IPv6.
    #[inline]
    pub fn is_ipv6(self) -> bool {
        matches!(self, SocketProtocol::Tcp6 | SocketProtocol::Udp6)
    }

    /// The path of the file the kernel reports this protocol in.
    #[inline]
    fn path(self) -> &'static str {
        match self {
            SocketProtocol::Tcp => "/proc/net/tcp",
            SocketProtocol::Tcp6 => "/proc/net/tcp6",
            SocketProtocol::Udp => "/proc/net/udp",
            SocketProtocol::Udp6 => "/proc/net/udp6",
        }
    }
}

impl Display for SocketProtocol {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The state of a socket, which is only meaningful for TCP. A UDP socket is `Close` while it is unconnected and `Established` after `connect(2)`.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SocketState {
    /// The kernel reported a value this crate does not know.
    #[default]
    Unknown,
    /// `TCP_ESTABLISHED`
    Established,
    /// `TCP_SYN_SENT`
    SynSent,
    /// `TCP_SYN_RECV`
    SynRecv,
    /// `TCP_FIN_WAIT1`
    FinWait1,
    /// `TCP_FIN_WAIT2`
    FinWait2,
    /// `TCP_TIME_WAIT`
    TimeWait,
    /// `TCP_CLOSE`
    Close,
    /// `TCP_CLOSE_WAIT`
    CloseWait,
    /// `TCP_LAST_ACK`
    LastAck,
    /// `TCP_LISTEN`
    Listen,
    /// `TCP_CLOSING`
    Closing,
    /// `TCP_NEW_SYN_RECV`
    NewSynRecv,
}

impl SocketState {
    /// Convert the number that the socket files report.
    #[inline]
    pub fn from_raw(value: u8) -> SocketState {
        match value {
            1 => SocketState::Established,
            2 => SocketState::SynSent,
            3 => SocketState::SynRecv,
            4 => SocketState::FinWait1,
            5 => SocketState::FinWait2,
            6 => SocketState::TimeWait,
            7 => SocketState::Close,
            8 => SocketState::CloseWait,
            9 => SocketState::LastAck,
            10 => SocketState::Listen,
            11 => SocketState::Closing,
            12 => SocketState::NewSynRecv,
            _ => SocketState::Unknown,
        }
    }

    /// Get the name of this state as the `ss` command shows it, e.g. `ESTAB`.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            SocketState::Unknown => "UNKNOWN",
            SocketState::Established => "ESTAB",
            SocketState::SynSent => "SYN-SENT",
            SocketState::SynRecv => "SYN-RECV",
            SocketState::FinWait1 => "FIN-WAIT-1",
            SocketState::FinWait2 => "FIN-WAIT-2",
            SocketState::TimeWait => "TIME-WAIT",
            SocketState::Close => "CLOSE",
            SocketState::CloseWait => "CLOSE-WAIT",
            SocketState::LastAck => "LAST-ACK",
            SocketState::Listen => "LISTEN",
            SocketState::Closing => "CLOSING",
            SocketState::NewSynRecv => "NEW-SYN-RECV",
        }
    }
}

impl Display for SocketState {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One socket, read from the `/proc/net/tcp` family of files.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SocketConnection {
    /// The transport protocol of this socket.
    pub protocol:       SocketProtocol,
    /// The local address and port. The address is unspecified for a socket that listens on every interface.
    pub local_address:  SocketAddr,
    /// The remote address and port, which is all zeros for a listening or unconnected socket.
    pub remote_address: SocketAddr,
    /// The state of the socket.
    pub state:          SocketState,
    /// The bytes that are queued to be sent.
    pub tx_queue:       u64,
    /// The bytes that have been received but not read by the application yet. For a listening TCP socket, the kernel puts the length of the accept queue here instead.
    pub rx_queue:       u64,
    /// The UID of the owner of the socket.
    pub uid:            u32,
    /// The inode number of the socket, which is what a `/proc/PID/fd` entry points to.
    pub inode:          u64,
}

/// Parse an address that the kernel writes as a hexadecimal word per four bytes, followed by a colon and a hexadecimal port.
fn parse_socket_address(field: &[u8], ipv6: bool) -> Option<SocketAddr> {
    let colon_index = field.iter().position(|&b| b == b':')?;

    let (address, port) = (&field[..colon_index], &field[(colon_index + 1)..]);

    let port = u16::from_str_radix(from_utf8(port).ok()?, 16).ok()?;

    // Each word is the four address bytes read back as a native integer, so the native byte order turns them into bytes again on either endianness.
    let word = |index: usize| -> Option<[u8; 4]> {
        let word = address.get((index * 8)..((index * 8) + 8))?;

        Some(u32::from_str_radix(from_utf8(word).ok()?, 16).ok()?.to_ne_bytes())
    };

    let address = if ipv6 {
        let mut bytes = [0u8; 16];

        for index in 0..4 {
            bytes[(index * 4)..((index * 4) + 4)].copy_from_slice(&word(index)?);
        }

        IpAddr::V6(Ipv6Addr::from(bytes))
    } else {
        IpAddr::V4(Ipv4Addr::from(word(0)?))
    };

    Some(SocketAddr::new(address, port))
}

/// Parse the content of one of the socket files, whose first line only names the columns.
fn parse_socket_connections(
    data: &[u8],
    protocol: SocketProtocol,
) -> Result<Vec<SocketConnection>, Error> {
    let ipv6 = protocol.is_ipv6();

    let mut lines = ScannerU8SliceAscii::new(data);

    // The first line only names the columns.
    lines.drop_next_line()?;

    let mut connections = Vec::with_capacity(16);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        // The slot number comes first.
        if sc.drop_next()?.is_none() {
            continue;
        }

        let (
            Some(local_address),
            Some(remote_address),
            Some(state),
            Some(queues),
            Some(_timer),
            Some(_retransmit),
            Some(uid),
        ) = (sc.next()?, sc.next()?, sc.next()?, sc.next()?, sc.next()?, sc.next()?, sc.next()?)
        else {
            continue;
        };

        let (Some(local_address), Some(remote_address)) =
            (parse_socket_address(local_address, ipv6), parse_socket_address(remote_address, ipv6))
        else {
            continue;
        };

        let state = SocketState::from_raw(
            u8::from_str_radix(from_utf8(state).unwrap_or_default(), 16).unwrap_or(0),
        );

        // The two queue lengths share one field, as `tx_queue:rx_queue` in hexadecimal.
        let (tx_queue, rx_queue) = match queues.iter().position(|&b| b == b':') {
            Some(index) => (
                u64::from_str_radix(from_utf8(&queues[..index]).unwrap_or_default(), 16)
                    .unwrap_or(0),
                u64::from_str_radix(from_utf8(&queues[(index + 1)..]).unwrap_or_default(), 16)
                    .unwrap_or(0),
            ),
            None => (0, 0),
        };

        let uid = parse_number(uid)?;

        // The timeout comes between the UID and the inode.
        sc.drop_next()?;

        let Some(inode) = sc.next()? else {
            continue;
        };

        connections.push(SocketConnection {
            protocol,
            local_address,
            remote_address,
            state,
            tx_queue,
            rx_queue,
            uid,
            inode: parse_number(inode)?,
        });
    }

    Ok(connections)
}

/// Get every socket of one protocol by reading the file the kernel reports it in, e.g. the `/proc/net/tcp` file. An empty result is returned when the file does not exist, which is the case for the IPv6 files on a kernel without IPv6.
///
/// ```rust
/// use mprober_lib::network;
///
/// let connections =
///     network::get_socket_connections(network::SocketProtocol::Tcp).unwrap();
///
/// println!("{connections:#?}");
/// ```
pub fn get_socket_connections(protocol: SocketProtocol) -> Result<Vec<SocketConnection>, Error> {
    match read_file(protocol.path(), 64 * 1024) {
        Ok(data) => parse_socket_connections(&data, protocol),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

/// Get every TCP and UDP socket by reading the `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp` and `/proc/net/udp6` files, like the `ss -tuan` command. The kernel formats these files one socket at a time, so this gets expensive on a host with many connections and should not be called at a high frequency; the totals of [`crate::network::get_socket_stat`] are far cheaper when the individual sockets are not needed.
///
/// ```rust
/// use mprober_lib::network;
///
/// let connections = network::get_all_socket_connections().unwrap();
///
/// for connection in connections {
///     println!(
///         "{} {} {} {}",
///         connection.protocol,
///         connection.local_address,
///         connection.remote_address,
///         connection.state,
///     );
/// }
/// ```
pub fn get_all_socket_connections() -> Result<Vec<SocketConnection>, Error> {
    const PROTOCOLS: [SocketProtocol; 4] =
        [SocketProtocol::Tcp, SocketProtocol::Tcp6, SocketProtocol::Udp, SocketProtocol::Udp6];

    let mut connections = Vec::with_capacity(64);

    for protocol in PROTOCOLS {
        connections.append(&mut get_socket_connections(protocol)?);
    }

    Ok(connections)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TCP: &[u8] = b"  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000:445C 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 13280 1 0000000000000000 100 0 0 10 0
   1: 0100007F:44C3 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 23345 1 0000000000000000 100 0 0 10 0
   2: 0A01A8C0:D7EE 0A684FA0:01BB 01 00000024:00000012 00:00000000 00000000  1000        0 1228228 2 0000000000000000 0
";

    const TCP6: &[u8] = b"  sl  local_address                         remote_address                        st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000000000000000000000000000:0386 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 24194 1 0000000000000000 100 0 0 10 0
   1: 00000000000000000000000001000000:0277 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 22934 1 0000000000000000 100 0 0 10 0
";

    #[test]
    fn parse_tcp() {
        let connections = parse_socket_connections(TCP, SocketProtocol::Tcp).unwrap();

        assert_eq!(3, connections.len());

        assert_eq!("0.0.0.0:17500", connections[0].local_address.to_string());
        assert_eq!(SocketState::Listen, connections[0].state);
        assert_eq!(1000, connections[0].uid);
        assert_eq!(13280, connections[0].inode);

        // The address is written as a native integer per four bytes, not in network byte order.
        assert_eq!("127.0.0.1:17603", connections[1].local_address.to_string());

        assert_eq!("192.168.1.10:55278", connections[2].local_address.to_string());
        assert_eq!("160.79.104.10:443", connections[2].remote_address.to_string());
        assert_eq!(SocketState::Established, connections[2].state);
        assert_eq!(0x24, connections[2].tx_queue);
        assert_eq!(0x12, connections[2].rx_queue);
        assert_eq!(1228228, connections[2].inode);
    }

    #[test]
    fn parse_tcp6() {
        let connections = parse_socket_connections(TCP6, SocketProtocol::Tcp6).unwrap();

        assert_eq!(2, connections.len());

        assert_eq!("[::]:902", connections[0].local_address.to_string());
        assert_eq!(SocketState::Listen, connections[0].state);

        assert_eq!("[::1]:631", connections[1].local_address.to_string());
        assert_eq!(24194, connections[0].inode);
    }
}
