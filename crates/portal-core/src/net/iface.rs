//! Network interfaces: what kind each one is (brief §6.6) and which ones we
//! use for discovery and dialling.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use serde::{Deserialize, Serialize};

/// How a local interface reaches the network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IfaceKind {
    Ethernet,
    /// Thunderbolt Bridge (macOS) or USB4 host-to-host (Windows). Experimental.
    Bridge,
    Wifi,
    Vpn,
    Loopback,
    Other,
}

impl IfaceKind {
    pub fn is_wired(self) -> bool {
        matches!(self, IfaceKind::Ethernet | IfaceKind::Bridge)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Iface {
    pub index: u32,
    /// BSD name on macOS (`en0`), adapter GUID on Windows.
    pub name: String,
    /// "Wi-Fi", "Ethernet 2" (Windows) or the macOS display name.
    pub label: String,
    pub kind: IfaceKind,
    pub v4: Vec<(Ipv4Addr, u8)>,
    pub v6: Vec<Ipv6Addr>,
    pub has_gateway: bool,
    pub is_default: bool,
}

/// What `classify` needs to know, independent of netdev (so it's testable).
#[derive(Clone, Debug, Default)]
pub struct RawIface<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub friendly: &'a str,
    pub netdev_type: &'a str,
    pub has_ipv4: bool,
    pub is_default: bool,
    pub is_loopback: bool,
}

const VPN_WORDS: &[&str] = &[
    "vpn",
    "wireguard",
    "wintun",
    "tap-windows",
    "openvpn",
    "anyconnect",
    "tailscale",
    "zerotier",
    "fortinet",
    "globalprotect",
    "pangp",
    "nordlynx",
    "juniper",
    "pulse secure",
    "sonicwall",
];

/// Classifies an interface. On macOS a `utun`/`ipsec`/`ppp` interface only
/// counts as a VPN when it has an IPv4 address or carries the default route:
/// macOS always has several idle `utun`s (iCloud Private Relay, Continuity).
pub fn classify(raw: &RawIface<'_>) -> IfaceKind {
    let name = raw.name.to_ascii_lowercase();
    let text = format!("{} {}", raw.description, raw.friendly).to_ascii_lowercase();
    if raw.is_loopback || raw.netdev_type == "Loopback" {
        return IfaceKind::Loopback;
    }
    let tunnel_name = ["utun", "ipsec", "ppp", "tun", "tap", "wg"].iter().any(|p| name.starts_with(p));
    let tunnel_type = matches!(raw.netdev_type, "Tunnel" | "Ppp" | "ProprietaryVirtual");
    if VPN_WORDS.iter().any(|w| text.contains(w)) {
        return IfaceKind::Vpn;
    }
    if tunnel_name || tunnel_type {
        return if raw.has_ipv4 || raw.is_default { IfaceKind::Vpn } else { IfaceKind::Other };
    }
    if text.contains("usb4")
        || text.contains("thunderbolt")
        || (raw.netdev_type == "Bridge" && name.starts_with("bridge"))
    {
        return IfaceKind::Bridge;
    }
    match raw.netdev_type {
        "Wireless80211" => IfaceKind::Wifi,
        "Ethernet" | "GigabitEthernet" | "FastEthernetT" | "FastEthernetFx" | "Ethernet3Megabit" => IfaceKind::Ethernet,
        _ => IfaceKind::Other,
    }
}

/// Interfaces we never use: Apple peer-to-peer Wi-Fi (AWDL, low-latency
/// WLAN, the access-point interface) and inactive tunnels.
pub fn is_ignored_name(name: &str) -> bool {
    ["awdl", "llw", "ap", "gif", "stf", "anpi", "utun"].iter().any(|p| name.starts_with(p))
        && !name.starts_with("apple")
}

/// The directed broadcast address of `addr/prefix` (e.g. 192.168.1.255,
/// 169.254.255.255).
pub fn directed_broadcast(addr: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    if prefix >= 32 {
        return addr;
    }
    let mask = u32::MAX.checked_shl(32 - prefix as u32).unwrap_or(0);
    Ipv4Addr::from(u32::from(addr) | !mask)
}

pub fn is_link_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_link_local(),
        IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

pub fn same_subnet(a: Ipv4Addr, b: Ipv4Addr, prefix: u8) -> bool {
    let mask = u32::MAX.checked_shl(32 - prefix.min(32) as u32).unwrap_or(0);
    (u32::from(a) & mask) == (u32::from(b) & mask)
}

/// Current interfaces that are up, with addresses, minus ignored ones.
#[cfg(target_os = "ios")]
pub fn snapshot() -> Vec<Iface> {
    getifaddrs_snapshot()
}

/// Current interfaces that are up, with addresses, minus ignored ones.
#[cfg(not(target_os = "ios"))]
pub fn snapshot() -> Vec<Iface> {
    netdev::get_interfaces()
        .into_iter()
        .filter(|i| i.is_up() && !is_ignored_name(&i.name))
        .map(|i| {
            let netdev_type = format!("{:?}", i.if_type);
            let kind = classify(&RawIface {
                name: &i.name,
                description: i.description.as_deref().unwrap_or(""),
                friendly: i.friendly_name.as_deref().unwrap_or(""),
                netdev_type: &netdev_type,
                has_ipv4: !i.ipv4.is_empty(),
                is_default: i.default,
                is_loopback: i.is_loopback(),
            });
            Iface {
                index: i.index,
                label: i.friendly_name.clone().unwrap_or_else(|| i.name.clone()),
                name: i.name.clone(),
                kind,
                v4: i.ipv4.iter().map(|n| (n.addr(), n.prefix_len())).collect(),
                v6: i.ipv6.iter().map(|n| n.addr()).collect(),
                has_gateway: i.gateway.is_some() && kind != IfaceKind::Vpn || i.default,
                is_default: i.default,
            }
        })
        .filter(|i| !i.v4.is_empty() || !i.v6.is_empty())
        .collect()
}

/// The same from the system's own list (`getifaddrs`), for where netdev
/// doesn't build (iOS). It knows no interface types, so only loopback and
/// VPN-looking names are classified; everything else is Other.
#[cfg(unix)]
pub fn getifaddrs_snapshot() -> Vec<Iface> {
    use std::collections::BTreeMap;
    let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills a linked list that is freed below, and nothing
    // from it is kept past that (names and addresses are copied out).
    if unsafe { libc::getifaddrs(&mut list) } != 0 {
        return Vec::new();
    }
    let mut by_name: BTreeMap<String, Iface> = BTreeMap::new();
    let mut cur = list;
    while !cur.is_null() {
        // SAFETY: `cur` is a node of the list getifaddrs returned.
        let ifa = unsafe { &*cur };
        cur = ifa.ifa_next;
        if ifa.ifa_addr.is_null() || ifa.ifa_name.is_null() {
            continue;
        }
        let flags = ifa.ifa_flags as i32;
        if flags & libc::IFF_UP == 0 {
            continue;
        }
        // SAFETY: a NUL-terminated name owned by the list.
        let name = unsafe { std::ffi::CStr::from_ptr(ifa.ifa_name) }.to_string_lossy().into_owned();
        if is_ignored_name(&name) {
            continue;
        }
        let loopback = flags & libc::IFF_LOOPBACK != 0;
        let entry = by_name.entry(name.clone()).or_insert_with(|| Iface {
            // SAFETY: the same valid name.
            index: unsafe { libc::if_nametoindex(ifa.ifa_name) },
            label: name.clone(),
            kind: classify(&RawIface { name: &name, is_loopback: loopback, ..Default::default() }),
            name: name.clone(),
            v4: Vec::new(),
            v6: Vec::new(),
            has_gateway: false,
            is_default: false,
        });
        // SAFETY: ifa_addr is non-null; its family says which struct it is.
        match i32::from(unsafe { (*ifa.ifa_addr).sa_family }) {
            libc::AF_INET => {
                // SAFETY: AF_INET means a sockaddr_in (and so is the netmask).
                let sin = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in) };
                let prefix = if ifa.ifa_netmask.is_null() {
                    24
                } else {
                    // SAFETY: as above.
                    let mask = unsafe { &*(ifa.ifa_netmask as *const libc::sockaddr_in) };
                    u32::from_be(mask.sin_addr.s_addr).count_ones() as u8
                };
                entry.v4.push((Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)), prefix));
                entry.has_gateway |= !loopback;
            }
            libc::AF_INET6 => {
                // SAFETY: AF_INET6 means a sockaddr_in6.
                let sin6 = unsafe { &*(ifa.ifa_addr as *const libc::sockaddr_in6) };
                entry.v6.push(Ipv6Addr::from(sin6.sin6_addr.s6_addr));
            }
            _ => {}
        }
    }
    // SAFETY: the list from getifaddrs above, freed once.
    unsafe { libc::freeifaddrs(list) };
    by_name.into_values().filter(|i| !i.v4.is_empty() || !i.v6.is_empty()).collect()
}

/// The local interface a peer address is reached through: the scope id for
/// IPv6 link-local, otherwise the interface whose IPv4 subnet contains it.
pub fn iface_for<'a>(ifaces: &'a [Iface], addr: &SocketAddr, hint: Option<u32>) -> Option<&'a Iface> {
    if let Some(index) = hint.filter(|i| *i != 0)
        && let Some(found) = ifaces.iter().find(|i| i.index == index)
    {
        return Some(found);
    }
    match addr {
        SocketAddr::V6(v6) if v6.scope_id() != 0 => ifaces.iter().find(|i| i.index == v6.scope_id()),
        SocketAddr::V4(v4) => ifaces.iter().find(|i| i.v4.iter().any(|(a, p)| same_subnet(*a, *v4.ip(), *p))),
        SocketAddr::V6(v6) => ifaces.iter().find(|i| i.v6.iter().any(|a| a.segments()[..4] == v6.ip().segments()[..4])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw<'a>(name: &'a str, netdev_type: &'a str) -> RawIface<'a> {
        RawIface { name, netdev_type, ..Default::default() }
    }

    #[test]
    fn wifi_and_ethernet_come_from_the_os_type_not_the_name() {
        assert_eq!(classify(&raw("en0", "Wireless80211")), IfaceKind::Wifi);
        assert_eq!(classify(&raw("en0", "Ethernet")), IfaceKind::Ethernet);
        assert_eq!(classify(&raw("en7", "Ethernet")), IfaceKind::Ethernet);
        assert_eq!(classify(&raw("{4D36E972-...}", "GigabitEthernet")), IfaceKind::Ethernet);
    }

    #[test]
    fn idle_macos_utun_interfaces_are_not_vpns() {
        assert_eq!(classify(&raw("utun3", "Tunnel")), IfaceKind::Other);
    }

    #[test]
    fn a_utun_with_ipv4_or_the_default_route_is_a_vpn() {
        let mut r = raw("utun4", "Tunnel");
        r.has_ipv4 = true;
        assert_eq!(classify(&r), IfaceKind::Vpn);
        let mut r = raw("ipsec0", "Unknown");
        r.is_default = true;
        assert_eq!(classify(&r), IfaceKind::Vpn);
    }

    #[test]
    fn windows_vpn_adapters_are_recognised_by_description() {
        let r =
            RawIface { name: "{GUID}", description: "WireGuard Tunnel", netdev_type: "Unknown", ..Default::default() };
        assert_eq!(classify(&r), IfaceKind::Vpn);
        let r = RawIface {
            name: "{GUID}",
            description: "TAP-Windows Adapter V9",
            netdev_type: "Ethernet",
            ..Default::default()
        };
        assert_eq!(classify(&r), IfaceKind::Vpn);
    }

    #[test]
    fn thunderbolt_bridge_and_usb4_networking_are_bridges() {
        assert_eq!(classify(&raw("bridge0", "Bridge")), IfaceKind::Bridge);
        let r = RawIface {
            name: "{GUID}",
            description: "USB4(TM) P2P Network Adapter",
            netdev_type: "Ethernet",
            ..Default::default()
        };
        assert_eq!(classify(&r), IfaceKind::Bridge);
    }

    #[test]
    fn loopback_is_loopback() {
        let mut r = raw("lo0", "Loopback");
        r.is_loopback = true;
        assert_eq!(classify(&r), IfaceKind::Loopback);
    }

    #[test]
    fn apple_peer_to_peer_interfaces_are_ignored() {
        for n in ["awdl0", "llw0", "ap1", "utun0", "gif0", "stf0", "anpi0"] {
            assert!(is_ignored_name(n), "{n}");
        }
        for n in ["en0", "en7", "bridge0", "lo0"] {
            assert!(!is_ignored_name(n), "{n}");
        }
    }

    #[test]
    fn directed_broadcast_covers_home_and_link_local_subnets() {
        assert_eq!(directed_broadcast("192.168.1.200".parse().unwrap(), 24), Ipv4Addr::new(192, 168, 1, 255));
        assert_eq!(directed_broadcast("169.254.12.34".parse().unwrap(), 16), Ipv4Addr::new(169, 254, 255, 255));
        assert_eq!(directed_broadcast("10.1.2.3".parse().unwrap(), 8), Ipv4Addr::new(10, 255, 255, 255));
        assert_eq!(directed_broadcast("10.1.2.3".parse().unwrap(), 32), Ipv4Addr::new(10, 1, 2, 3));
        assert_eq!(directed_broadcast("10.1.2.3".parse().unwrap(), 0), Ipv4Addr::BROADCAST);
    }

    #[test]
    fn link_local_detection_covers_both_families() {
        assert!(is_link_local(&"169.254.1.1".parse().unwrap()));
        assert!(is_link_local(&"fe80::1".parse().unwrap()));
        assert!(!is_link_local(&"192.168.1.1".parse().unwrap()));
        assert!(!is_link_local(&"2a00::1".parse().unwrap()));
    }

    #[test]
    fn a_peer_address_maps_to_the_interface_that_reaches_it() {
        let ifaces = vec![
            Iface {
                index: 5,
                name: "en0".into(),
                label: "Wi-Fi".into(),
                kind: IfaceKind::Wifi,
                v4: vec![("192.168.1.200".parse().unwrap(), 24)],
                v6: vec![],
                has_gateway: true,
                is_default: true,
            },
            Iface {
                index: 9,
                name: "en7".into(),
                label: "USB LAN".into(),
                kind: IfaceKind::Ethernet,
                v4: vec![("169.254.3.4".parse().unwrap(), 16)],
                v6: vec!["fe80::1".parse().unwrap()],
                has_gateway: false,
                is_default: false,
            },
        ];
        let wifi_peer: SocketAddr = "192.168.1.20:47920".parse().unwrap();
        let cable_v4: SocketAddr = "169.254.9.9:47920".parse().unwrap();
        let cable_v6 = SocketAddr::V6(std::net::SocketAddrV6::new("fe80::2".parse().unwrap(), 47920, 0, 9));
        assert_eq!(iface_for(&ifaces, &wifi_peer, None).unwrap().index, 5);
        assert_eq!(iface_for(&ifaces, &cable_v4, None).unwrap().index, 9);
        assert_eq!(iface_for(&ifaces, &cable_v6, None).unwrap().index, 9);
        assert_eq!(iface_for(&ifaces, &wifi_peer, Some(9)).unwrap().index, 9, "an explicit hint wins");
    }
}

/// The getifaddrs list (used on iOS) agrees with netdev's on this computer:
/// the same interfaces with the same IPv4 addresses and prefixes.
#[cfg(all(test, unix, not(target_os = "ios")))]
mod getifaddrs_tests {
    #[test]
    fn getifaddrs_finds_what_netdev_finds() {
        let ours = super::getifaddrs_snapshot();
        let theirs = super::snapshot();
        for t in theirs.iter().filter(|t| !t.v4.is_empty()) {
            let o = ours.iter().find(|o| o.name == t.name).unwrap_or_else(|| panic!("{} missing: {ours:#?}", t.name));
            let (mut a, mut b) = (o.v4.clone(), t.v4.clone());
            a.sort();
            b.sort();
            assert_eq!(a, b, "{}", t.name);
            assert_eq!(o.index, t.index, "{}", t.name);
        }
        assert!(
            ours.iter().any(|i| i.kind == super::IfaceKind::Loopback)
                || !theirs.iter().any(|i| i.kind == super::IfaceKind::Loopback)
        );
    }
}
