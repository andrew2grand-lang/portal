//! Why a transfer stopped decides what happens next (Mission 05 §1.4):
//! quitting or a crash asks on that computer ("stopped partway: Continue or
//! Cancel") while the other waits for it; a dropped network and a restart to
//! update continue by themselves.

mod common;

use common::Transfers;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{TestNode, pair, start, start_in};
use portal_core::events::{Snapshot, TransferPhase, TransferView};
use portal_core::node::Config;
use portal_core::transfer::journal::Stopped;

const MB: usize = 1024 * 1024;

fn phase(s: &Snapshot) -> Option<TransferPhase> {
    s.transfer().map(|t| t.phase)
}

fn view(s: &Snapshot) -> TransferView {
    s.transfer().cloned().expect("a transfer")
}

fn write_file(path: &Path, len: usize, seed: u8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut data = vec![0u8; len];
    let mut x = seed as u32 | 1;
    for b in data.iter_mut() {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *b = x as u8;
    }
    std::fs::write(path, data).unwrap();
}

fn slow(c: &mut Config) {
    c.fault_rate = Some(8 * MB as u64);
}

async fn pair_up() -> (TestNode, TestNode) {
    let mut mac = start("MacBook Pro", slow).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    (mac, pc)
}

/// The Mac sends 24 MB to the PC; returns once 6 MB are there.
async fn start_sending(mac: &mut TestNode, pc: &mut TestNode) -> PathBuf {
    let src = mac.dir.path().join("out/big10g.bin");
    write_file(&src, 24 * MB, 7);
    mac.core.send(&pc.id().await, vec![src.clone()]).await.unwrap();
    pc.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    pc.accept(false).await;
    pc.wait_for("bytes moving", |s| s.transfer().is_some_and(|t| t.bytes >= 6 * MB as u64)).await;
    src
}

async fn stop(node: TestNode, why: Stopped, tweak: impl FnOnce(&mut Config)) -> TestNode {
    let (name, port) = (node.name.clone(), node.port);
    node.core.shutdown_for(why).await;
    let TestNode { dir, .. } = node;
    start_in(&name, dir, |c| {
        c.port = port;
        tweak(c);
    })
    .await
}

/// A crash: the saved state stays as it was mid-run (no clean stop).
async fn crash(node: TestNode, tweak: impl FnOnce(&mut Config)) -> TestNode {
    let dir = node.dir.path().join("transfers");
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "json"))
        .expect("saved state");
    let running = std::fs::read(&path).unwrap();
    let (name, port) = (node.name.clone(), node.port);
    node.core.shutdown().await;
    std::fs::write(&path, running).unwrap();
    let TestNode { dir, .. } = node;
    start_in(&name, dir, |c| {
        c.port = port;
        tweak(c);
    })
    .await
}

async fn both_done(a: &mut TestNode, b: &mut TestNode) {
    let long = Duration::from_secs(60);
    let finished = |s: &Snapshot| matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed));
    let sa = a.wait_for_within(long, "done", finished).await;
    let sb = b.wait_for_within(long, "done", finished).await;
    assert_eq!(phase(&sa), Some(TransferPhase::Done), "{:?}", view(&sa).failure);
    assert_eq!(phase(&sb), Some(TransferPhase::Done), "{:?}", view(&sb).failure);
}

/// The asking side and the waiting side, and nothing moving for 3 s.
async fn asks_and_waits(asking: &mut TestNode, waiting: &mut TestNode) {
    let a = asking.wait_for("asks", |s| s.transfer().is_some_and(|t| t.stopped_here)).await;
    assert_eq!(phase(&a), Some(TransferPhase::Paused));
    let w =
        waiting.wait_for("waits for the other to continue", |s| s.transfer().is_some_and(|t| t.waiting_there)).await;
    assert_eq!(phase(&w), Some(TransferPhase::Paused));
    let before = view(&w).bytes;
    tokio::time::sleep(Duration::from_secs(3)).await;
    let w = waiting.core.snapshot().await.unwrap();
    assert_eq!(phase(&w), Some(TransferPhase::Paused), "nothing continues by itself");
    assert_eq!(view(&w).bytes, before);
    let a = asking.core.snapshot().await.unwrap();
    assert!(view(&a).stopped_here && view(&a).percent_known, "the asking side knows how far it got");
}

#[tokio::test]
async fn quitting_the_sender_asks_there_and_the_receiver_waits() {
    let (mut mac, mut pc) = pair_up().await;
    let src = start_sending(&mut mac, &mut pc).await;
    let mut mac = stop(mac, Stopped::Quit, slow).await;
    asks_and_waits(&mut mac, &mut pc).await;
    let tid = mac.tid_where("held", |t| t.stopped_here).await;
    mac.core.continue_transfer(tid).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big10g.bin")).unwrap(), std::fs::read(&src).unwrap());
}

#[tokio::test]
async fn quitting_the_receiver_asks_there_and_the_sender_waits() {
    let (mut mac, mut pc) = pair_up().await;
    let src = start_sending(&mut mac, &mut pc).await;
    let mut pc = stop(pc, Stopped::Quit, |_| {}).await;
    asks_and_waits(&mut pc, &mut mac).await;
    let tid = pc.tid_where("held", |t| t.stopped_here).await;
    pc.core.continue_transfer(tid).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big10g.bin")).unwrap(), std::fs::read(&src).unwrap());
}

#[tokio::test]
async fn a_crash_asks_too() {
    let (mut mac, mut pc) = pair_up().await;
    start_sending(&mut mac, &mut pc).await;
    let mut mac = crash(mac, slow).await;
    asks_and_waits(&mut mac, &mut pc).await;
    let (mut pc, mut mac) = (pc, mac);
    let tid = mac.tid_where("held", |t| t.stopped_here).await;
    mac.core.continue_transfer(tid).await;
    both_done(&mut mac, &mut pc).await;
}

#[tokio::test]
async fn a_crashed_receiver_asks_too() {
    let (mut mac, mut pc) = pair_up().await;
    start_sending(&mut mac, &mut pc).await;
    let mut pc = crash(pc, |_| {}).await;
    asks_and_waits(&mut pc, &mut mac).await;
    let tid = pc.tid_where("held", |t| t.stopped_here).await;
    pc.core.continue_transfer(tid).await;
    both_done(&mut mac, &mut pc).await;
}

#[tokio::test]
async fn restarting_to_update_continues_by_itself() {
    let (mut mac, mut pc) = pair_up().await;
    let src = start_sending(&mut mac, &mut pc).await;
    let mut mac = stop(mac, Stopped::Update, slow).await;
    let mut pc = stop(pc, Stopped::Update, |_| {}).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big10g.bin")).unwrap(), std::fs::read(&src).unwrap());
}

#[tokio::test]
async fn a_network_drop_continues_by_itself() {
    let mut mac = start("MacBook Pro", |c| {
        slow(c);
        c.fault_cut_at = Some(8 * MB as u64);
    })
    .await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    start_sending(&mut mac, &mut pc).await;
    let seen_held = std::cell::Cell::new(false);
    let finished = |s: &Snapshot| {
        seen_held.set(seen_held.get() || s.transfer().is_some_and(|t| t.stopped_here || t.waiting_there));
        matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed))
    };
    let got = pc.wait_for_within(Duration::from_secs(60), "done", finished).await;
    assert_eq!(phase(&got), Some(TransferPhase::Done));
    assert!(!seen_held.get(), "a drop never asks");
    assert!(pc.drain_logs().iter().any(|l| l.contains("Lost the connection")), "it did drop");
}

#[tokio::test]
async fn cancel_on_the_asking_side_cleans_up_both() {
    let (mut mac, mut pc) = pair_up().await;
    start_sending(&mut mac, &mut pc).await;
    let mut pc = stop(pc, Stopped::Quit, |_| {}).await;
    asks_and_waits(&mut pc, &mut mac).await;
    pc.cancel().await;
    let ended = |s: &Snapshot| matches!(phase(s), Some(TransferPhase::Cancelled | TransferPhase::Failed));
    mac.wait_for("the sender hears", ended).await;
    pc.wait_for("ended here", ended).await;
    common::assert_nothing_partial(&pc.received()).await;
}

#[tokio::test]
async fn cancel_on_the_waiting_side_cleans_up_both() {
    let (mut mac, mut pc) = pair_up().await;
    start_sending(&mut mac, &mut pc).await;
    let mut mac = stop(mac, Stopped::Quit, slow).await;
    asks_and_waits(&mut mac, &mut pc).await;
    pc.cancel().await;
    let ended = |s: &Snapshot| matches!(phase(s), Some(TransferPhase::Cancelled | TransferPhase::Failed));
    mac.wait_for("the asking side hears", ended).await;
    pc.wait_for("ended here", ended).await;
    common::assert_nothing_partial(&pc.received()).await;
}
