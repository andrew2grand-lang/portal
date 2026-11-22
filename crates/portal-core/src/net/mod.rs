//! Networking: interfaces, dialling, links and discovery.

pub mod dial;
pub mod discovery;
pub mod iface;
pub mod link;
pub mod listen;

/// Apple TN3179: macOS only asks for Local Network access when an app first
/// touches the local network. Call this at launch: it sends one UDP datagram
/// to a link-local IPv6 address on each interface, then repeats twice, since
/// the first attempt can be refused before the person has answered.
pub fn trigger_local_network_prompt() {
    std::thread::spawn(|| {
        for delay in [0u64, 2, 6] {
            std::thread::sleep(std::time::Duration::from_secs(delay));
            for i in iface::snapshot() {
                if !i.v6.iter().any(|a| iface::is_link_local(&std::net::IpAddr::V6(*a))) {
                    continue;
                }
                let target =
                    std::net::SocketAddrV6::new(std::net::Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1), 9, 0, i.index);
                if let Ok(sock) = std::net::UdpSocket::bind("[::]:0") {
                    let _ = sock.connect(target).and_then(|_| sock.send(b"portal"));
                }
            }
        }
    });
}
