//! Resuming interrupted transfers (Mission 03 §2): a drop, a restart or a
//! sleep pauses a transfer; it continues by itself from where it stopped when
//! the two computers see each other again, every file still checked whole.

mod common;

use common::Transfers;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{TestNode, pair, start, start_in};
use portal_core::events::{Snapshot, TransferPhase};
use portal_core::node::Config;

const MB: usize = 1024 * 1024;

fn phase(s: &Snapshot) -> Option<TransferPhase> {
    s.transfer().map(|t| t.phase)
}

fn failure(s: &Snapshot) -> String {
    s.transfer().and_then(|t| t.failure.clone()).unwrap_or_default()
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

/// Relative path -> contents for every file under `root`.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"),
                    std::fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|r| r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// After a transfer that finished: nothing left in the staging folder.
async fn assert_no_staging(receive: &Path) {
    let staging = receive.join(".seamdrop-incoming");
    for _ in 0..100 {
        if names_in(&staging).is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("partial files left behind: {:?}", names_in(&staging));
}

/// Nothing partial in the receive folder (cleanup may lag a moment).
async fn assert_clean(receive: &Path) {
    let staging = receive.join(".seamdrop-incoming");
    for _ in 0..100 {
        if names_in(receive).iter().all(|n| n == ".seamdrop-incoming") && names_in(&staging).is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("partial files left behind: {:?} / {:?}", names_in(receive), names_in(&staging));
}

async fn pair_up(mac: impl FnOnce(&mut Config), pc: impl FnOnce(&mut Config)) -> (TestNode, TestNode) {
    let mut mac = start("MacBook Pro", mac).await;
    let mut pc = start("Desk PC", pc).await;
    pair(&mut mac, &mut pc).await;
    (mac, pc)
}

/// Shuts a node down and starts it again on the same data folder and port
/// (like restarting the app to update, which continues by itself; a quit
/// asks first, `tests/stopped.rs`).
async fn restart(node: TestNode, tweak: impl FnOnce(&mut Config)) -> TestNode {
    let (name, port) = (node.name.clone(), node.port);
    node.core.shutdown_for(portal_core::transfer::journal::Stopped::Update).await;
    let TestNode { dir, .. } = node;
    start_in(&name, dir, |c| {
        c.port = port;
        tweak(c);
    })
    .await
}

async fn send_and_accept(from: &mut TestNode, to: &mut TestNode, paths: Vec<PathBuf>) {
    from.core.send(&to.id().await, paths).await.expect("send");
    to.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    to.accept(false).await;
}

async fn both_done(a: &mut TestNode, b: &mut TestNode) {
    let long = Duration::from_secs(60);
    let finished = |s: &Snapshot| matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed));
    let sa = a.wait_for_within(long, "sender done", finished).await;
    let sb = b.wait_for_within(long, "receiver done", finished).await;
    if phase(&sa) != Some(TransferPhase::Done) || phase(&sb) != Some(TransferPhase::Done) {
        panic!(
            "sender: {:?} {}\nreceiver: {:?} {}\nsender log: {:#?}\nreceiver log: {:#?}",
            phase(&sa),
            failure(&sa),
            phase(&sb),
            failure(&sb),
            a.drain_logs(),
            b.drain_logs()
        );
    }
}

async fn wait_bytes(node: &mut TestNode, at_least: u64) {
    node.wait_for("bytes moving", |s| s.transfer().is_some_and(|t| t.bytes >= at_least)).await;
}

fn paused(s: &Snapshot) -> bool {
    phase(s) == Some(TransferPhase::Paused)
}

/// The connection drops at every awkward point; each time both sides pause,
/// continue by themselves, and every byte arrives intact.
#[tokio::test]
async fn a_dropped_connection_pauses_and_resumes_at_every_point() {
    // A folder of three files: 3 MiB, 5 MiB, 2 MiB.
    let sizes = [3 * MB, 5 * MB, 2 * MB];
    let total: usize = sizes.iter().sum();
    let cuts = [
        ("at the start", 1),
        ("in the middle of a file", 4 * MB as u64 + 12_345),
        ("between two files", (3 * MB) as u64),
        ("on the last chunk", total as u64 - 1000),
    ];
    for (what, cut) in cuts {
        let (mut mac, mut pc) = pair_up(|c| c.fault_cut_at = Some(cut), |_| {}).await;
        let src = mac.dir.path().join("out/Photos");
        for (i, len) in sizes.iter().enumerate() {
            write_file(&src.join(format!("{i}.bin")), *len, i as u8 + 1);
        }
        send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
        both_done(&mut mac, &mut pc).await;

        assert_eq!(tree(&pc.received().join("Photos")), tree(&src), "{what}: same bytes");
        let (mac_logs, pc_logs) = (mac.drain_logs(), pc.drain_logs());
        for logs in [&mac_logs, &pc_logs] {
            assert!(logs.iter().any(|l| l.starts_with("pause")), "{what}: paused\n{logs:#?}");
            assert!(logs.iter().any(|l| l.starts_with("resume")), "{what}: resumed\n{logs:#?}");
        }
        assert_eq!(pc_logs.iter().filter(|l| l.starts_with("offer")).count(), 1, "{what}: no second prompt");
        assert_no_staging(&pc.received()).await;
    }
}

/// Finished files aren't sent again: only what was missing moves after a pause.
#[tokio::test]
async fn finished_files_are_not_sent_again() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/Folder");
    for i in 0..4 {
        write_file(&src.join(format!("{i}.bin")), 2 * MB, i + 10);
    }
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    // Three files in (and some of the fourth), then the receiver quits.
    wait_bytes(&mut pc, (6 * MB + MB / 2) as u64).await;
    let mut pc = restart(pc, |_| {}).await;
    both_done(&mut mac, &mut pc).await;

    let logs = pc.drain_logs();
    let resumed = logs.iter().find(|l| l.contains("continue from file")).expect("resumed");
    assert!(resumed.ends_with("continue from file 4 of 4)"), "{resumed}");
    assert_eq!(tree(&pc.received().join("Folder")), tree(&src));

    // Review 04 (developer): the finished speed counts only what crossed the
    // wire this time, and says how much was already there
    let mut results = Vec::new();
    for (who, n) in [("sender", &mac), ("receiver", &pc)] {
        let s = n.core.snapshot().await.expect("core running");
        results.push((who, s.transfer().and_then(|t| t.result.clone()).expect("result")));
    }
    for (who, r) in results {
        assert!(
            r.kept_bytes >= (6 * MB) as u64 && r.kept_bytes < r.bytes,
            "{who}: kept {} of {}",
            r.kept_bytes,
            r.bytes
        );
    }
    let sent = mac.drain_logs().into_iter().find(|l| l.starts_with("sent")).expect("sent line");
    assert!(sent.contains("already there"), "{sent}");
}

#[tokio::test]
async fn the_receiver_restarting_mid_transfer_resumes() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 3);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;

    let mut pc = restart(pc, |_| {}).await;
    pc.wait_for("paused after the restart", |s| paused(s) || phase(s) == Some(TransferPhase::Running)).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big.bin")).unwrap(), std::fs::read(&src).unwrap());
    assert_no_staging(&pc.received()).await;

    // Both done screens say it continued, and the sender's says where it went.
    let sent = mac.core.snapshot().await.unwrap().transfer().cloned().unwrap().result.unwrap();
    let got = pc.core.snapshot().await.unwrap().transfer().cloned().unwrap().result.unwrap();
    assert!(sent.notes[0].starts_with("Continued after the connection dropped"), "{:?}", sent.notes);
    assert!(got.notes[0].starts_with("Continued after the connection dropped"), "{:?}", got.notes);
    assert_eq!(sent.saved_to.as_deref(), Some(pc.received().display().to_string().as_str()));
}

#[tokio::test]
async fn the_sender_restarting_mid_transfer_resumes() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 4);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;

    let mac_dir_src = src.clone();
    let mut mac = restart(mac, |c| c.fault_rate = Some(8 * MB as u64)).await;
    pc.wait_for("receiver pauses", paused).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big.bin")).unwrap(), std::fs::read(&mac_dir_src).unwrap());
}

#[tokio::test]
async fn both_restarting_resumes() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 5);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;

    let mut pc = restart(pc, |_| {}).await;
    let mut mac = restart(mac, |_| {}).await;
    let s = mac.wait_for("paused with its progress", paused).await;
    // The sender shows what the receiver confirmed (reported every 250 ms,
    // so at 8 MB/s up to 2 MB behind what had arrived), never bytes merely queued.
    let shown = s.transfer().cloned().unwrap().bytes;
    assert!(shown >= 4 * MB as u64, "shows what's done so far: {shown}");
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big.bin")).unwrap(), std::fs::read(&src).unwrap());
}

/// The sender's file changed while paused: that file starts again, and the
/// sender says why.
#[tokio::test]
async fn a_source_file_changed_while_paused_is_sent_again_from_the_start() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/report.bin");
    write_file(&src, 24 * MB, 6);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;
    let pc_port = pc.port;
    pc.core.shutdown_for(portal_core::transfer::journal::Stopped::Update).await;
    mac.wait_for("paused", paused).await;

    write_file(&src, 20 * MB, 7); // a new version, different size and time
    let TestNode { dir, name, .. } = pc;
    let mut pc = start_in(&name, dir, |c| c.port = pc_port).await;
    both_done(&mut mac, &mut pc).await;

    assert_eq!(
        std::fs::read(pc.received().join("report.bin")).unwrap(),
        std::fs::read(&src).unwrap(),
        "the new version"
    );
    let logs = mac.drain_logs();
    assert!(logs.iter().any(|l| l.contains("report.bin") && l.contains("changed")), "{logs:#?}");
    let notes = mac.core.snapshot().await.unwrap().transfer().cloned().unwrap().result.unwrap().notes;
    assert!(notes.iter().any(|n| n.contains("report.bin") && n.contains("changed")), "{notes:?}");
}

/// Re-reading what already arrived can take longer than the stall limit (a
/// big file on a slow disk): the two sides keep each other posted meanwhile,
/// so the transfer doesn't stall, pause and start the same check again.
#[tokio::test]
async fn a_long_check_of_what_already_arrived_does_not_stall_the_transfer() {
    let stall = Duration::from_secs(2);
    let (mut mac, mut pc) = pair_up(
        |c| {
            c.stall = stall;
            c.fault_cut_at = Some(12 * MB as u64);
            c.fault_hash_rate = Some(2 * MB as u64); // 6 s to check 12 MiB
        },
        |c| c.stall = stall,
    )
    .await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 16 * MB, 21);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    both_done(&mut mac, &mut pc).await;

    assert_eq!(std::fs::read(pc.received().join("big.bin")).unwrap(), std::fs::read(&src).unwrap());
    let logs = pc.drain_logs();
    assert_eq!(logs.iter().filter(|l| l.starts_with("pause")).count(), 1, "paused once, for the cut\n{logs:#?}");
}

/// The source changed while paused but kept its size and time (some apps
/// and copies keep the time): the part that arrived no longer matches, so
/// that file starts again instead of failing at the final check.
#[tokio::test]
async fn a_source_changed_in_place_while_paused_starts_that_file_again() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/notes.bin");
    write_file(&src, 24 * MB, 22);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;
    let (name, port) = (pc.name.clone(), pc.port);
    pc.core.shutdown_for(portal_core::transfer::journal::Stopped::Update).await;
    mac.wait_for("paused", paused).await;

    let mtime = filetime::FileTime::from_last_modification_time(&std::fs::metadata(&src).unwrap());
    let mut bytes = std::fs::read(&src).unwrap();
    bytes[1000] ^= 0xff;
    std::fs::write(&src, &bytes).unwrap();
    filetime::set_file_mtime(&src, mtime).unwrap();

    let TestNode { dir, .. } = pc;
    let mut pc = start_in(&name, dir, |c| c.port = port).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("notes.bin")).unwrap(), bytes, "the new version");
    let notes = mac.core.snapshot().await.unwrap().transfer().cloned().unwrap().result.unwrap().notes;
    assert!(notes.iter().any(|n| n.contains("notes.bin") && n.contains("changed")), "{notes:?}");
}

/// The only saved state of a transfer, from a stopped core's data folder.
fn journal_of(node: &TestNode) -> (PathBuf, serde_json::Value) {
    let dir = node.dir.path().join("transfers");
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "json"))
        .expect("saved state");
    let v = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    (path, v)
}

/// A transfer that paused once and then ran on for longer than the pause
/// limit: a crash now must not count the old pause and remove everything.
#[tokio::test]
async fn a_crash_long_after_a_resume_does_not_count_the_old_pause() {
    let limit = Duration::from_secs(3);
    let (mut mac, mut pc) = pair_up(
        |c| {
            c.fault_cut_at = Some(4 * MB as u64);
            c.fault_rate = Some(4 * MB as u64);
            c.resume_for = limit;
        },
        |c| c.resume_for = limit,
    )
    .await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 48 * MB, 23);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    pc.wait_for("paused at the cut", paused).await;
    pc.wait_for("running again", |s| phase(s) == Some(TransferPhase::Running)).await;
    tokio::time::sleep(limit + Duration::from_secs(2)).await;

    for node in [&mac, &pc] {
        let (_, saved) = journal_of(node);
        assert!(saved["paused_at"].is_null(), "{}: running, not paused: {saved:#}", node.name);
    }
    // The receiver crashes: its state on disk is the last one saved mid-run.
    let (path, saved) = journal_of(&pc);
    let (name, port) = (pc.name.clone(), pc.port);
    pc.core.shutdown().await;
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let TestNode { dir, .. } = pc;
    let mut pc = start_in(&name, dir, |c| {
        c.port = port;
        c.resume_for = limit;
    })
    .await;
    // A crash asks before continuing (Mission 05 §1.4).
    let tid = pc.tid_where("asks after the crash", |t| t.stopped_here).await;
    pc.core.continue_transfer(tid).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big.bin")).unwrap(), std::fs::read(&src).unwrap());
}

/// The partial file changed on the receiver's disk while paused: it isn't
/// trusted, so that file starts again; the result is still exact.
#[tokio::test]
async fn a_partial_file_changed_on_disk_while_paused_starts_again() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 8);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;
    let (name, port) = (pc.name.clone(), pc.port);
    pc.core.shutdown_for(portal_core::transfer::journal::Stopped::Update).await;

    let staging = pc.received().join(".seamdrop-incoming");
    let tid = names_in(&staging).pop().expect("a partial transfer is kept");
    let partial = staging.join(tid).join("big.bin");
    let mut bytes = std::fs::read(&partial).unwrap();
    bytes[100] ^= 0xff;
    std::fs::write(&partial, bytes).unwrap();

    let TestNode { dir, .. } = pc;
    let mut pc = start_in(&name, dir, |c| c.port = port).await;
    both_done(&mut mac, &mut pc).await;
    assert_eq!(std::fs::read(pc.received().join("big.bin")).unwrap(), std::fs::read(&src).unwrap());
    assert!(pc.drain_logs().iter().any(|l| l.contains("big.bin") && l.contains("changed on disk")));
}

#[tokio::test]
async fn a_disk_filling_up_while_resuming_stops_cleanly() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 9);
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;

    let mut pc = restart(pc, |c| c.fault_write_limit = Some(2 * MB as u64)).await;
    let got = pc.wait_for("receiver fails", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(failure(&got).contains("space"), "{}", failure(&got));
    let sent = mac.wait_for("sender told", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(failure(&sent).contains("space"), "{}", failure(&sent));
    assert_clean(&pc.received()).await;
}

/// Cancelling on the sender while the receiver is away: the receiver learns
/// when it's back and removes what had arrived.
#[tokio::test]
async fn cancelling_a_paused_transfer_on_the_sender_cleans_up_both_sides() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 10);
    send_and_accept(&mut mac, &mut pc, vec![src]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;
    let (name, port) = (pc.name.clone(), pc.port);
    pc.core.shutdown().await;
    mac.wait_for("paused", paused).await;
    mac.cancel().await;
    let s = mac.wait_for("cancelled here", |s| phase(s) == Some(TransferPhase::Cancelled)).await;
    assert!(failure(&s).contains("still on this"), "says the files are safe: {}", failure(&s));

    let TestNode { dir, .. } = pc;
    let mut pc = start_in(&name, dir, |c| c.port = port).await;
    let got = pc.wait_for("told it was cancelled", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(failure(&got).contains("MacBook Pro cancelled"), "{}", failure(&got));
    assert_clean(&pc.received()).await;
}

#[tokio::test]
async fn cancelling_a_paused_transfer_on_the_receiver_cleans_up_both_sides() {
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), |_| {}).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 11);
    send_and_accept(&mut mac, &mut pc, vec![src]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;
    let (name, port) = (mac.name.clone(), mac.port);
    mac.core.shutdown().await;
    pc.wait_for("paused", paused).await;
    pc.cancel().await;
    let s = pc.wait_for("cancelled here", |s| phase(s) == Some(TransferPhase::Cancelled)).await;
    assert!(failure(&s).contains("removed"), "says what happened to what arrived: {}", failure(&s));
    assert_clean(&pc.received()).await;

    let TestNode { dir, .. } = mac;
    let mut mac = start_in(&name, dir, |c| c.port = port).await;
    let sent = mac.wait_for("told it was cancelled", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(failure(&sent).contains("Desk PC cancelled"), "{}", failure(&sent));
}

/// Paused for longer than the limit: the partial data goes, and both sides
/// say so.
#[tokio::test]
async fn a_paused_transfer_expires() {
    let short = |c: &mut Config| c.resume_for = Duration::from_secs(2);
    let (mut mac, mut pc) = pair_up(|c| c.fault_rate = Some(8 * MB as u64), short).await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 24 * MB, 12);
    send_and_accept(&mut mac, &mut pc, vec![src]).await;
    wait_bytes(&mut pc, 6 * MB as u64).await;
    let (name, port) = (mac.name.clone(), mac.port);
    mac.core.shutdown().await;

    let got = pc.wait_for("expired", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(failure(&got).starts_with("Paused for more than 2 seconds"), "{}", failure(&got));
    assert_clean(&pc.received()).await;

    let TestNode { dir, .. } = mac;
    let mut mac = start_in(&name, dir, |c| {
        c.port = port;
        c.resume_for = Duration::from_secs(2);
    })
    .await;
    let sent = mac.wait_for("sender told", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(failure(&sent).contains("Send it again"), "{}", failure(&sent));
}

/// Builds before the rename kept partial files in `.portal-incoming`; they
/// can't be resumed, so they're removed.
#[tokio::test]
async fn partial_files_from_before_the_rename_are_removed() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("Received/.portal-incoming/00000000000000ab");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("half.bin"), b"half").unwrap();
    let mut node = start_in("Desk PC", dir, |_| {}).await;
    node.wait_for("started", |_| true).await;
    assert!(!node.received().join(".portal-incoming").exists());
}
