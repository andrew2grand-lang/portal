//! Dialling over link-local addresses (brief §6.6). macOS gives `lo0` the
//! link-local address fe80::1, which lets these run without a cable.

#![cfg(target_os = "macos")]

use std::net::{IpAddr, SocketAddr, SocketAddrV6};
use std::time::Duration;

use portal_core::net::dial::{self, LocalBind};

fn lo0_index() -> u32 {
    netdev::get_interfaces().into_iter().find(|i| i.name == "lo0").map(|i| i.index).expect("lo0")
}

#[tokio::test]
async fn ipv6_link_local_dials_carry_the_interface_scope() {
    let scope = lo0_index();
    let listener = std::net::TcpListener::bind(SocketAddrV6::new("fe80::1".parse().unwrap(), 0, 0, scope)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let scoped = SocketAddr::V6(SocketAddrV6::new("fe80::1".parse().unwrap(), port, 0, scope));

    let stream = dial::dial(scoped, None, Duration::from_secs(3)).await.expect("scoped dial works");
    assert_eq!(stream.peer_addr().unwrap().ip(), scoped.ip());

    let unscoped = SocketAddr::V6(SocketAddrV6::new("fe80::1".parse().unwrap(), port, 0, 0));
    assert!(
        dial::dial(unscoped, None, Duration::from_secs(3)).await.is_err(),
        "without the scope the OS can't route it"
    );
}

#[tokio::test]
async fn a_dial_can_be_pinned_to_one_interface_and_source_address() {
    // The 169.254 cable path binds its source address and IP_BOUND_IF; on
    // loopback the same calls must succeed and leave from that address.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let bind = LocalBind { ip: IpAddr::from([127, 0, 0, 1]), if_index: lo0_index() };
    let stream = dial::dial(addr, Some(bind), Duration::from_secs(3)).await.expect("bound dial works");
    assert_eq!(stream.local_addr().unwrap().ip(), bind.ip);
}
