//! A trusted but malicious peer must not write outside the receive folder
//! (brief §8: reject path traversal).

mod common;

use std::time::Duration;

use common::start_in;
use portal_core::identity::Identity;
use portal_core::model::{Os, Role};
use portal_core::net::dial;
use portal_core::net::link::{self, Inbound};
use portal_core::proto::{self, B32, CancelReason, Hello, ItemKind, Msg, Offer, OfferItem};
use portal_core::store::{Store, TrustedPeer};
use portal_core::tls;
use tokio::sync::mpsc;

async fn attack(path: Vec<String>) -> (tempfile::TempDir, Option<Msg>) {
    let evil_dir = tempfile::tempdir().unwrap();
    let evil = Identity::load_or_create(evil_dir.path()).unwrap();
    // The victim trusts the attacker and always accepts from it.
    let victim_dir = tempfile::tempdir().unwrap();
    Store::open(victim_dir.path())
        .unwrap()
        .upsert_peer(TrustedPeer {
            id: evil.device_id.clone(),
            name: "Evil".into(),
            os: Os::Windows,
            fingerprint: evil.fingerprint,
            addrs: vec![],
            trusted: true,
            incoming: portal_core::model::Incoming::Always,
            paired_at: 0,
            renamed: false,
            disconnected: false,
            clip_sync: false,
            clip_sync_at: 0,
            shelf_share: false,
            shelf_share_at: 0,
            shelf_answered: false,
            clip_to: Default::default(),
            folder: None,
        })
        .unwrap();
    let victim = start_in("Victim", victim_dir, |_| {}).await;
    let victim_id = Identity::load_or_create(victim.dir.path()).unwrap();

    let tcp = dial::dial(victim.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls = link::connect_tls(tcp, tls::client_config(&evil, Some(victim_id.fingerprint)).unwrap()).await.unwrap();
    let hello = Hello {
        proto: proto::PROTOCOL_VERSION,
        id: evil.device_id.clone(),
        name: "Evil".into(),
        os: Os::Windows,
        role: Role::Both,
        app: "x".into(),
        port: 1,
    };
    let (events_tx, mut events) = mpsc::channel(64);
    let link = link::handshake(tls, true, hello, events_tx).await.unwrap();
    let (sink_tx, mut sink) = mpsc::channel(64);
    link.set_sink(Some(sink_tx));

    let tid = 7;
    link.send(Msg::Offer(Offer {
        tid,
        items: vec![OfferItem { name: "x".into(), kind: ItemKind::File, bytes: 5, files: 1 }],
        total_bytes: 5,
        file_count: 1,
        dir_count: 0,
        skipped: vec![],
        sender_role: Role::Both,
    }))
    .await
    .unwrap();
    // Wait for the automatic accept.
    let accepted = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(ev) = events.recv().await {
            if let link::LinkEvent::Msg(_, Msg::Accept { .. }) = ev {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    assert!(accepted, "victim auto-accepts from a trusted device");

    link.send_ordered(Msg::File { tid, idx: 0, path, size: 5, mtime: None, from: 0 }).await.unwrap();
    link.send_data(b"pwned".to_vec()).await.unwrap();
    link.send_ordered(Msg::FileEnd { tid, idx: 0, blake3: B32(*blake3::hash(b"pwned").as_bytes()) }).await.unwrap();
    link.send_ordered(Msg::Done { tid }).await.unwrap();

    let reply = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(m) = sink.recv().await {
            if let Inbound::Msg(m @ (Msg::Cancel { .. } | Msg::Complete { .. })) = m {
                return Some(m);
            }
        }
        None
    })
    .await
    .unwrap_or(None);
    (victim.dir, reply)
}

/// Traversal from `<victim>/Received/.seamdrop-incoming/<transfer>/` could land
/// anywhere up the tree: check the victim folder, its parent and grandparent.
fn nothing_escaped(victim: &std::path::Path) {
    fn walk(dir: &std::path::Path, depth: usize, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.file_name().is_some_and(|n| n == "evil.txt") {
                    out.push(p.clone());
                }
                if depth > 0 && p.is_dir() {
                    walk(&p, depth - 1, out);
                }
            }
        }
    }
    let mut found = Vec::new();
    walk(victim, 8, &mut found);
    for up in victim.ancestors().skip(1).take(2) {
        walk(up, 0, &mut found);
    }
    assert!(found.is_empty(), "attacker wrote {found:?}");
}

#[tokio::test]
async fn dot_dot_paths_are_refused_and_nothing_is_written() {
    for path in [
        vec!["..".to_string(), "..".into(), "evil.txt".into()],
        vec!["a/../../evil.txt".to_string()],
        vec!["a\\..\\..\\evil.txt".to_string()],
        vec!["C:".to_string(), "evil.txt".into()],
        vec!["".to_string(), "evil.txt".into()],
        vec![".".to_string(), "evil.txt".into()],
    ] {
        let (victim, reply) = attack(path.clone()).await;
        match reply {
            Some(Msg::Cancel { reason: CancelReason::Error { code, .. }, .. }) => {
                assert_eq!(code, "protocol", "{path:?}")
            }
            other => panic!("{path:?}: expected a protocol cancel, got {other:?}"),
        }
        nothing_escaped(victim.path());
        let staging = victim.path().join("Received/.seamdrop-incoming");
        let clean = || !staging.exists() || std::fs::read_dir(&staging).unwrap().next().is_none();
        for _ in 0..50 {
            if clean() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(clean(), "staging cleaned");
    }
}
