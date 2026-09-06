use std::{
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::from_utf8,
};

use scanner_rust::ScannerU8SliceAscii;

use crate::{
    Error,
    utils::{parse_number, read_file},
};

/// One entry of the routing table, read from the `/proc/net/route` file or the `/proc/net/ipv6_route` file.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Route {
    /// The name of the interface the packets leave through.
    pub interface:     String,
    /// The network this route covers. It is unspecified (`0.0.0.0` or `::`) for the default route.
    pub destination:   IpAddr,
    /// The prefix length of the destination network, e.g. `24` for a `255.255.255.0` mask.
    pub prefix_length: u8,
    /// The next hop. It is unspecified when the destination is directly reachable.
    pub gateway:       IpAddr,
    /// The route flags (`RTF_*` in `route(8)`), e.g. `0x0003` for a usable route through a gateway.
    pub flags:         u32,
    /// The metric of the route. A lower one is preferred.
    pub metric:        u32,
}

impl Route {
    /// Whether this is a default route, which covers every destination.
    #[inline]
    pub fn is_default(&self) -> bool {
        self.prefix_length == 0
    }

    /// Whether this route only rejects the packets that match it (`RTF_REJECT`). The kernel keeps such an entry as a fallback for every address family that has no real route, so it never carries traffic.
    #[inline]
    pub fn is_reject(&self) -> bool {
        // `RTF_REJECT` is `0x0200` in `route.h`.
        self.flags & 0x0200 != 0
    }
}

/// Parse an address that the kernel writes as one hexadecimal word per four bytes.
fn parse_hex_ipv4(field: &[u8]) -> Option<Ipv4Addr> {
    // The word is the four address bytes read back as a native integer, so the native byte order turns them into bytes again on either endianness.
    let value = u32::from_str_radix(from_utf8(field).ok()?, 16).ok()?;

    Some(Ipv4Addr::from(value.to_ne_bytes()))
}

/// Parse the content of `/proc/net/route`, whose first line only names the columns.
fn parse_routes(data: &[u8]) -> Result<Vec<Route>, Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    // The first line only names the columns.
    lines.drop_next_line()?;

    let mut routes = Vec::with_capacity(8);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        let (
            Some(interface),
            Some(destination),
            Some(gateway),
            Some(flags),
            Some(_ref_count),
            Some(_use_count),
            Some(metric),
            Some(mask),
        ) = (
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
        )
        else {
            continue;
        };

        let (Some(destination), Some(gateway), Some(mask)) =
            (parse_hex_ipv4(destination), parse_hex_ipv4(gateway), parse_hex_ipv4(mask))
        else {
            continue;
        };

        routes.push(Route {
            interface:     String::from_utf8_lossy(interface).into_owned(),
            destination:   IpAddr::V4(destination),
            prefix_length: u32::from(mask).count_ones() as u8,
            gateway:       IpAddr::V4(gateway),
            flags:         u32::from_str_radix(from_utf8(flags).unwrap_or_default(), 16)
                .unwrap_or(0),
            metric:        parse_number(metric).unwrap_or(0),
        });
    }

    Ok(routes)
}

/// Parse the content of `/proc/net/ipv6_route`, which has no header line and writes every address as 32 hexadecimal digits.
fn parse_ipv6_routes(data: &[u8]) -> Result<Vec<Route>, Error> {
    let mut lines = ScannerU8SliceAscii::new(data);

    let mut routes = Vec::with_capacity(8);

    while let Some(line) = lines.next_line()? {
        let mut sc = ScannerU8SliceAscii::new(line);

        let (
            Some(destination),
            Some(prefix_length),
            Some(_source),
            Some(_source_prefix_length),
            Some(gateway),
            Some(metric),
            Some(_ref_count),
            Some(_use_count),
            Some(flags),
            Some(interface),
        ) = (
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
            sc.next()?,
        )
        else {
            continue;
        };

        // Unlike the IPv4 file, this one writes the bytes in order, so they need no reordering.
        let parse_address = |field: &[u8]| -> Option<Ipv6Addr> {
            let mut bytes = [0u8; 16];

            for (index, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(
                    from_utf8(field.get((index * 2)..((index * 2) + 2))?).ok()?,
                    16,
                )
                .ok()?;
            }

            Some(Ipv6Addr::from(bytes))
        };

        let (Some(destination), Some(gateway)) =
            (parse_address(destination), parse_address(gateway))
        else {
            continue;
        };

        routes.push(Route {
            interface:     String::from_utf8_lossy(interface).into_owned(),
            destination:   IpAddr::V6(destination),
            prefix_length: u8::from_str_radix(from_utf8(prefix_length).unwrap_or_default(), 16)
                .unwrap_or(0),
            gateway:       IpAddr::V6(gateway),
            flags:         u32::from_str_radix(from_utf8(flags).unwrap_or_default(), 16)
                .unwrap_or(0),
            metric:        u32::from_str_radix(from_utf8(metric).unwrap_or_default(), 16)
                .unwrap_or(0),
        });
    }

    Ok(routes)
}

/// Get the routing table by reading the `/proc/net/route` file and the `/proc/net/ipv6_route` file, like the `route -n` command. The IPv6 file is skipped when IPv6 is not available.
///
/// ```rust
/// use mprober_lib::network;
///
/// let routes = network::get_routes().unwrap();
///
/// println!("{routes:#?}");
/// ```
pub fn get_routes() -> Result<Vec<Route>, Error> {
    let mut routes = parse_routes(&read_file("/proc/net/route", 4096)?)?;

    match read_file("/proc/net/ipv6_route", 8192) {
        Ok(data) => routes.append(&mut parse_ipv6_routes(&data)?),
        Err(err) if err.kind() == ErrorKind::NotFound => (),
        Err(err) => return Err(err.into()),
    }

    Ok(routes)
}

/// Get the default routes, which are the ones that cover every destination, ordered by their metric so that the preferred one comes first. There is normally one for IPv4 and one for IPv6. The unreachable fallback entries the kernel keeps for an address family that has no real route are left out.
///
/// The files this reads only hold the main routing table, so a host that sends its traffic through a policy rule (`ip rule`) or another table, as a VPN normally does, legitimately has no default route here.
///
/// ```rust
/// use mprober_lib::network;
///
/// let default_routes = network::get_default_routes().unwrap();
///
/// for route in default_routes {
///     println!("{} via {}", route.interface, route.gateway);
/// }
/// ```
pub fn get_default_routes() -> Result<Vec<Route>, Error> {
    let mut routes: Vec<Route> = get_routes()?
        .into_iter()
        .filter(|route| route.is_default() && !route.is_reject())
        .collect();

    routes.sort_by_key(|route| route.metric);

    Ok(routes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUTE: &[u8] =
        b"Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
enp131s0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
enp131s0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0
";

    #[test]
    fn parse() {
        let routes = parse_routes(ROUTE).unwrap();

        assert_eq!(3, routes.len());

        assert_eq!("enp131s0", routes[0].interface);
        assert_eq!("0.0.0.0", routes[0].destination.to_string());
        assert_eq!("192.168.1.1", routes[0].gateway.to_string());
        assert_eq!(0, routes[0].prefix_length);
        assert_eq!(0x0003, routes[0].flags);
        assert_eq!(100, routes[0].metric);
        assert!(routes[0].is_default());

        assert_eq!("172.17.0.0", routes[1].destination.to_string());
        assert_eq!(16, routes[1].prefix_length);
        assert!(!routes[1].is_default());

        assert_eq!("192.168.1.0", routes[2].destination.to_string());
        assert_eq!(24, routes[2].prefix_length);
    }
}
