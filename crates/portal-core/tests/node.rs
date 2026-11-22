//! Two cores on one machine (loopback, different ports).

mod common;

use common::{free_udp_port, start};
use portal_core::model::Os;

#[tokio::test]
async fn connect_by_ip_finds_the_other_device_and_it_sees_us_back() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;

    let peer = pc.core.connect(&mac.addr()).await.expect("connect by IP");

    assert_eq!(peer.name, "MacBook Pro");
    assert_eq!(peer.os, Os::current());
    assert!(!peer.paired);
    let snap = pc.wait_for("mac listed", |s| s.peers.iter().any(|p| p.name == "MacBook Pro" && p.online)).await;
    assert_eq!(snap.peers.len(), 1);
    mac.wait_for("pc listed via inbound", |s| s.peers.iter().any(|p| p.name == "Desk PC")).await;
}

#[tokio::test]
async fn connect_by_ip_accepts_an_address_without_a_port_when_the_default_is_used() {
    // Parsing is unit-tested; here: a clear error when nothing answers.
    let pc = start("Desk PC", |_| {}).await;
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let err = pc.core.connect(&format!("127.0.0.1:{free}")).await.unwrap_err();
    assert!(err.contains("Nothing answered"), "{err}");
}

#[tokio::test]
async fn connecting_to_ourselves_is_refused_politely() {
    let pc = start("Desk PC", |_| {}).await;
    let err = pc.core.connect(&pc.addr()).await.unwrap_err();
    assert!(err.contains("this device"), "{err}");
}

#[tokio::test]
async fn devices_on_the_same_network_appear_without_typing_anything() {
    let (ua, ub) = (free_udp_port(), free_udp_port());
    let mut mac = start("MacBook Pro", |c| {
        c.broadcast = true;
        c.udp_port = ua;
        c.extra_targets = vec![format!("127.0.0.1:{ub}").parse().unwrap()];
    })
    .await;
    let mut pc = start("Desk PC", |c| {
        c.broadcast = true;
        c.udp_port = ub;
        c.extra_targets = vec![format!("127.0.0.1:{ua}").parse().unwrap()];
    })
    .await;

    let started = std::time::Instant::now();
    pc.wait_for("mac discovered", |s| s.peers.iter().any(|p| p.name == "MacBook Pro")).await;
    mac.wait_for("pc discovered", |s| s.peers.iter().any(|p| p.name == "Desk PC")).await;
    assert!(started.elapsed().as_secs_f32() < 5.0, "brief §4.2: visible within 5 s");
}

#[tokio::test]
async fn the_snapshot_describes_this_device() {
    let pc = start("Desk PC", |_| {}).await;
    let s = pc.core.snapshot().await.unwrap();
    assert_eq!(s.me.name, "Desk PC");
    assert!(s.me.port > 0);
    assert!(s.peers.is_empty());
}

#[tokio::test]
async fn the_doctor_checklist_is_in_the_snapshot_while_nothing_is_found() {
    let mut pc = start("Desk PC", |_| {}).await;
    let s = pc.wait_for("doctor", |s| !s.doctor.is_empty()).await;
    assert!(s.doctor.iter().any(|c| c.id == "peer-open"), "{:?}", s.doctor);
    // Problems first, then unknowns, then passes.
    let order: Vec<_> = s.doctor.iter().map(|c| c.status).collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);
}

/// Mission 04 §1.7: Appearance is System until changed, and it's kept.
#[tokio::test]
async fn appearance_follows_the_system_until_changed_and_is_kept() {
    use portal_core::model::Appearance;
    use portal_core::node::SettingsPatch;

    let mut mac = common::start("MacBook Pro", |_| {}).await;
    assert_eq!(mac.core.snapshot().await.unwrap().me.appearance, Appearance::System);
    mac.core.update_settings(SettingsPatch { appearance: Some(Appearance::Dark), ..Default::default() }).await;
    mac.wait_for("dark", |s| s.me.appearance == Appearance::Dark).await;

    let dir = mac.dir;
    mac.core.shutdown().await;
    let again = common::start_in("MacBook Pro", dir, |_| {}).await;
    assert_eq!(again.core.snapshot().await.unwrap().me.appearance, Appearance::Dark);
}
