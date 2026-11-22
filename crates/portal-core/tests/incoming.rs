//! "Files from <device>: Ask every time · Always accept · Never", per paired
//! device, enforced in the core (Mission 02 §7.1).

mod common;

use common::Transfers;

use std::path::Path;
use std::time::Duration;

use common::{TestNode, pair, start, start_in};
use portal_core::events::{Block, Direction, Snapshot, TransferPhase};
use portal_core::model::Incoming;

fn phase(s: &Snapshot) -> Option<TransferPhase> {
    s.transfer().map(|t| t.phase)
}

fn block(s: &Snapshot) -> Option<Block> {
    s.transfer().and_then(|t| t.block)
}

async fn paired() -> (TestNode, TestNode) {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    (mac, pc)
}

fn file(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("out/plan.pdf");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, b"floor plan").unwrap();
    p
}

/// What `node` does with files from `from`
fn incoming(s: &Snapshot, from: &str) -> Incoming {
    s.peers.iter().find(|p| p.name == from).map(|p| p.incoming).expect("peer listed")
}

#[tokio::test]
async fn ask_every_time_is_the_default() {
    let (mac, mut pc) = paired().await;
    assert_eq!(incoming(&pc.core.snapshot().await.unwrap(), "MacBook Pro"), Incoming::Ask);

    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();

    let s = pc.wait_for("offer waits for an answer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    assert!(!s.transfer().cloned().unwrap().auto_accepted);
}

#[tokio::test]
async fn always_accept_takes_files_without_asking() {
    let (mac, mut pc) = paired().await;
    let mac_id = mac.id().await;
    pc.core.set_incoming(&mac_id, Incoming::Always).await;
    pc.wait_for("saved", |s| incoming(s, "MacBook Pro") == Incoming::Always).await;

    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();

    pc.wait_for("received without asking", |s| phase(s) == Some(TransferPhase::Done)).await;
    assert!(pc.received().join("plan.pdf").exists());
}

#[tokio::test]
async fn accepting_with_always_ticked_sets_always_accept() {
    let (mac, mut pc) = paired().await;
    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();
    pc.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;

    pc.accept(true).await;

    pc.wait_for("now Always", |s| incoming(s, "MacBook Pro") == Incoming::Always).await;
}

#[tokio::test]
async fn never_refuses_and_the_sender_is_told_which_setting_to_change() {
    let (mut mac, mut pc) = paired().await;
    let mac_id = mac.id().await;
    pc.core.set_incoming(&mac_id, Incoming::Never).await;
    mac.wait_for("mac learns it", |s| s.peers.iter().any(|p| p.accepts == Some(Incoming::Never))).await;

    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();

    let s = mac.wait_for("sender blocked", |s| phase(s) == Some(TransferPhase::Blocked)).await;
    assert_eq!(block(&s), Some(Block::NotAccepted));
    let t = s.transfer().cloned().unwrap();
    assert_eq!((t.peer_name.as_str(), t.items.len()), ("Desk PC", 1), "names the computer, keeps what's waiting");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(pc.core.snapshot().await.unwrap().transfer().is_none(), "nothing reaches the PC's screen");
    assert!(std::fs::read_dir(pc.received()).map(|d| d.count() == 0).unwrap_or(true), "nothing written");
    let pc_log = pc.drain_logs();
    assert!(!pc_log.iter().any(|l| l.contains("refuse")), "the Mac knew, so it never offered: {pc_log:?}");
}

#[tokio::test]
async fn the_receiver_refuses_even_when_the_sender_did_not_know_yet() {
    let (mut mac, mut pc) = paired().await;
    let mac_id = mac.id().await;
    pc.core.set_incoming(&mac_id, Incoming::Never).await;
    pc.wait_for("saved", |s| incoming(s, "MacBook Pro") == Incoming::Never).await;

    // No waiting for the news to reach the Mac: the PC's core decides.
    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();

    let s = mac.wait_for("sender blocked", |s| phase(s) == Some(TransferPhase::Blocked)).await;
    assert_eq!(block(&s), Some(Block::NotAccepted));
    assert!(std::fs::read_dir(pc.received()).map(|d| d.count() == 0).unwrap_or(true), "nothing written");
}

#[tokio::test]
async fn allowing_files_again_sends_straight_away() {
    let (mut mac, mut pc) = paired().await;
    let mac_id = mac.id().await;
    pc.core.set_incoming(&mac_id, Incoming::Never).await;
    mac.wait_for("mac learns it", |s| s.peers.iter().any(|p| p.accepts == Some(Incoming::Never))).await;
    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();
    mac.wait_for("blocked", |s| block(s) == Some(Block::NotAccepted)).await;

    pc.core.set_incoming(&mac_id, Incoming::Ask).await;

    let offer = pc.wait_for("offer arrives by itself", |s| phase(s) == Some(TransferPhase::Offered)).await;
    assert_eq!(offer.transfer().cloned().unwrap().direction, Direction::In);
    pc.accept(false).await;
    pc.wait_for("received", |s| phase(s) == Some(TransferPhase::Done)).await;
}

#[tokio::test]
async fn never_only_stops_files_coming_in_this_computer_still_sends() {
    let (mut mac, mut pc) = paired().await;
    let pc_id = pc.id().await;
    mac.core.set_incoming(&pc_id, Incoming::Never).await;
    mac.wait_for("saved", |s| incoming(s, "Desk PC") == Incoming::Never).await;

    mac.core.send(&pc_id, vec![file(mac.dir.path())]).await.unwrap();

    pc.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    pc.accept(false).await;
    mac.wait_for("sent", |s| phase(s) == Some(TransferPhase::Done)).await;
}

#[tokio::test]
async fn dismissing_a_blocked_send_forgets_it() {
    let (mut mac, pc) = paired().await;
    let mac_id = mac.id().await;
    pc.core.set_incoming(&mac_id, Incoming::Never).await;
    mac.wait_for("mac learns it", |s| s.peers.iter().any(|p| p.accepts == Some(Incoming::Never))).await;
    mac.core.send(&pc.id().await, vec![file(mac.dir.path())]).await.unwrap();
    mac.wait_for("blocked", |s| block(s).is_some()).await;
    mac.dismiss().await;
    mac.wait_for("cleared", |s| s.transfer().is_none()).await;

    pc.core.set_incoming(&mac_id, Incoming::Ask).await;

    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(pc.core.snapshot().await.unwrap().transfer().is_none(), "no surprise send later");
}

#[tokio::test]
async fn the_setting_survives_a_restart() {
    let (mac, mut pc) = paired().await;
    let mac_id = mac.id().await;
    pc.core.set_incoming(&mac_id, Incoming::Never).await;
    pc.wait_for("saved", |s| incoming(s, "MacBook Pro") == Incoming::Never).await;

    pc.core.shutdown().await;
    let pc = start_in("Desk PC", pc.dir, |_| {}).await;

    assert_eq!(incoming(&pc.core.snapshot().await.unwrap(), "MacBook Pro"), Incoming::Never);
}

#[tokio::test]
async fn pairing_again_forgets_an_old_never() {
    let (mut mac, mut pc) = paired().await;
    let (mac_id, pc_id) = (mac.id().await, pc.id().await);
    pc.core.set_incoming(&mac_id, Incoming::Never).await;
    mac.wait_for("mac learns it", |s| s.peers.iter().any(|p| p.accepts == Some(Incoming::Never))).await;

    // The Mac removes the PC and pairs again over the connection that's still open
    mac.core.remove_peer(&pc_id).await;
    mac.wait_for("removed", |s| !s.peers.iter().any(|p| p.paired)).await;
    pc.wait_for("pc sees it", |s| s.peers.iter().any(|p| p.revoked)).await;
    let code = common::ask_and_show(&mut pc, &mut mac).await;
    mac.core.pairing(portal_core::node::PairAction::Code(code)).await.unwrap();
    mac.wait_for("paired", |s| s.peers.iter().any(|p| p.paired && p.online)).await;
    pc.wait_for("the PC asks again", |s| incoming(s, "MacBook Pro") == Incoming::Ask).await;
    mac.wait_for("the Mac hears it", |s| s.peers.iter().any(|p| p.paired && p.accepts != Some(Incoming::Never))).await;

    mac.core.send(&pc_id, vec![file(mac.dir.path())]).await.unwrap();
    pc.wait_for("offer arrives", |s| phase(s) == Some(TransferPhase::Offered)).await;
}
