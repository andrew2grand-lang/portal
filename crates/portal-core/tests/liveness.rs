//! Liveness (Mission 05 §1.2): a computer that really disappears is shown as
//! Paused on the other within about 3 s, and a slow link or a long re-check
//! after a resume never is. Two cores on one machine with a relay between
//! them that can freeze traffic without closing anything (a dead Wi-Fi link,
//! as TCP sees it), or hold it to a slow rate.
//!
//! The spec asks for 20 out of 20 of each. CI runs each once per OS (the
//! cut test does `ROUNDS` drops in one run); `LIVENESS_ROUNDS=20` and a
//! shell loop do the 20 here (PROGRESS has the counts).
mod common;

use std::cell::Cell;
use std::path::Path;
use std::time::{Duration, Instant};

use common::relay::{Relay, Shape};
use common::{TestNode, Transfers, free_tcp_port, pair_via, set_known_addrs, start, start_in};
use portal_core::Config;
use portal_core::events::{Snapshot, TransferPhase};

fn phase(s: &Snapshot) -> Option<TransferPhase> {
    s.transfer().map(|t| t.phase)
}

fn paused(s: &Snapshot) -> bool {
    phase(s) == Some(TransferPhase::Paused)
}

fn running(s: &Snapshot) -> bool {
    phase(s) == Some(TransferPhase::Running)
}

fn finished(s: &Snapshot) -> bool {
    matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed))
}

fn write_file(path: &Path, len: usize, seed: u8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut x = seed as u32 | 1;
    let data: Vec<u8> = (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect();
    std::fs::write(path, data).unwrap();
}

/// The PC never listens, so every link is the PC dialing the Mac through
/// the relay (the Mac knows the PC's own address from pairing and would
/// otherwise reconnect around the relay).
fn pc_only_dials(c: &mut Config) {
    c.port = free_tcp_port();
    c.fault_listen_after = Some(Duration::from_secs(3600));
}

/// Starts the PC, pairs it with the Mac through the relay, then leaves it
/// knowing only the relay's address for the Mac (pairing also stores the
/// Mac's own address, which on one machine is reachable around the relay).
async fn pc_through(mac: &mut TestNode, relay: &Relay, tweak: impl Fn(&mut Config)) -> TestNode {
    let mut pc = start("Desk PC", &tweak).await;
    pair_via(mac, &mut pc, &relay.addr.to_string()).await;
    pc.core.shutdown().await;
    set_known_addrs(pc.dir.path(), &[relay.addr.to_string()]);
    let mut pc = start_in("Desk PC", pc.dir, &tweak).await;
    pc.wait_for("back through the relay", common::paired_online).await;
    pc
}

fn rounds() -> usize {
    std::env::var("LIVENESS_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(5)
}

/// The most either side may take to show Paused after the other vanished.
const PAUSED_WITHIN: Duration = Duration::from_secs(3);

#[tokio::test]
async fn a_computer_that_vanishes_is_paused_on_the_other_within_3_seconds() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    // 2 MB/s: slow enough that the transfer outlasts every round.
    let relay = Relay::start(mac.addr().parse().unwrap(), Shape { delay: Duration::ZERO, rate: 2_000_000 }).await;
    let pc = pc_through(&mut mac, &relay, pc_only_dials).await;

    let file = pc.dir.path().join("src/big10g.bin");
    let len = 96 * 1024 * 1024;
    write_file(&file, len, 5);
    pc.core.send(&mac.id().await, vec![file.clone()]).await.unwrap();
    mac.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    mac.accept(false).await;

    let mut worst = (Duration::ZERO, Duration::ZERO);
    for round in 1..=rounds() {
        // Moving, and a little further than last time.
        let before = mac.wait_for_within(Duration::from_secs(20), "running", running).await.transfer().unwrap().bytes;
        mac.wait_for_within(Duration::from_secs(20), "moving", |s| {
            running(s) && s.transfer().is_some_and(|t| t.bytes > before + 256 * 1024)
        })
        .await;

        relay.drop_out();
        let cut = Instant::now();
        // Neither process closed anything: each must notice the silence.
        let (mut r, mut s) = (None, None);
        while r.is_none() || s.is_none() {
            if r.is_none() && paused(&mac.core.snapshot().await.unwrap()) {
                r = Some(cut.elapsed());
            }
            if s.is_none() && paused(&pc.core.snapshot().await.unwrap()) {
                s = Some(cut.elapsed());
            }
            assert!(cut.elapsed() < Duration::from_secs(40), "round {round}: still not paused after 40 s");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let (r, s) = (r.unwrap(), s.unwrap());
        eprintln!("round {round}: receiver paused after {r:?}, sender after {s:?}");
        worst = (worst.0.max(r), worst.1.max(s));
        assert!(r <= PAUSED_WITHIN, "round {round}: the receiver took {r:?} to show Paused");
        assert!(s <= PAUSED_WITHIN, "round {round}: the sender took {s:?} to show Paused");

        relay.come_back();
    }
    eprintln!("worst of {} rounds: receiver {:?}, sender {:?}", rounds(), worst.0, worst.1);
}

#[tokio::test]
async fn a_link_held_to_300_kb_per_s_never_pauses() {
    // A 1 MiB frame takes about 3.5 s to cross at this rate: longer than the
    // silence that counts as gone, so only "any bytes count" keeps it alive.
    let mut mac = start("MacBook Pro", |_| {}).await;
    let relay =
        Relay::start(mac.addr().parse().unwrap(), Shape { delay: Duration::from_millis(15), rate: 300_000 }).await;
    let mut pc = pc_through(&mut mac, &relay, pc_only_dials).await;

    let file = pc.dir.path().join("src/slow.bin");
    let len = 6 * 1024 * 1024;
    write_file(&file, len, 11);
    pc.core.send(&mac.id().await, vec![file.clone()]).await.unwrap();
    mac.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    mac.accept(false).await;

    let (mac_paused, pc_paused) = (Cell::new(false), Cell::new(false));
    let m = mac
        .wait_for_within(Duration::from_secs(60), "receiver done", |s| {
            mac_paused.set(mac_paused.get() || paused(s));
            finished(s)
        })
        .await;
    let p = pc
        .wait_for_within(Duration::from_secs(10), "sender done", |s| {
            pc_paused.set(pc_paused.get() || paused(s));
            finished(s)
        })
        .await;
    assert_eq!(phase(&m), Some(TransferPhase::Done), "{:?}", m.transfer().and_then(|t| t.failure.clone()));
    assert_eq!(phase(&p), Some(TransferPhase::Done));
    let logs = [mac.drain_logs(), pc.drain_logs()].concat();
    let pauses: Vec<_> = logs.iter().filter(|l| l.contains("Lost the connection")).collect();
    assert!(!mac_paused.get() && !pc_paused.get() && pauses.is_empty(), "paused on a slow link: {pauses:?}");
    assert_eq!(std::fs::read(mac.received().join("slow.bin")).unwrap(), std::fs::read(&file).unwrap());
}

#[tokio::test]
async fn a_long_re_check_after_a_resume_never_pauses() {
    // The sender re-reads what already arrived before it continues: held
    // to 16 MB/s, 160 MB of it takes about 10 s with nothing on the wire
    // but liveness. (Same quiet as a multi-GB file at disk speed.)
    let mut mac = start("MacBook Pro", |_| {}).await;
    // 40 MB/s: the cut lands mid-transfer (unthrottled loopback finished the
    // whole file before the test could cut it, and there was nothing to pause).
    let relay = Relay::start(mac.addr().parse().unwrap(), Shape { delay: Duration::ZERO, rate: 40_000_000 }).await;
    let mut pc = pc_through(&mut mac, &relay, |c| {
        pc_only_dials(c);
        c.fault_hash_rate = Some(16_000_000);
    })
    .await;

    let file = pc.dir.path().join("src/long.bin");
    let len = 200 * 1024 * 1024;
    write_file(&file, len, 13);
    pc.core.send(&mac.id().await, vec![file.clone()]).await.unwrap();
    mac.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    mac.accept(false).await;
    mac.wait_for_within(Duration::from_secs(30), "most of it here", |s| {
        s.transfer().is_some_and(|t| t.bytes >= 160 * 1024 * 1024)
    })
    .await;

    relay.drop_out();
    let cut = std::time::Instant::now();
    while !paused(&mac.core.snapshot().await.unwrap()) || !paused(&pc.core.snapshot().await.unwrap()) {
        if cut.elapsed() > Duration::from_secs(40) {
            panic!("not paused 40 s after the cut\nreceiver: {:#?}\nsender: {:#?}", mac.drain_logs(), pc.drain_logs());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    mac.drain_logs();
    pc.drain_logs();
    relay.come_back();

    let (mac_paused, pc_paused) = (Cell::new(false), Cell::new(false));
    mac.wait_for_within(Duration::from_secs(20), "resumed", |s| !paused(s)).await;
    let m = mac
        .wait_for_within(Duration::from_secs(90), "receiver done", |s| {
            mac_paused.set(mac_paused.get() || paused(s));
            finished(s)
        })
        .await;
    let p = pc
        .wait_for_within(Duration::from_secs(10), "sender done", |s| {
            pc_paused.set(pc_paused.get() || paused(s));
            finished(s)
        })
        .await;
    assert_eq!(phase(&m), Some(TransferPhase::Done), "{:?}", m.transfer().and_then(|t| t.failure.clone()));
    assert_eq!(phase(&p), Some(TransferPhase::Done));
    let logs = [mac.drain_logs(), pc.drain_logs()].concat();
    let pauses: Vec<_> = logs.iter().filter(|l| l.contains("Lost the connection")).collect();
    assert!(!mac_paused.get() && !pc_paused.get() && pauses.is_empty(), "paused during the re-check: {pauses:?}");
}

/// This computer stalls (its runtime blocked for 4 s, as a test writing a big
/// file does, or a machine under heavy load): when it wakes, the bytes the
/// other side sent meanwhile are waiting, and the link must not be taken
/// for dead before the reader has had its turn.
#[tokio::test]
async fn a_computer_that_stalls_for_a_moment_does_not_drop_a_healthy_link() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    common::pair(&mut mac, &mut pc).await;
    let s = mac.core.snapshot().await.unwrap();
    let link_before = s.peers.iter().find(|p| p.paired).and_then(|p| p.addr.clone());
    for _ in 0..3 {
        // Both cores run on this test's one thread: block it.
        std::thread::sleep(Duration::from_secs(4));
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    let logs = [mac.drain_logs(), pc.drain_logs()].concat();
    let dropped: Vec<_> = logs.iter().filter(|l| l.contains("nothing heard for")).collect();
    assert!(dropped.is_empty(), "a stall here was taken for the other side vanishing: {dropped:?}");
    let s = mac.core.snapshot().await.unwrap();
    assert_eq!(s.peers.iter().find(|p| p.paired).and_then(|p| p.addr.clone()), link_before);
}
