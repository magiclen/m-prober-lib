use std::{
    collections::HashMap,
    ffi::CStr,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ptr,
};

use crate::Error;

/// One IP address of a network interface.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NetworkAddress {
    /// The IPv4 or IPv6 address.
    pub address:       IpAddr,
    /// The prefix length of the subnet, e.g. `24` for `192.168.1.10/24`.
    pub prefix_length: u8,
}

/// Convert a `sockaddr` to an `IpAddr`. It returns `None` for a null pointer and for address families other than `AF_INET` and `AF_INET6`.
///
/// # Safety
///
/// `addr` must be null or point to a `sockaddr` whose real type matches its `sa_family`, which is what `getifaddrs` guarantees.
unsafe fn sockaddr_to_ip_addr(addr: *const libc::sockaddr) -> Option<IpAddr> {
    if addr.is_null() {
        return None;
    }

    match i32::from(unsafe { (*addr).sa_family }) {
        libc::AF_INET => {
            let addr = unsafe { &*(addr as *const libc::sockaddr_in) };

            // `s_addr` is in network byte order, which is the order of the bytes in memory.
            Some(IpAddr::V4(Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes())))
        },
        libc::AF_INET6 => {
            let addr = unsafe { &*(addr as *const libc::sockaddr_in6) };

            Some(IpAddr::V6(Ipv6Addr::from(addr.sin6_addr.s6_addr)))
        },
        _ => None,
    }
}

/// Get the IP addresses of all network interfaces using the `getifaddrs` function in libc. The keys are interface names, and an interface without an IP address is not included.
///
/// ```rust
/// use mprober_lib::network;
///
/// let network_addresses = network::get_network_addresses().unwrap();
///
/// println!("{network_addresses:#?}");
/// ```
pub fn get_network_addresses() -> Result<HashMap<String, Vec<NetworkAddress>>, Error> {
    let mut ifap: *mut libc::ifaddrs = ptr::null_mut();

    if unsafe { libc::getifaddrs(&mut ifap) } != 0 {
        return Err(io::Error::last_os_error().into());
    }

    let mut addresses: HashMap<String, Vec<NetworkAddress>> = HashMap::new();

    let mut ifa = ifap;

    // The list is valid until `freeifaddrs` is called after this loop.
    while !ifa.is_null() {
        let entry = unsafe { &*ifa };

        if let Some(address) = unsafe { sockaddr_to_ip_addr(entry.ifa_addr) } {
            let prefix_length = match unsafe { sockaddr_to_ip_addr(entry.ifa_netmask) } {
                Some(IpAddr::V4(mask)) => u32::from(mask).count_ones() as u8,
                Some(IpAddr::V6(mask)) => u128::from(mask).count_ones() as u8,
                None => 0,
            };

            let interface =
                unsafe { CStr::from_ptr(entry.ifa_name) }.to_string_lossy().into_owned();

            addresses.entry(interface).or_default().push(NetworkAddress {
                address,
                prefix_length,
            });
        }

        ifa = entry.ifa_next;
    }

    unsafe { libc::freeifaddrs(ifap) };

    Ok(addresses)
}
