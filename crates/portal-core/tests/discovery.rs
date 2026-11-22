//! Discovery: mDNS and the UDP broadcast fallback (brief §6.1).

use std::net::SocketAddr;
use std::time::Duration;

use portal_core::model::{DeviceId, Os};
use portal_core::net::discovery::{self, Announce, DiscoveryConfig, Sighting, Via};
use tokio::sync::mpsc;

fn free_udp_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn config(name: &str, tcp_port: u16, udp_port: u16) -> DiscoveryConfig {
    DiscoveryConfig {
        id: DeviceId::random(),
        name: name.into(),
        os: Os::current(),
        port: tcp_port,
        udp_port,
        mdns: false,
        broadcast: true,
        extra_targets: vec![],
        loopback: true,
    }
}

async fn first_sighting_of(rx: &mut mpsc::Receiver<Sighting>, id: &DeviceId, via: Via, wait: Duration) -> Sighting {
    tokio::time::timeout(wait, async {
        loop {
            let s = rx.recv().await.expect("discovery stopped");
            if &s.id == id && s.via == via {
                return s;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no {via:?} sighting within {wait:?}"))
}

#[test]
fn announcements_round_trip_and_garbage_is_ignored() {
    let a = Announce {
        proto: 1,
        id: DeviceId::random(),
        name: "Desk PC".into(),
        os: Os::Windows,
        port: 47920,
        uport: 47920,
        reply: false,
    };
    assert_eq!(discovery::decode_announce(&discovery::encode_announce(&a)), Some(a));
    assert_eq!(discovery::decode_announce(b"hello"), None);
    assert_eq!(discovery::decode_announce(b"PORTAL1\n{not json"), None);
    assert_eq!(discovery::decode_announce(&[0u8; 2000]), None);
}

#[tokio::test]
async fn two_devices_find_each_other_by_udp_broadcast_and_ignore_themselves() {
    let (pa, pb) = (free_udp_port(), free_udp_port());
    let mut ca = config("Mac", 5001, pa);
    let mut cb = config("PC", 5002, pb);
    // Loopback has no broadcast; aim each side at the other directly.
    ca.extra_targets = vec![format!("127.0.0.1:{pb}").parse().unwrap()];
    cb.extra_targets = vec![format!("127.0.0.1:{pa}").parse().unwrap()];
    let (ida, idb) = (ca.id.clone(), cb.id.clone());
    let (txa, mut rxa) = mpsc::channel(64);
    let (txb, mut rxb) = mpsc::channel(64);
    let _a = discovery::start(ca, txa).await.unwrap();
    let _b = discovery::start(cb, txb).await.unwrap();

    let seen_by_a = first_sighting_of(&mut rxa, &idb, Via::Broadcast, Duration::from_secs(5)).await;
    let seen_by_b = first_sighting_of(&mut rxb, &ida, Via::Broadcast, Duration::from_secs(5)).await;

    assert_eq!(seen_by_a.name, "PC");
    assert_eq!(seen_by_a.addr, "127.0.0.1:5002".parse::<SocketAddr>().unwrap());
    assert_eq!(seen_by_b.addr, "127.0.0.1:5001".parse::<SocketAddr>().unwrap());
    // Nobody reports itself.
    while let Ok(s) = rxa.try_recv() {
        assert_ne!(s.id, ida);
    }
}

#[tokio::test]
async fn a_new_device_gets_an_immediate_unicast_reply() {
    // B announces only once (no extra targets on B) but A replies, so B still learns about A.
    let (pa, pb) = (free_udp_port(), free_udp_port());
    let ca = config("Mac", 5001, pa);
    let mut cb = config("PC", 5002, pb);
    cb.extra_targets = vec![format!("127.0.0.1:{pa}").parse().unwrap()];
    let ida = ca.id.clone();
    let (txa, _rxa) = mpsc::channel(64);
    let (txb, mut rxb) = mpsc::channel(64);
    let _a = discovery::start(ca, txa).await.unwrap();
    let _b = discovery::start(cb, txb).await.unwrap();
    let seen = first_sighting_of(&mut rxb, &ida, Via::Broadcast, Duration::from_secs(5)).await;
    assert_eq!(seen.name, "Mac");
}

/// Real multicast between two daemons on this machine. Needs a network
/// interface with multicast (not available on every CI runner).
#[tokio::test]
#[ignore = "needs a multicast-capable interface; run with --ignored"]
async fn two_devices_find_each_other_by_mdns() {
    let mut ca = config("Mac", 5101, free_udp_port());
    let mut cb = config("PC", 5102, free_udp_port());
    for c in [&mut ca, &mut cb] {
        c.mdns = true;
        c.broadcast = false;
        c.loopback = false;
    }
    let (ida, idb) = (ca.id.clone(), cb.id.clone());
    let (txa, mut rxa) = mpsc::channel(256);
    let (txb, mut rxb) = mpsc::channel(256);
    let _a = discovery::start(ca, txa).await.unwrap();
    let _b = discovery::start(cb, txb).await.unwrap();
    let seen_by_a = first_sighting_of(&mut rxa, &idb, Via::Mdns, Duration::from_secs(5)).await;
    let seen_by_b = first_sighting_of(&mut rxb, &ida, Via::Mdns, Duration::from_secs(5)).await;
    assert_eq!(seen_by_a.name, "PC");
    assert_eq!(seen_by_a.addr.port(), 5102);
    assert_eq!(seen_by_b.addr.port(), 5101);
}
