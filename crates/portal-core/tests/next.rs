//! What goes next to a busy device (Mission 05 §1.5): files sent to a device
//! that's busy wait ("Next for Desk PC"), and start by themselves when its
//! transfer ends; with both computers waiting for each other, the one
//! queued earliest goes first. Cores in one process over loopback.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{TestNode, pair, start, start_in};
use portal_core::events::{Direction, Snapshot, TransferPhase};
use portal_core::model::Incoming;
use portal_core::node::Config;
use portal_core::transfer::journal::Stopped;

const MB: usize = 1024 * 1024;

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
    c.fault_rate = Some(16 * MB as u64);
}

fn file(node: &TestNode, name: &str, len: usize, seed: u8) -> PathBuf {
    let p = node.dir.path().join("out").join(name);
    write_file(&p, len, seed);
    p
}

/// Received, by name, in a receive folder.
fn got(received: &Path, name: &str) -> bool {
    received.join(name).is_file()
}

fn done_with(s: &Snapshot, name: &str) -> bool {
    s.transfers.iter().any(|t| t.phase == TransferPhase::Done && t.items.iter().any(|i| i.name == name))
}

async fn always_between(a: &TestNode, b: &TestNode) {
    a.core.set_incoming(&b.id().await, Incoming::Always).await;
    b.core.set_incoming(&a.id().await, Incoming::Always).await;
}

async fn running_to(node: &mut TestNode, peer: &portal_core::model::DeviceId) {
    node.wait_for("running", |s| {
        s.transfers.iter().any(|t| &t.peer_id == peer && t.phase == TransferPhase::Running && t.bytes > 0)
    })
    .await;
}

#[tokio::test]
async fn files_for_a_busy_device_wait_and_start_when_it_is_free() {
    let mut mac = start("MacBook Pro", slow).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    always_between(&mac, &pc).await;
    let (_mac_r, pc_r) = (mac.received(), pc.received());
    let pc_id = pc.id().await;

    let big = file(&mac, "big10g.bin", 32 * MB, 1);
    let small = file(&mac, "notes.txt", 4096, 2);
    mac.core.send(&pc_id, vec![big]).await.unwrap();
    running_to(&mut mac, &pc_id).await;

    // Busy: not refused, but next.
    let r = mac.core.send_many(vec![pc_id.clone()], vec![small]).await;
    assert!(r.iter().all(|(_, r)| r.is_ok()), "{r:?}");
    let s = mac.wait_for("next for the PC", |s| s.next.len() == 1).await;
    assert_eq!(s.next[0].peer_name, "Desk PC");
    assert_eq!(s.next[0].names, vec!["notes.txt"]);
    assert_eq!(s.transfers.iter().filter(|t| t.direction == Direction::Out).count(), 1, "nothing else started");

    let long = Duration::from_secs(60);
    mac.wait_for_within(long, "the next one went too", |s| done_with(s, "notes.txt") && s.next.is_empty()).await;
    pc.wait_for_within(long, "both arrived", |_| got(&pc_r, "notes.txt") && got(&pc_r, "big10g.bin")).await;
}

#[tokio::test]
async fn an_item_waiting_can_be_taken_off() {
    let mut mac = start("MacBook Pro", slow).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    always_between(&mac, &pc).await;
    let (_mac_r, pc_r) = (mac.received(), pc.received());
    let pc_id = pc.id().await;
    let big = file(&mac, "big10g.bin", 32 * MB, 1);
    mac.core.send(&pc_id, vec![big]).await.unwrap();
    running_to(&mut mac, &pc_id).await;
    mac.core.send_many(vec![pc_id.clone()], vec![file(&mac, "a.txt", 10, 3)]).await;
    let s = mac.wait_for("waiting", |s| s.next.len() == 1).await;
    mac.core.unqueue_next(s.next[0].id).await;
    mac.wait_for("gone", |s| s.next.is_empty()).await;
    mac.wait_for_within(Duration::from_secs(60), "big done", |s| done_with(s, "big10g.bin")).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(!got(&pc_r, "a.txt"), "taken off, so never sent");
}

#[tokio::test]
async fn both_computers_waiting_for_each_other_go_earliest_first() {
    let mut mac = start("MacBook Pro", slow).await;
    let mut pc = start("Desk PC", slow).await;
    pair(&mut mac, &mut pc).await;
    always_between(&mac, &pc).await;
    let (mac_r, pc_r) = (mac.received(), pc.received());
    let (mac_id, pc_id) = (mac.id().await, pc.id().await);

    mac.core.send(&pc_id, vec![file(&mac, "big10g.bin", 32 * MB, 1)]).await.unwrap();
    running_to(&mut mac, &pc_id).await;
    // The PC queues first, then the Mac.
    pc.core.send_many(vec![mac_id.clone()], vec![file(&pc, "from-pc.txt", 5000, 4)]).await;
    pc.wait_for("queued on the PC", |s| s.next.len() == 1).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    mac.core.send_many(vec![pc_id.clone()], vec![file(&mac, "from-mac.txt", 5000, 5)]).await;
    mac.wait_for("queued on the Mac", |s| s.next.len() == 1).await;

    // When the big one ends, the PC's (queued earlier) goes first: the Mac's
    // is still waiting when the PC's arrives.
    let long = Duration::from_secs(60);
    mac.wait_for_within(long, "the PC's arrived", |_| got(&mac_r, "from-pc.txt")).await;
    assert!(!got(&pc_r, "from-mac.txt"), "the Mac's (queued later) went first");
    let order_ok = mac.core.snapshot().await.unwrap().transfers.iter().all(|t| t.phase != TransferPhase::Failed);
    assert!(order_ok, "nothing failed as busy");
    pc.wait_for_within(long, "then the Mac's", |_| got(&pc_r, "from-mac.txt")).await;
    let logs = [mac.drain_logs(), pc.drain_logs()].concat();
    assert!(!logs.iter().any(|l| l.contains("busy")), "no refusals: {logs:#?}");
}

#[tokio::test]
async fn three_computers_each_get_what_waits_for_them() {
    let mut mac = start("MacBook Pro", slow).await;
    let mut pc = start("Desk PC", |_| {}).await;
    let mut air = start("MacBook Air", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    pair(&mut mac, &mut air).await;
    always_between(&mac, &pc).await;
    always_between(&mac, &air).await;
    let (mac_r, pc_r, air_r) = (mac.received(), pc.received(), air.received());
    let (pc_id, air_id, mac_id) = (pc.id().await, air.id().await, mac.id().await);

    // The Mac sends to both at once; both busy.
    mac.core.send_many(vec![pc_id.clone(), air_id.clone()], vec![file(&mac, "big10g.bin", 24 * MB, 1)]).await;
    running_to(&mut mac, &pc_id).await;
    running_to(&mut mac, &air_id).await;
    // More for both, and the Air has something for the Mac too.
    mac.core.send_many(vec![pc_id.clone(), air_id.clone()], vec![file(&mac, "more.txt", 3000, 6)]).await;
    air.core.send_many(vec![mac_id.clone()], vec![file(&air, "from-air.txt", 3000, 7)]).await;
    mac.wait_for("two waiting on the Mac", |s| s.next.len() == 2).await;
    air.wait_for("one waiting on the Air", |s| s.next.len() == 1).await;

    let long = Duration::from_secs(90);
    pc.wait_for_within(long, "the PC got both", |_| got(&pc_r, "big10g.bin") && got(&pc_r, "more.txt")).await;
    air.wait_for_within(long, "the Air got both", |_| got(&air_r, "big10g.bin") && got(&air_r, "more.txt")).await;
    mac.wait_for_within(long, "the Mac got the Air's", |_| got(&mac_r, "from-air.txt")).await;
    mac.wait_for("nothing left waiting", |s| s.next.is_empty()).await;
    air.wait_for("nothing left on the Air", |s| s.next.is_empty()).await;
}

#[tokio::test]
async fn what_waits_survives_a_restart_and_after_a_quit_waits_for_continue() {
    let mut mac = start("MacBook Pro", slow).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    always_between(&mac, &pc).await;
    let (_mac_r, pc_r) = (mac.received(), pc.received());
    let pc_id = pc.id().await;
    mac.core.send(&pc_id, vec![file(&mac, "big10g.bin", 32 * MB, 1)]).await.unwrap();
    running_to(&mut mac, &pc_id).await;
    mac.core.send_many(vec![pc_id.clone()], vec![file(&mac, "next.txt", 2000, 8)]).await;
    mac.wait_for("waiting", |s| s.next.len() == 1).await;

    // Quit: both the transfer and what's next wait for the person.
    let (name, port) = (mac.name.clone(), mac.port);
    mac.core.shutdown_for(Stopped::Quit).await;
    let TestNode { dir, .. } = mac;
    let mut mac = start_in(&name, dir, |c| {
        c.port = port;
        slow(c);
    })
    .await;
    let s = mac.wait_for("kept, held", |s| s.next.len() == 1 && s.next[0].held).await;
    assert_eq!(s.next[0].names, vec!["next.txt"]);
    let tid = mac.tid_where("asks", |t| t.stopped_here).await;
    mac.core.continue_transfer(tid).await;
    mac.core.continue_next(&pc_id).await;
    let long = Duration::from_secs(60);
    pc.wait_for_within(long, "both arrived", |_| got(&pc_r, "big10g.bin") && got(&pc_r, "next.txt")).await;

    // After an update restart it isn't held.
    let big2 = file(&mac, "big2.bin", 32 * MB, 9);
    mac.core.send(&pc_id, vec![big2]).await.unwrap();
    running_to(&mut mac, &pc_id).await;
    mac.core.send_many(vec![pc_id.clone()], vec![file(&mac, "after.txt", 2000, 10)]).await;
    mac.wait_for("waiting again", |s| s.next.len() == 1).await;
    let port = mac.port;
    mac.core.shutdown_for(Stopped::Update).await;
    let TestNode { dir, .. } = mac;
    let mac = start_in("MacBook Pro", dir, |c| {
        c.port = port;
        slow(c);
    })
    .await;
    let s = mac.core.snapshot().await.unwrap();
    assert!(s.next.iter().all(|n| !n.held), "an update doesn't hold it");
    pc.wait_for_within(long, "it went by itself", |_| got(&pc_r, "after.txt")).await;
}
