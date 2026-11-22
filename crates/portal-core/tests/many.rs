//! Mission 04 §3: as many devices as you have. Four cores in one process
//! (loopback only): pairing all of them, sending to two at once, receiving
//! from two at once with free space reserved, a drop and a resume while the
//! others carry on, a restart with two paused transfers, and the clipboard
//! to one, to several, and synced automatically with two.

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use common::{TestNode, start, start_in};
use portal_core::clip::{Contents, MemoryClipboard, Read};
use portal_core::events::{Direction, Snapshot, TransferPhase, TransferView};
use portal_core::model::{DeviceId, Incoming};

const MB: usize = 1_000_000;

struct Device {
    node: TestNode,
    clip: Arc<MemoryClipboard>,
}

async fn device(name: &str, tweak: impl FnOnce(&mut portal_core::Config)) -> Device {
    let clip = Arc::new(MemoryClipboard::default());
    let c = clip.clone();
    let node = start(name, move |cfg| {
        cfg.clipboard = c;
        tweak(cfg);
    })
    .await;
    Device { node, clip }
}

fn online_with(s: &Snapshot, id: &DeviceId) -> bool {
    s.peers.iter().any(|p| &p.id == id && p.paired && p.online)
}

/// `b` pairs with `a`: `b` asks, `a` shows the code, it's typed on `b`.
async fn pair_with(a: &mut TestNode, b: &mut TestNode) {
    let (ida, idb) = (a.id().await, b.id().await);
    b.core.connect(&a.addr()).await.unwrap();
    let code = common::ask_and_show(a, b).await;
    b.core.pairing(portal_core::node::PairAction::Code(code)).await.unwrap();
    a.wait_for("paired", |s| online_with(s, &idb)).await;
    b.wait_for("paired", |s| online_with(s, &ida)).await;
}

fn write_file(path: &Path, len: usize, seed: u8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let data: Vec<u8> = (0..len).map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed)).collect();
    std::fs::write(path, data).unwrap();
}

fn with(s: &Snapshot, peer: &DeviceId) -> Option<TransferView> {
    s.transfers.iter().rev().find(|t| &t.peer_id == peer).cloned()
}

fn phase_with(s: &Snapshot, peer: &DeviceId) -> Option<TransferPhase> {
    with(s, peer).map(|t| t.phase)
}

async fn always_from(receiver: &TestNode, sender: &TestNode) {
    receiver.core.set_incoming(&sender.id().await, Incoming::Always).await;
}

async fn same_file(a: &Path, b: &Path) {
    for _ in 0..100 {
        if std::fs::read(b).ok().as_deref() == std::fs::read(a).ok().as_deref() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{} differs from {}", b.display(), a.display());
}

#[tokio::test]
async fn four_computers_pair_with_each_other() {
    let mut d: Vec<TestNode> = Vec::new();
    for name in ["Studio Mac", "Desk PC", "MacBook Air", "Old Laptop"] {
        d.push(start(name, |_| {}).await);
    }
    for i in 0..4 {
        for j in (i + 1)..4 {
            let (left, right) = d.split_at_mut(j);
            pair_with(&mut left[i], &mut right[0]).await;
        }
    }
    for n in &mut d {
        let s = n
            .wait_for("three paired, connected", |s| s.peers.iter().filter(|p| p.paired && p.online).count() == 3)
            .await;
        assert_eq!(s.peers.iter().filter(|p| p.paired).count(), 3, "{}: {:?}", n.name, s.peers);
    }
}

#[tokio::test]
async fn sending_to_two_at_once_runs_side_by_side() {
    let mut mac = device("Studio Mac", |c| c.fault_rate = Some(6 * MB as u64)).await;
    let mut pc = device("Desk PC", |_| {}).await;
    let mut air = device("MacBook Air", |_| {}).await;
    pair_with(&mut pc.node, &mut mac.node).await;
    pair_with(&mut air.node, &mut mac.node).await;
    always_from(&pc.node, &mac.node).await;
    always_from(&air.node, &mac.node).await;
    let src = mac.node.dir.path().join("out/photos.bin");
    write_file(&src, 18 * MB, 1);
    let (to_pc, to_air) = (pc.node.id().await, air.node.id().await);
    let results = mac.node.core.send_many(vec![to_pc.clone(), to_air.clone()], vec![src.clone()]).await;
    assert!(results.iter().all(|(_, r)| r.is_ok()), "{results:?}");

    // Both run at the same time: one transfer each, not one after the other.
    let both = mac
        .node
        .wait_for("both running", |s| {
            phase_with(s, &to_pc) == Some(TransferPhase::Running)
                && phase_with(s, &to_air) == Some(TransferPhase::Running)
        })
        .await;
    assert_eq!(both.transfers.len(), 2);
    let long = Duration::from_secs(60);
    mac.node
        .wait_for_within(long, "both done", |s| {
            phase_with(s, &to_pc) == Some(TransferPhase::Done) && phase_with(s, &to_air) == Some(TransferPhase::Done)
        })
        .await;
    same_file(&src, &pc.node.received().join("photos.bin")).await;
    same_file(&src, &air.node.received().join("photos.bin")).await;

    // One at a time per device: a second send to a busy device says so.
    let busy_src = mac.node.dir.path().join("out/more.bin");
    write_file(&busy_src, 30 * MB, 2);
    mac.node.core.send(&to_pc, vec![busy_src.clone()]).await.unwrap();
    let again = mac.node.core.send(&to_pc, vec![busy_src]).await;
    let said = again.expect_err("a device takes one transfer at a time");
    assert!(said.contains("busy") && said.contains("Desk PC"), "{said}");
}

#[tokio::test]
async fn receiving_from_two_at_once_reserves_space_for_both() {
    // 300 MB free: each 16 MB transfer needs it plus a 64 MB margin.
    let mut mac = device("Studio Mac", |c| c.fault_free_space = Some(300 * MB as u64)).await;
    let mut pc = device("Desk PC", |c| c.fault_rate = Some(2 * MB as u64)).await;
    let mut air = device("MacBook Air", |c| c.fault_rate = Some(2 * MB as u64)).await;
    let mut old = device("Old Laptop", |_| {}).await;
    for d in [&mut pc, &mut air, &mut old] {
        pair_with(&mut mac.node, &mut d.node).await;
        always_from(&mac.node, &d.node).await;
    }
    let (pc_id, air_id, old_id) = (pc.node.id().await, air.node.id().await, old.node.id().await);
    let mac_id = mac.node.id().await;
    for (d, seed) in [(&pc, 3u8), (&air, 4)] {
        let src = d.node.dir.path().join(format!("out/{seed}.bin"));
        write_file(&src, 16 * MB, seed);
        d.node.core.send(&mac_id, vec![src]).await.unwrap();
    }
    mac.node
        .wait_for("both arriving", |s| {
            phase_with(s, &pc_id) == Some(TransferPhase::Running)
                && phase_with(s, &air_id) == Some(TransferPhase::Running)
        })
        .await;
    // A third doesn't fit next to the two already arriving: 300 MB free, the
    // two hold ~156 MB (what they still need plus a margin each), and it
    // needs 164 MB. Without the reservation it would fit. Refused before
    // anything is sent, saying why.
    let big = old.node.dir.path().join("out/big.bin");
    write_file(&big, 100 * MB, 5);
    old.node.core.send(&mac_id, vec![big]).await.unwrap();
    let s = old.node.wait_for("refused", |s| phase_with(s, &mac_id) == Some(TransferPhase::Failed)).await;
    let why = with(&s, &mac_id).unwrap().failure.unwrap();
    assert!(why.contains("space"), "{why}");
    let long = Duration::from_secs(60);
    mac.node
        .wait_for_within(long, "both arrived", |s| {
            phase_with(s, &pc_id) == Some(TransferPhase::Done) && phase_with(s, &air_id) == Some(TransferPhase::Done)
        })
        .await;
    assert!(phase_with(&mac.node.core.snapshot().await.unwrap(), &old_id) != Some(TransferPhase::Running));
}

#[tokio::test]
async fn one_drops_and_resumes_while_the_other_carries_on() {
    let mut mac = device("Studio Mac", |c| c.fault_rate = Some(4 * MB as u64)).await;
    let mut pc = device("Desk PC", |_| {}).await;
    let mut air = device("MacBook Air", |_| {}).await;
    pair_with(&mut pc.node, &mut mac.node).await;
    pair_with(&mut air.node, &mut mac.node).await;
    always_from(&pc.node, &mac.node).await;
    always_from(&air.node, &mac.node).await;
    let src = mac.node.dir.path().join("out/video.bin");
    write_file(&src, 20 * MB, 6);
    let (pc_id, air_id) = (pc.node.id().await, air.node.id().await);
    mac.node.core.send_many(vec![pc_id.clone(), air_id.clone()], vec![src.clone()]).await;
    pc.node.wait_for("some arrived", |s| s.transfers.iter().any(|t| t.bytes > 3 * MB as u64)).await;

    // The PC goes away mid-transfer (restarting to update, so it continues
    // by itself; a quit would ask there, `tests/stopped.rs`): its transfer
    // pauses, the Air's carries on.
    let pc_port = pc.node.port;
    pc.node.core.shutdown_for(portal_core::transfer::journal::Stopped::Update).await;
    let s = mac.node.wait_for("paused with the PC", |s| phase_with(s, &pc_id) == Some(TransferPhase::Paused)).await;
    assert_eq!(phase_with(&s, &air_id), Some(TransferPhase::Running), "the other one keeps going");
    let long = Duration::from_secs(60);
    mac.node.wait_for_within(long, "the Air's done", |s| phase_with(s, &air_id) == Some(TransferPhase::Done)).await;
    same_file(&src, &air.node.received().join("video.bin")).await;

    // The PC comes back: it continues where it was.
    let pc_dir = pc.node.dir;
    let clip = pc.clip.clone();
    let mut pc = start_in("Desk PC", pc_dir, move |c| {
        c.port = pc_port;
        c.clipboard = clip;
    })
    .await;
    mac.node.wait_for_within(long, "the PC's done", |s| phase_with(s, &pc_id) == Some(TransferPhase::Done)).await;
    pc.wait_for_within(long, "arrived", |s| s.transfers.iter().any(|t| t.phase == TransferPhase::Done)).await;
    same_file(&src, &pc.received().join("video.bin")).await;
}

#[tokio::test]
async fn two_paused_transfers_both_survive_a_restart() {
    let mut mac = device("Studio Mac", |c| c.fault_rate = Some(3 * MB as u64)).await;
    let mut pc = device("Desk PC", |_| {}).await;
    let mut air = device("MacBook Air", |_| {}).await;
    pair_with(&mut pc.node, &mut mac.node).await;
    pair_with(&mut air.node, &mut mac.node).await;
    always_from(&pc.node, &mac.node).await;
    always_from(&air.node, &mac.node).await;
    let a = mac.node.dir.path().join("out/a.bin");
    let b = mac.node.dir.path().join("out/b.bin");
    write_file(&a, 15 * MB, 7);
    write_file(&b, 15 * MB, 8);
    let (pc_id, air_id) = (pc.node.id().await, air.node.id().await);
    mac.node.core.send(&pc_id, vec![a.clone()]).await.unwrap();
    mac.node.core.send(&air_id, vec![b.clone()]).await.unwrap();
    mac.node.wait_for("both under way", |s| s.transfers.iter().filter(|t| t.bytes > 2 * MB as u64).count() == 2).await;

    // Restart mid-transfer (to update: a quit would ask first): both are
    // still there, paused, and finish.
    let port = mac.node.port;
    mac.node.core.shutdown_for(portal_core::transfer::journal::Stopped::Update).await;
    let clip = mac.clip.clone();
    let mut mac = start_in("Studio Mac", mac.node.dir, move |c| {
        c.port = port;
        c.fault_rate = Some(3 * MB as u64);
        c.clipboard = clip;
    })
    .await;
    let s = mac.core.snapshot().await.unwrap();
    let paused: Vec<&TransferView> = s.transfers.iter().filter(|t| t.direction == Direction::Out).collect();
    assert_eq!(paused.len(), 2, "both kept: {:?}", s.transfers);
    let long = Duration::from_secs(90);
    mac.wait_for_within(long, "both done", |s| {
        phase_with(s, &pc_id) == Some(TransferPhase::Done) && phase_with(s, &air_id) == Some(TransferPhase::Done)
    })
    .await;
    same_file(&a, &pc.node.received().join("a.bin")).await;
    same_file(&b, &air.node.received().join("b.bin")).await;
}

fn text(s: &str) -> Contents {
    Contents { text: Some(s.into()), ..Default::default() }
}

fn holds(clip: &MemoryClipboard, want: &str) -> bool {
    matches!(clip.current(), Read::Contents { contents, .. } if contents == text(want))
}

async fn arrives(clip: &MemoryClipboard, want: &str) {
    for _ in 0..100 {
        if holds(clip, want) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{want} never arrived: {:?}", clip.current());
}

#[tokio::test]
async fn the_clipboard_goes_to_one_or_to_several() {
    let mut mac = device("Studio Mac", |_| {}).await;
    let mut pc = device("Desk PC", |_| {}).await;
    let mut air = device("MacBook Air", |_| {}).await;
    pair_with(&mut pc.node, &mut mac.node).await;
    pair_with(&mut air.node, &mut mac.node).await;
    let (pc_id, air_id) = (pc.node.id().await, air.node.id().await);

    mac.clip.copy(text("just for the PC"));
    let said = mac.node.core.send_clipboard(Some(vec![pc_id.clone()])).await.unwrap().said;
    assert!(said.starts_with("Sent to Desk PC"), "{said}");
    arrives(&pc.clip, "just for the PC").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!holds(&air.clip, "just for the PC"), "only where it was sent");

    mac.clip.copy(text("for everyone"));
    let said = mac.node.core.send_clipboard(Some(vec![pc_id, air_id])).await.unwrap().said;
    assert!(said.starts_with("Sent to Desk PC and MacBook Air"), "{said}");
    arrives(&pc.clip, "for everyone").await;
    arrives(&air.clip, "for everyone").await;
}

#[tokio::test]
async fn automatic_sync_with_two_devices_never_loops() {
    let mut mac = device("Studio Mac", |_| {}).await;
    let mut pc = device("Desk PC", |_| {}).await;
    let mut air = device("MacBook Air", |_| {}).await;
    pair_with(&mut pc.node, &mut mac.node).await;
    pair_with(&mut air.node, &mut mac.node).await;
    pair_with(&mut air.node, &mut pc.node).await;
    let (mac_id, pc_id, air_id) = (mac.node.id().await, pc.node.id().await, air.node.id().await);
    // Every pair agrees, from both sides.
    for (d, others) in [(&mac, [&pc_id, &air_id]), (&pc, [&mac_id, &air_id]), (&air, [&mac_id, &pc_id])] {
        for o in others {
            d.node.core.set_clip_sync(o, true).await;
        }
    }
    for d in [&mut mac, &mut pc, &mut air] {
        d.node
            .wait_for("syncing with both", |s| {
                s.peers.iter().filter(|p| p.clip.as_ref().is_some_and(|c| c.mine && c.theirs)).count() == 2
            })
            .await;
    }
    mac.clip.copy(text("from the Mac"));
    arrives(&pc.clip, "from the Mac").await;
    arrives(&air.clip, "from the Mac").await;
    pc.clip.copy(text("from the PC"));
    arrives(&mac.clip, "from the PC").await;
    arrives(&air.clip, "from the PC").await;

    // Quiet afterwards: each copy arrived once on each other device, and
    // nothing bounced back to where it came from.
    tokio::time::sleep(Duration::from_millis(2500)).await;
    let shelf = |s: &Snapshot| s.clips.iter().map(|c| c.from.clone()).collect::<Vec<_>>();
    assert_eq!(shelf(&mac.node.core.snapshot().await.unwrap()), ["Desk PC"]);
    assert_eq!(shelf(&pc.node.core.snapshot().await.unwrap()), ["Studio Mac"]);
    let mut air_got = shelf(&air.node.core.snapshot().await.unwrap());
    air_got.sort();
    assert_eq!(air_got, ["Desk PC", "Studio Mac"]);
    assert!(holds(&mac.clip, "from the PC") && holds(&pc.clip, "from the PC") && holds(&air.clip, "from the PC"));
}
