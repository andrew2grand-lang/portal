//! Which way to reach a peer (brief §6.6): classify each path, rank them,
//! and decide when a better one is worth switching to.

use std::net::{IpAddr, SocketAddr};

use crate::events::LinkKind;
use crate::net::iface::{self, Iface, IfaceKind};

/// What a path is, from our side: the interface it leaves through and
/// whether it's a direct link (a wired interface with no router, peer at a
/// link-local address). Every network has link-local IPv6, so a PC plugged
/// into the home router reaching the Mac's fe80:: address is plain Ethernet.
pub fn classify(ifaces: &[Iface], remote: &SocketAddr, local_ip: Option<IpAddr>, hint: Option<u32>) -> LinkKind {
    let by_local = local_ip.and_then(|ip| {
        ifaces.iter().find(|i| match ip {
            IpAddr::V4(v4) => i.v4.iter().any(|(a, _)| *a == v4),
            IpAddr::V6(v6) => i.v6.contains(&v6),
        })
    });
    let found = by_local.or_else(|| iface::iface_for(ifaces, remote, hint));
    match found.map(|i| i.kind) {
        Some(k) if k.is_wired() => {
            if iface::is_link_local(&remote.ip()) && !found.is_some_and(|i| i.has_gateway) {
                LinkKind::Cable
            } else {
                LinkKind::Ethernet
            }
        }
        Some(IfaceKind::Wifi) => LinkKind::Wifi,
        _ => LinkKind::Other,
    }
}

/// Lower is better: a direct link before Ethernet before Wi-Fi. On a direct
/// link IPv6 comes first (link-local IPv6 is there within seconds); through
/// a router IPv4 does (some routers filter the multicast IPv6 neighbour
/// discovery needs between wired and wireless).
pub fn rank(kind: LinkKind, remote: &SocketAddr) -> (LinkKind, u8) {
    let second_choice = if kind == LinkKind::Cable { remote.is_ipv4() } else { remote.is_ipv6() };
    (kind, second_choice as u8)
}

/// Only a strictly better kind of link is worth re-dialling for.
pub fn is_upgrade(current: LinkKind, candidate: LinkKind) -> bool {
    candidate < current
}

/// Two links to the same peer: keep the new one unless it's worse.
pub fn keep_new_link(old: (LinkKind, u8), new: (LinkKind, u8)) -> bool {
    new <= old
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ifaces() -> Vec<Iface> {
        vec![
            Iface {
                index: 5,
                name: "en0".into(),
                label: "Wi-Fi".into(),
                kind: IfaceKind::Wifi,
                v4: vec![("192.168.1.24".parse().unwrap(), 24)],
                v6: vec!["fe80::aaaa".parse().unwrap()],
                has_gateway: true,
                is_default: true,
            },
            Iface {
                index: 9,
                name: "en7".into(),
                label: "USB LAN".into(),
                kind: IfaceKind::Ethernet,
                v4: vec![("169.254.3.4".parse().unwrap(), 16)],
                v6: vec!["fe80::bbbb".parse().unwrap()],
                has_gateway: false,
                is_default: false,
            },
            Iface {
                index: 11,
                name: "en8".into(),
                label: "Office LAN".into(),
                kind: IfaceKind::Ethernet,
                v4: vec![("10.0.0.7".parse().unwrap(), 24)],
                v6: vec![],
                has_gateway: true,
                is_default: false,
            },
        ]
    }

    fn v6(addr: &str, scope: u32) -> SocketAddr {
        SocketAddr::V6(std::net::SocketAddrV6::new(addr.parse().unwrap(), 47920, 0, scope))
    }

    #[test]
    fn a_link_local_peer_over_a_wired_interface_is_a_cable() {
        let i = ifaces();
        assert_eq!(classify(&i, &v6("fe80::2", 9), None, None), LinkKind::Cable);
        assert_eq!(classify(&i, &"169.254.9.9:47920".parse().unwrap(), None, None), LinkKind::Cable);
        assert_eq!(
            classify(&i, &v6("fe80::2", 0), Some("fe80::bbbb".parse().unwrap()), None),
            LinkKind::Cable,
            "by our local address"
        );
    }

    #[test]
    fn a_wired_network_through_a_router_is_ethernet_not_cable() {
        assert_eq!(classify(&ifaces(), &"10.0.0.9:47920".parse().unwrap(), None, None), LinkKind::Ethernet);
    }

    #[test]
    fn a_link_local_peer_on_a_routed_wired_lan_is_ethernet_not_cable() {
        // A PC plugged into the home router sees the Mac's fe80:: address too.
        assert_eq!(classify(&ifaces(), &v6("fe80::2", 11), None, None), LinkKind::Ethernet);
        assert_eq!(
            classify(&ifaces(), &v6("fe80::2", 0), Some("10.0.0.7".parse().unwrap()), Some(11)),
            LinkKind::Ethernet
        );
    }

    #[test]
    fn link_local_over_wifi_is_still_wifi() {
        assert_eq!(classify(&ifaces(), &v6("fe80::2", 5), None, None), LinkKind::Wifi);
        assert_eq!(classify(&ifaces(), &"192.168.1.31:47920".parse().unwrap(), None, None), LinkKind::Wifi);
    }

    #[test]
    fn unknown_interfaces_are_other() {
        assert_eq!(classify(&[], &"127.0.0.1:47920".parse().unwrap(), None, None), LinkKind::Other);
    }

    #[test]
    fn a_direct_link_ranks_first_with_ipv6_and_routed_networks_prefer_ipv4() {
        let wifi_v4 = rank(LinkKind::Wifi, &"192.168.1.31:1".parse().unwrap());
        let wifi_v6 = rank(LinkKind::Wifi, &v6("fe80::2", 5));
        let mut paths = [
            wifi_v6,
            wifi_v4,
            rank(LinkKind::Cable, &"169.254.9.9:1".parse().unwrap()),
            rank(LinkKind::Ethernet, &"10.0.0.9:1".parse().unwrap()),
            rank(LinkKind::Cable, &v6("fe80::2", 9)),
        ];
        paths.sort();
        assert_eq!(paths[0], (LinkKind::Cable, 0), "IPv6 link-local first on a direct link");
        assert_eq!(paths[1], (LinkKind::Cable, 1));
        assert_eq!(paths[2].0, LinkKind::Ethernet);
        assert_eq!(paths[3], wifi_v4, "IPv4 first on a routed network");
        assert_eq!(paths[4], wifi_v6);
    }

    #[test]
    fn only_a_strictly_better_link_triggers_a_switch() {
        assert!(is_upgrade(LinkKind::Wifi, LinkKind::Cable));
        assert!(is_upgrade(LinkKind::Ethernet, LinkKind::Cable));
        assert!(!is_upgrade(LinkKind::Cable, LinkKind::Cable));
        assert!(!is_upgrade(LinkKind::Cable, LinkKind::Wifi));
    }

    #[test]
    fn a_duplicate_link_is_kept_only_if_it_is_not_worse() {
        assert!(keep_new_link((LinkKind::Wifi, 1), (LinkKind::Cable, 0)));
        assert!(keep_new_link((LinkKind::Wifi, 1), (LinkKind::Wifi, 1)), "ties keep the newest");
        assert!(!keep_new_link((LinkKind::Cable, 0), (LinkKind::Wifi, 1)));
    }
}
