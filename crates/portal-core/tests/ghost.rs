//! A device claiming a paired peer's id, but holding a different key, must
//! never be treated as that peer (security review finding 1).

mod common;

use common::Transfers;

use std::time::Duration;

use common::{pair, paired_online, start, start_in};
use portal_core::events::{Direction, TransferPhase};
use portal_core::identity::Identity;
use portal_core::model::{Os, Role};
use portal_core::net::dial;
use portal_core::net::link::{self, LinkEvent};
use portal_core::proto::{self, Hello, ItemKind, Msg, Offer, OfferItem};
use portal_core::tls;
use tokio::sync::mpsc;

/// Two data folders whose identities make the first one the follower (larger
/// id): the case where the duplicate-link rule would not close the ghost.
fn follower_and_leader_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let ida = Identity::load_or_create(a.path()).unwrap().device_id;
    let idb = Identity::load_or_create(b.path()).unwrap().device_id;
    if ida > idb { (a, b) } else { (b, a) }
}

#[tokio::test]
async fn a_ghost_link_opened_before_pairing_never_inherits_trust() {
    let (victim_dir, real_dir) = follower_and_leader_dirs();
    let mut victim = start_in("Victim", victim_dir, |_| {}).await;
    let mut real = start_in("Real B", real_dir, |_| {}).await;
    let real_id = real.id().await;
    assert!(victim.id().await > real_id, "victim is the follower");

    // The attacker read B's id from mDNS and connects first, claiming to be B.
    let ghost_dir = tempfile::tempdir().unwrap();
    let ghost_identity = Identity::load_or_create(ghost_dir.path()).unwrap();
    let tcp = dial::dial(victim.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls_stream = link::connect_tls(tcp, tls::client_config(&ghost_identity, None).unwrap()).await.unwrap();
    let hello = Hello {
        proto: proto::PROTOCOL_VERSION,
        id: real_id.clone(),
        name: "Real B".into(),
        os: Os::Windows,
        role: Role::Both,
        app: "x".into(),
        port: 1,
    };
    let (ghost_tx, mut ghost_events) = mpsc::channel(64);
    let ghost = link::handshake(tls_stream, true, hello, ghost_tx).await.unwrap();
    victim.wait_for("ghost seen", |s| s.peers.iter().any(|p| p.name == "Real B")).await;

    // Then the victim genuinely pairs with the real B.
    pair(&mut victim, &mut real).await;

    // The ghost is cut off once B's real key is pinned...
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(ev) = ghost_events.recv().await {
            if matches!(ev, LinkEvent::Closed(..)) {
                return true;
            }
        }
        true
    })
    .await
    .unwrap_or(false);
    assert!(closed, "the victim closes the ghost link");

    // ...nothing it sends is accepted...
    let _ = ghost
        .send(Msg::Offer(Offer {
            tid: 9,
            items: vec![OfferItem { name: "x".into(), kind: ItemKind::File, bytes: 1, files: 1 }],
            total_bytes: 1,
            file_count: 1,
            dir_count: 0,
            skipped: vec![],
            sender_role: Role::Both,
        }))
        .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(victim.core.snapshot().await.unwrap().transfer().is_none(), "no offer from the ghost");

    // ...and the victim's own sends go to the real B.
    let file = victim.dir.path().join("secret.txt");
    std::fs::write(&file, b"for B only").unwrap();
    victim.wait_for("still paired", paired_online).await;
    victim.core.send(&real_id, vec![file]).await.unwrap();
    let offer = real
        .wait_for("real B gets the offer", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Offered))
        .await;
    assert_eq!(offer.transfer().cloned().unwrap().direction, Direction::In);
}

#[tokio::test]
async fn untrusted_connections_from_one_address_are_capped() {
    let victim = start("Victim", |_| {}).await;
    let mut links = Vec::new();
    let mut refused = 0;
    for _ in 0..12 {
        let dir = tempfile::tempdir().unwrap();
        let id = Identity::load_or_create(dir.path()).unwrap();
        let tcp = dial::dial(victim.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
        let tls_stream = link::connect_tls(tcp, tls::client_config(&id, None).unwrap()).await.unwrap();
        let hello = Hello {
            proto: proto::PROTOCOL_VERSION,
            id: id.device_id.clone(),
            name: "Spam".into(),
            os: Os::Windows,
            role: Role::Both,
            app: "x".into(),
            port: 1,
        };
        let (tx, rx) = mpsc::channel(8);
        match link::handshake(tls_stream, true, hello, tx).await {
            Ok(l) => links.push((l, rx, dir)),
            Err(_) => refused += 1,
        }
    }
    tokio::time::sleep(Duration::from_millis(500)).await;
    let open = links.iter().filter(|(l, _, _)| !l.is_closed()).count();
    assert!(
        open <= portal_core::node::MAX_UNTRUSTED_PER_ADDR,
        "{open} untrusted links stayed open ({refused} refused)"
    );
}
