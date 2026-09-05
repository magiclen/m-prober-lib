use std::io::ErrorKind;

use crate::{
    Error,
    utils::{parse_number, read_file},
};

/// The protocol counters read from the `/proc/net/snmp` file and the `/proc/net/netstat` file, like the `netstat -s` command. Every field counts since boot, and a counter the kernel does not report is `0`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProtocolStat {
    /// `Ip: InReceives`, the datagrams received from the interfaces.
    pub ip_in_receives:       u64,
    /// `Ip: InDiscards`, the incoming datagrams dropped for a reason other than an error, e.g. a lack of buffer space.
    pub ip_in_discards:       u64,
    /// `Ip: InDelivers`, the incoming datagrams handed to the upper protocols.
    pub ip_in_delivers:       u64,
    /// `Ip: OutRequests`, the datagrams the upper protocols asked to send.
    pub ip_out_requests:      u64,
    /// `Ip: OutDiscards`, the outgoing datagrams dropped for a reason other than an error.
    pub ip_out_discards:      u64,
    /// `Ip: OutNoRoutes`, the outgoing datagrams dropped because there was no route.
    pub ip_out_no_routes:     u64,
    /// `Icmp: InMsgs`, the ICMP messages received.
    pub icmp_in_msgs:         u64,
    /// `Icmp: InErrors`, the ICMP messages received with an error.
    pub icmp_in_errors:       u64,
    /// `Icmp: OutMsgs`, the ICMP messages sent.
    pub icmp_out_msgs:        u64,
    /// `Icmp: OutErrors`, the ICMP messages that could not be sent.
    pub icmp_out_errors:      u64,
    /// `Tcp: ActiveOpens`, the connections this host opened.
    pub tcp_active_opens:     u64,
    /// `Tcp: PassiveOpens`, the connections this host accepted.
    pub tcp_passive_opens:    u64,
    /// `Tcp: AttemptFails`, the connection attempts that failed.
    pub tcp_attempt_fails:    u64,
    /// `Tcp: EstabResets`, the established connections that were reset.
    pub tcp_estab_resets:     u64,
    /// `Tcp: CurrEstab`, the connections that are established right now. This is a gauge, not a counter.
    pub tcp_curr_estab:       u64,
    /// `Tcp: InSegs`, the segments received.
    pub tcp_in_segs:          u64,
    /// `Tcp: OutSegs`, the segments sent.
    pub tcp_out_segs:         u64,
    /// `Tcp: RetransSegs`, the segments retransmitted. Its ratio to `tcp_out_segs` is the retransmission rate.
    pub tcp_retrans_segs:     u64,
    /// `Tcp: InErrs`, the segments received with an error, e.g. a bad checksum.
    pub tcp_in_errs:          u64,
    /// `Tcp: OutRsts`, the segments sent with the `RST` flag.
    pub tcp_out_rsts:         u64,
    /// `Udp: InDatagrams`, the datagrams delivered to the applications.
    pub udp_in_datagrams:     u64,
    /// `Udp: NoPorts`, the datagrams received for a port nothing was listening on.
    pub udp_no_ports:         u64,
    /// `Udp: InErrors`, the datagrams that could not be delivered for a reason other than a missing port.
    pub udp_in_errors:        u64,
    /// `Udp: OutDatagrams`, the datagrams sent.
    pub udp_out_datagrams:    u64,
    /// `Udp: RcvbufErrors`, the datagrams dropped because the receive buffer of the socket was full.
    pub udp_rcvbuf_errors:    u64,
    /// `Udp: SndbufErrors`, the datagrams dropped because the send buffer of the socket was full.
    pub udp_sndbuf_errors:    u64,
    /// `TcpExt: ListenOverflows`, the connections dropped because the accept queue of a listening socket was full. Anything above zero means a server is not accepting fast enough.
    pub tcp_listen_overflows: u64,
    /// `TcpExt: ListenDrops`, the connections dropped from a listening socket for any reason, which includes the overflows.
    pub tcp_listen_drops:     u64,
    /// `TcpExt: TCPTimeouts`, the retransmission timers that fired.
    pub tcp_timeouts:         u64,
    /// `TcpExt: TCPLostRetransmit`, the retransmitted segments that were lost again.
    pub tcp_lost_retransmit:  u64,
    /// `TcpExt: TCPSynRetrans`, the `SYN` segments retransmitted, which means a connection attempt got no answer.
    pub tcp_syn_retrans:      u64,
    /// `TcpExt: TCPAbortOnTimeout`, the connections aborted because a timer kept firing.
    pub tcp_abort_on_timeout: u64,
}

/// Parse a file that writes a header line of names and then a line of values, both starting with the same protocol label.
fn parse_protocol_stat(data: &[u8], stat: &mut ProtocolStat) -> Result<(), Error> {
    let mut lines = data.split(|&b| b == b'\n');

    while let (Some(header), Some(values)) = (lines.next(), lines.next()) {
        let mut names = header.split(|b| b.is_ascii_whitespace()).filter(|token| !token.is_empty());
        let mut values =
            values.split(|b| b.is_ascii_whitespace()).filter(|token| !token.is_empty());

        let (Some(protocol), Some(value_protocol)) = (names.next(), values.next()) else {
            continue;
        };

        // A truncated file can leave a header without its values, and then the pairing is off.
        if protocol != value_protocol {
            continue;
        }

        for (name, value) in names.zip(values) {
            // A few counters are signed, e.g. `Tcp: MaxConn` is `-1` when there is no limit.
            let Ok(value) = parse_number::<i64>(value) else {
                continue;
            };

            let value = value.max(0) as u64;

            match (protocol, name) {
                (b"Ip:", b"InReceives") => stat.ip_in_receives = value,
                (b"Ip:", b"InDiscards") => stat.ip_in_discards = value,
                (b"Ip:", b"InDelivers") => stat.ip_in_delivers = value,
                (b"Ip:", b"OutRequests") => stat.ip_out_requests = value,
                (b"Ip:", b"OutDiscards") => stat.ip_out_discards = value,
                (b"Ip:", b"OutNoRoutes") => stat.ip_out_no_routes = value,
                (b"Icmp:", b"InMsgs") => stat.icmp_in_msgs = value,
                (b"Icmp:", b"InErrors") => stat.icmp_in_errors = value,
                (b"Icmp:", b"OutMsgs") => stat.icmp_out_msgs = value,
                (b"Icmp:", b"OutErrors") => stat.icmp_out_errors = value,
                (b"Tcp:", b"ActiveOpens") => stat.tcp_active_opens = value,
                (b"Tcp:", b"PassiveOpens") => stat.tcp_passive_opens = value,
                (b"Tcp:", b"AttemptFails") => stat.tcp_attempt_fails = value,
                (b"Tcp:", b"EstabResets") => stat.tcp_estab_resets = value,
                (b"Tcp:", b"CurrEstab") => stat.tcp_curr_estab = value,
                (b"Tcp:", b"InSegs") => stat.tcp_in_segs = value,
                (b"Tcp:", b"OutSegs") => stat.tcp_out_segs = value,
                (b"Tcp:", b"RetransSegs") => stat.tcp_retrans_segs = value,
                (b"Tcp:", b"InErrs") => stat.tcp_in_errs = value,
                (b"Tcp:", b"OutRsts") => stat.tcp_out_rsts = value,
                (b"Udp:", b"InDatagrams") => stat.udp_in_datagrams = value,
                (b"Udp:", b"NoPorts") => stat.udp_no_ports = value,
                (b"Udp:", b"InErrors") => stat.udp_in_errors = value,
                (b"Udp:", b"OutDatagrams") => stat.udp_out_datagrams = value,
                (b"Udp:", b"RcvbufErrors") => stat.udp_rcvbuf_errors = value,
                (b"Udp:", b"SndbufErrors") => stat.udp_sndbuf_errors = value,
                (b"TcpExt:", b"ListenOverflows") => stat.tcp_listen_overflows = value,
                (b"TcpExt:", b"ListenDrops") => stat.tcp_listen_drops = value,
                (b"TcpExt:", b"TCPTimeouts") => stat.tcp_timeouts = value,
                (b"TcpExt:", b"TCPLostRetransmit") => stat.tcp_lost_retransmit = value,
                (b"TcpExt:", b"TCPSynRetrans") => stat.tcp_syn_retrans = value,
                (b"TcpExt:", b"TCPAbortOnTimeout") => stat.tcp_abort_on_timeout = value,
                _ => (),
            }
        }
    }

    Ok(())
}

/// Get the protocol counters by reading the `/proc/net/snmp` file and the `/proc/net/netstat` file, like the `netstat -s` command. The `Ip`, `Icmp` and `Udp` counters only cover IPv4, because the kernel keeps the IPv6 ones in the `/proc/net/snmp6` file, while the `Tcp` and `TcpExt` counters cover both address families, because TCP shares one set of counters. The retransmission ratio and the listen queue overflows these report are the usual first look at a network problem, which the per-interface counters of [`crate::network::get_networks`] cannot show.
///
/// ```rust
/// use mprober_lib::network;
///
/// let protocol_stat = network::get_protocol_stat().unwrap();
///
/// println!("{protocol_stat:#?}");
/// ```
pub fn get_protocol_stat() -> Result<ProtocolStat, Error> {
    let mut stat = ProtocolStat::default();

    parse_protocol_stat(&read_file("/proc/net/snmp", 4096)?, &mut stat)?;

    // The extended counters live in a separate file, which a very old kernel may not have.
    match read_file("/proc/net/netstat", 8192) {
        Ok(data) => parse_protocol_stat(&data, &mut stat)?,
        Err(err) if err.kind() == ErrorKind::NotFound => (),
        Err(err) => return Err(err.into()),
    }

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNMP: &[u8] = b"Ip: Forwarding DefaultTTL InReceives InHdrErrors InAddrErrors ForwDatagrams InUnknownProtos InDiscards InDelivers OutRequests OutDiscards OutNoRoutes
Ip: 1 64 1815278 0 0 0 0 7 1809541 624321 20 3
Icmp: InMsgs InErrors InCsumErrors InDestUnreachs OutMsgs OutErrors
Icmp: 359 1 0 359 363 2
Tcp: RtoAlgorithm RtoMin RtoMax MaxConn ActiveOpens PassiveOpens AttemptFails EstabResets CurrEstab InSegs OutSegs RetransSegs InErrs OutRsts
Tcp: 1 200 120000 -1 34064 2233 402 1152 34 1170485 1201853 203 5 1936
Udp: InDatagrams NoPorts InErrors OutDatagrams RcvbufErrors SndbufErrors
Udp: 34586 341 12 35090 12 0
";

    const NETSTAT: &[u8] = b"TcpExt: ListenOverflows ListenDrops TCPTimeouts TCPLostRetransmit TCPSynRetrans TCPAbortOnTimeout
TcpExt: 4 9 392 287 352 27
IpExt: InNoRoutes InTruncatedPkts
IpExt: 0 0
";

    #[test]
    fn parse() {
        let mut stat = ProtocolStat::default();

        parse_protocol_stat(SNMP, &mut stat).unwrap();
        parse_protocol_stat(NETSTAT, &mut stat).unwrap();

        assert_eq!(1815278, stat.ip_in_receives);
        assert_eq!(7, stat.ip_in_discards);
        assert_eq!(624321, stat.ip_out_requests);
        assert_eq!(3, stat.ip_out_no_routes);

        assert_eq!(359, stat.icmp_in_msgs);
        assert_eq!(1, stat.icmp_in_errors);
        assert_eq!(2, stat.icmp_out_errors);

        assert_eq!(34064, stat.tcp_active_opens);
        assert_eq!(2233, stat.tcp_passive_opens);
        assert_eq!(402, stat.tcp_attempt_fails);
        assert_eq!(34, stat.tcp_curr_estab);
        assert_eq!(1201853, stat.tcp_out_segs);
        assert_eq!(203, stat.tcp_retrans_segs);
        assert_eq!(5, stat.tcp_in_errs);

        assert_eq!(341, stat.udp_no_ports);
        assert_eq!(12, stat.udp_in_errors);
        assert_eq!(12, stat.udp_rcvbuf_errors);

        assert_eq!(4, stat.tcp_listen_overflows);
        assert_eq!(9, stat.tcp_listen_drops);
        assert_eq!(392, stat.tcp_timeouts);
        assert_eq!(287, stat.tcp_lost_retransmit);
        assert_eq!(27, stat.tcp_abort_on_timeout);
    }
}
