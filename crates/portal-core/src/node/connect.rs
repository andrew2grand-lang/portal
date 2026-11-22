//! Parsing what people type into "Connect by IP".

use std::net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV6};

use crate::consts::DEFAULT_PORT;
use crate::net::iface::{self, Iface};

/// Accepts `192.168.1.20`, `192.168.1.20:5000`, `fe80::1%en7`, `fe80::1%7`,
/// `[fe80::1%en7]:5000`, `2a00::1`, `[2a00::1]:5000`. Interface names are
/// matched against the system name or the friendly label.
pub fn parse_target(input: &str, ifaces: &[Iface]) -> Result<SocketAddr, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err("Type the other computer's IP address.".into());
    }
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, DEFAULT_PORT));
    }
    // IPv6 with a zone, optionally bracketed with a port.
    let (host, port) = match text.strip_prefix('[') {
        Some(rest) => {
            let (host, after) = rest.split_once(']').ok_or_else(invalid)?;
            let port = match after.strip_prefix(':') {
                Some(p) => p.parse::<u16>().map_err(|_| invalid())?,
                None if after.is_empty() => DEFAULT_PORT,
                None => return Err(invalid()),
            };
            (host, port)
        }
        None => (text, DEFAULT_PORT),
    };
    let (ip, zone) = host.split_once('%').ok_or_else(invalid)?;
    let ip: Ipv6Addr = ip.parse().map_err(|_| invalid())?;
    let scope = match zone.parse::<u32>() {
        Ok(index) => index,
        Err(_) => ifaces
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(zone) || i.label.eq_ignore_ascii_case(zone))
            .map(|i| i.index)
            .ok_or_else(|| format!("This computer has no network called “{zone}”."))?,
    };
    if !iface::is_link_local(&IpAddr::V6(ip)) {
        return Ok(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, 0)));
    }
    Ok(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, scope)))
}

fn invalid() -> String {
    "That doesn’t look like an IP address. Try something like 192.168.1.20.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::iface::IfaceKind;

    fn ifaces() -> Vec<Iface> {
        vec![Iface {
            index: 9,
            name: "en7".into(),
            label: "USB 10/100/1000 LAN".into(),
            kind: IfaceKind::Ethernet,
            v4: vec![],
            v6: vec!["fe80::1".parse().unwrap()],
            has_gateway: false,
            is_default: false,
        }]
    }

    #[test]
    fn plain_ipv4_uses_the_default_port() {
        assert_eq!(
            parse_target(" 192.168.1.20 ", &[]).unwrap(),
            format!("192.168.1.20:{DEFAULT_PORT}").parse().unwrap()
        );
    }

    #[test]
    fn explicit_ports_are_kept() {
        assert_eq!(parse_target("192.168.1.20:5000", &[]).unwrap(), "192.168.1.20:5000".parse().unwrap());
        assert_eq!(parse_target("[2a00::1]:5000", &[]).unwrap(), "[2a00::1]:5000".parse().unwrap());
    }

    #[test]
    fn link_local_ipv6_takes_a_zone_by_name_label_or_index() {
        let by_name = parse_target("fe80::2%en7", &ifaces()).unwrap();
        let by_label = parse_target("[fe80::2%usb 10/100/1000 lan]:6000", &ifaces()).unwrap();
        let by_index = parse_target("fe80::2%9", &ifaces()).unwrap();
        for (addr, port) in [(by_name, DEFAULT_PORT), (by_label, 6000), (by_index, DEFAULT_PORT)] {
            match addr {
                SocketAddr::V6(v6) => {
                    assert_eq!(v6.scope_id(), 9);
                    assert_eq!(v6.port(), port);
                }
                _ => panic!("expected v6"),
            }
        }
    }

    #[test]
    fn unknown_zones_and_garbage_get_a_plain_message() {
        assert!(parse_target("fe80::2%en99", &ifaces()).unwrap_err().contains("en99"));
        assert!(parse_target("desk-pc", &[]).unwrap_err().contains("IP address"));
        assert!(parse_target("", &[]).unwrap_err().contains("IP address"));
        assert!(parse_target("[fe80::1%en7]x", &ifaces()).is_err());
    }
}
