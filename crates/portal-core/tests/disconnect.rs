//! "Disconnect": stop connecting to a paired device without forgetting it.
mod common;

use std::time::Duration;

use common::{TestNode, free_tcp_port, pair, start, start_in};
use portal_core::events::Snapshot;

fn the_peer(s: &Snapshot) -> &portal_core::events::PeerView {
    s.peers.first().expect("a paired device")
}

/// Waits `secs` and checks the link never came back meanwhile.
async fn stays_apart(node: &mut TestNode, secs: u64) {
    let until = tokio::time::Instant::now() + Duration::from_secs(secs);
    while tokio::time::Instant::now() < until {
        let s = node.core.snapshot().await.unwrap();
        assert!(!the_peer(&s).online, "{}: reconnected while disconnected", node.name);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn paired() -> (TestNode, TestNode, u16) {
    let mac_port = free_tcp_port();
    let mut mac = start("MacBook Pro", |c| c.port = mac_port).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    (mac, pc, mac_port)
}

#[tokio::test]
async fn disconnecting_cuts_the_link_keeps_the_pairing_and_nothing_reconnects() {
    let (mut mac, mut pc, _) = paired().await;
    let pc_id = pc.id().await;
    mac.core.set_connected(&pc_id, false).await;

    let m = mac.wait_for("mac shows it's disconnected", |s| the_peer(s).disconnected && !the_peer(s).online).await;
    assert!(the_peer(&m).paired, "still paired");
    pc.wait_for("pc lost the link", |s| !the_peer(s).online).await;
    // The PC keeps trying (it doesn't know), and is turned away each time.
    stays_apart(&mut mac, 4).await;
    let p = pc.core.snapshot().await.unwrap();
    assert!(the_peer(&p).paired, "the PC still trusts the Mac");
    assert!(
        p.doctor.iter().any(|c| c.id == "remote-disconnected" && c.title.contains("MacBook Pro")),
        "the PC says why: {:?}",
        p.doctor.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
    assert!(mac.drain_logs().iter().any(|l| l.contains("Disconnected from Desk PC")));
}

#[tokio::test]
async fn sending_to_a_disconnected_device_says_so() {
    let (mut mac, pc, _) = paired().await;
    let pc_id = pc.id().await;
    mac.core.set_connected(&pc_id, false).await;
    mac.wait_for("disconnected", |s| the_peer(s).disconnected).await;
    let file = mac.dir.path().join("a.txt");
    std::fs::write(&file, b"hi").unwrap();
    let err = mac.core.send(&pc_id, vec![file]).await.unwrap_err();
    assert!(err.contains("Disconnected") && err.contains("Connect"), "{err}");
}

#[tokio::test]
async fn it_stays_disconnected_after_a_restart_and_connect_needs_no_code() {
    let (mut mac, mut pc, mac_port) = paired().await;
    let pc_id = pc.id().await;
    mac.core.set_connected(&pc_id, false).await;
    mac.wait_for("disconnected", |s| the_peer(s).disconnected).await;

    mac.core.shutdown().await;
    let mut mac = start_in("MacBook Pro", mac.dir, |c| c.port = mac_port).await;
    let m = mac.wait_for("remembered", |s| !s.peers.is_empty()).await;
    assert!(the_peer(&m).disconnected && the_peer(&m).paired);
    stays_apart(&mut mac, 3).await;

    mac.core.set_connected(&pc_id, true).await;
    let m = mac.wait_for("connected again", |s| the_peer(s).online && !the_peer(s).disconnected).await;
    assert!(m.pairing.is_none(), "no code needed");
    let p = pc.wait_for("pc connected again", |s| the_peer(s).online).await;
    assert!(!p.doctor.iter().any(|c| c.id == "remote-disconnected"), "the PC's warning clears");
}
