//! Transfers between two cores (brief §6.4, §9).

mod common;

use common::Transfers;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use common::{TestNode, pair, start};
use portal_core::events::{Direction, Snapshot, Tone, TransferPhase};

fn phase(s: &Snapshot) -> Option<TransferPhase> {
    s.transfer().map(|t| t.phase)
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

fn set_mtime(path: &Path, secs: i64) {
    filetime::set_file_mtime(path, filetime::FileTime::from_unix_time(secs, 0)).unwrap();
}

fn mtime(path: &Path) -> i64 {
    filetime::FileTime::from_last_modification_time(&std::fs::metadata(path).unwrap()).unix_seconds()
}

/// Relative path -> contents for every file under `root` (and empty dirs as None).
fn tree(root: &Path) -> std::collections::BTreeMap<String, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, Option<Vec<u8>>>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        entries.sort();
        if entries.is_empty() && dir != root {
            out.insert(dir.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"), None);
        }
        for p in entries {
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"),
                    Some(std::fs::read(&p).unwrap()),
                );
            }
        }
    }
    let mut out = Default::default();
    walk(root, root, &mut out);
    out
}

async fn paired_pair() -> (TestNode, TestNode) {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    (mac, pc)
}

/// Sends `paths` from `from` to `to`, accepting on `to`; waits for both to finish.
async fn send_and_accept(from: &mut TestNode, to: &mut TestNode, paths: Vec<PathBuf>) -> (Snapshot, Snapshot) {
    let peer = to.id().await;
    from.core.send(&peer, paths).await.expect("send");
    to.wait_for("offer", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Offered)).await;
    to.accept(false).await;
    let long = Duration::from_secs(300);
    let a = from
        .wait_for_within(long, "sender done", |s| matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed)))
        .await;
    let b = to
        .wait_for_within(long, "receiver done", |s| {
            matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed))
        })
        .await;
    assert_eq!(phase(&a), Some(TransferPhase::Done), "sender: {:?}", a.transfer().and_then(|t| t.failure.clone()));
    assert_eq!(phase(&b), Some(TransferPhase::Done), "receiver: {:?}", b.transfer().and_then(|t| t.failure.clone()));
    (a, b)
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect(),
        Err(_) => vec![],
    };
    v.sort();
    v
}

#[tokio::test]
async fn items_with_the_same_name_from_different_folders_all_arrive_in_one_send() {
    // Collected one by one into the send list: two "Photos" folders and two
    // "notes.txt" from different places, sent together.
    let (mut mac, mut pc) = paired_pair().await;
    let base = mac.dir.path().join("out");
    write_file(&base.join("trip/Photos/beach.jpg"), 3000, 1);
    write_file(&base.join("work/Photos/site.jpg"), 4000, 2);
    write_file(&base.join("trip/notes.txt"), 100, 3);
    write_file(&base.join("work/NOTES.txt"), 200, 4);
    let paths = vec![
        base.join("trip/Photos"),
        base.join("work/Photos"),
        base.join("trip/notes.txt"),
        base.join("work/NOTES.txt"),
    ];
    let (sent, got) = send_and_accept(&mut mac, &mut pc, paths).await;
    assert_eq!(phase(&sent), Some(TransferPhase::Done), "{:?}", sent.transfer().and_then(|t| t.failure.clone()));
    assert_eq!(phase(&got), Some(TransferPhase::Done));
    let r = pc.received();
    assert_eq!(
        std::fs::read(r.join("Photos/beach.jpg")).unwrap(),
        std::fs::read(base.join("trip/Photos/beach.jpg")).unwrap()
    );
    assert_eq!(
        std::fs::read(r.join("Photos (2)/site.jpg")).unwrap(),
        std::fs::read(base.join("work/Photos/site.jpg")).unwrap()
    );
    assert_eq!(std::fs::read(r.join("notes.txt")).unwrap(), std::fs::read(base.join("trip/notes.txt")).unwrap());
    assert_eq!(std::fs::read(r.join("NOTES (2).txt")).unwrap(), std::fs::read(base.join("work/NOTES.txt")).unwrap());
}

#[tokio::test]
async fn a_single_file_arrives_complete_with_its_time() {
    let (mut mac, mut pc) = paired_pair().await;
    // Mission 05 §4: arrivals are dated when they arrive unless the receiver
    // keeps the sender's dates; this checks the dates do travel when asked.
    pc.core.update_settings(portal_core::node::SettingsPatch { keep_dates: Some(true), ..Default::default() }).await;
    let src = mac.dir.path().join("out/report.pdf");
    write_file(&src, 20 * 1024 * 1024 + 17, 1);
    set_mtime(&src, 1_600_000_000);

    let (sent, got) = send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;

    let dest = pc.received().join("report.pdf");
    assert_eq!(std::fs::read(&dest).unwrap(), std::fs::read(&src).unwrap());
    assert_eq!(mtime(&dest), 1_600_000_000);
    let t = got.transfer().cloned().unwrap();
    assert_eq!(t.direction, Direction::In);
    assert_eq!(t.total_bytes, 20 * 1024 * 1024 + 17);
    assert_eq!(t.result.as_ref().unwrap().files, 1);
    assert_eq!(sent.transfer().cloned().unwrap().direction, Direction::Out);
    assert_eq!(files_in(&pc.received()), vec!["report.pdf"], "no staging folder left behind");
}

#[tokio::test]
async fn a_deep_folder_keeps_its_structure_contents_and_times() {
    let (mut mac, mut pc) = paired_pair().await;
    // Mission 05 §4: arrivals are dated when they arrive unless the receiver
    // keeps the sender's dates; this checks the dates do travel when asked.
    pc.core.update_settings(portal_core::node::SettingsPatch { keep_dates: Some(true), ..Default::default() }).await;
    let root = mac.dir.path().join("out/Project");
    let mut deep = root.clone();
    for level in 0..6 {
        deep = deep.join(format!("level {level}"));
        write_file(&deep.join(format!("f{level}.bin")), 1000 * (level + 1), level as u8);
    }
    std::fs::create_dir_all(root.join("empty folder")).unwrap();
    write_file(&root.join("top.txt"), 0, 0);
    set_mtime(&root.join("level 0"), 1_500_000_000);

    send_and_accept(&mut mac, &mut pc, vec![root.clone()]).await;

    let dest = pc.received().join("Project");
    assert_eq!(tree(&dest), tree(&root));
    assert_eq!(mtime(&dest.join("level 0")), 1_500_000_000, "folder times are set after their contents");
}

#[tokio::test]
async fn five_thousand_small_files_arrive_quickly() {
    let (mut mac, mut pc) = paired_pair().await;
    let root = mac.dir.path().join("out/Many");
    for i in 0..5000 {
        write_file(&root.join(format!("d{}/file-{i}.txt", i % 50)), 512 + (i % 7) * 300, (i % 251) as u8);
    }
    let started = Instant::now();
    let (_, got) = send_and_accept(&mut mac, &mut pc, vec![root.clone()]).await;
    let took = started.elapsed();
    println!("5,000 files in {took:?}");

    assert_eq!(got.transfer().cloned().unwrap().result.unwrap().files, 5000);
    assert_eq!(tree(&pc.received().join("Many")), tree(&root));
    assert!(took < Duration::from_secs(60), "pipelined: no round trip per file ({took:?})");
}

#[tokio::test]
async fn a_large_file_streams_through() {
    let mb: usize = std::env::var("PORTAL_BIG_MB").ok().and_then(|v| v.parse().ok()).unwrap_or(256);
    let (mut mac, mut pc) = paired_pair().await;
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, mb * 1024 * 1024, 9);
    let started = Instant::now();
    send_and_accept(&mut mac, &mut pc, vec![src.clone()]).await;
    let secs = started.elapsed().as_secs_f64();
    println!("{mb} MiB in {secs:.2} s = {:.0} MB/s (loopback)", (mb as f64 * 1.048_576) / secs);
    let dest = pc.received().join("big.bin");
    assert_eq!(std::fs::metadata(&dest).unwrap().len(), (mb * 1024 * 1024) as u64);
    let logs = mac.drain_logs();
    let speed = logs.iter().find(|l| l.starts_with("speed")).expect("speed over the middle 80 % is logged");
    println!("{speed}");
    assert_eq!(blake3::hash(&std::fs::read(&dest).unwrap()), blake3::hash(&std::fs::read(&src).unwrap()));
}

#[tokio::test]
async fn nothing_is_overwritten_and_existing_names_get_a_number() {
    let (mut mac, mut pc) = paired_pair().await;
    std::fs::create_dir_all(pc.received().join("Photos")).unwrap();
    std::fs::write(pc.received().join("notes.txt"), b"keep me").unwrap();
    std::fs::write(pc.received().join("Photos/old.jpg"), b"old").unwrap();
    let out = mac.dir.path().join("out");
    write_file(&out.join("notes.txt"), 100, 3);
    write_file(&out.join("Photos/new.jpg"), 100, 4);

    send_and_accept(&mut mac, &mut pc, vec![out.join("notes.txt"), out.join("Photos")]).await;

    assert_eq!(std::fs::read(pc.received().join("notes.txt")).unwrap(), b"keep me");
    assert_eq!(std::fs::read(pc.received().join("Photos/old.jpg")).unwrap(), b"old");
    assert_eq!(
        std::fs::read(pc.received().join("notes (2).txt")).unwrap(),
        std::fs::read(out.join("notes.txt")).unwrap()
    );
    assert!(pc.received().join("Photos (2)/new.jpg").exists());
}

#[tokio::test]
async fn declining_sends_nothing() {
    let (mut mac, mut pc) = paired_pair().await;
    let src = mac.dir.path().join("out/a.txt");
    write_file(&src, 10, 1);
    mac.core.send(&pc.id().await, vec![src]).await.unwrap();
    pc.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    pc.decline().await;
    mac.wait_for("declined", |s| phase(s) == Some(TransferPhase::Declined)).await;
    assert!(files_in(&pc.received()).is_empty());
}

#[tokio::test]
async fn always_accept_skips_the_prompt() {
    let (mac, mut pc) = paired_pair().await;
    pc.core.set_incoming(&mac.id().await, portal_core::model::Incoming::Always).await;
    let src = mac.dir.path().join("out/a.txt");
    write_file(&src, 10, 1);
    mac.core.send(&pc.id().await, vec![src]).await.unwrap();
    let got = pc.wait_for("done without a prompt", |s| phase(s) == Some(TransferPhase::Done)).await;
    assert!(got.transfer().cloned().unwrap().auto_accepted);
    assert_eq!(files_in(&pc.received()), vec!["a.txt"]);
}

async fn start_big_transfer(mac: &mut TestNode, pc: &mut TestNode) {
    let src = mac.dir.path().join("out/big.bin");
    write_file(&src, 300 * 1024 * 1024, 5);
    mac.core.send(&pc.id().await, vec![src]).await.unwrap();
    pc.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    pc.accept(false).await;
    pc.wait_for("some bytes", |s| s.transfer().is_some_and(|t| t.bytes > 1024 * 1024)).await;
}

/// Cleanup runs in the receiver's own task: allow it a moment (without
/// blocking the runtime it runs on), then insist.
async fn assert_clean(receive: &Path) {
    let clean = || {
        files_in(receive).iter().all(|n| n == ".seamdrop-incoming")
            && files_in(&receive.join(".seamdrop-incoming")).is_empty()
    };
    for _ in 0..50 {
        if clean() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("partial files left behind: {:?} / {:?}", files_in(receive), files_in(&receive.join(".seamdrop-incoming")));
}

#[tokio::test]
async fn cancelling_on_the_sender_cleans_up_both_sides() {
    let (mut mac, mut pc) = paired_pair().await;
    start_big_transfer(&mut mac, &mut pc).await;
    mac.cancel().await;
    let got = pc.wait_for("receiver told", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(got.transfer().cloned().unwrap().failure.unwrap().contains("cancelled"));
    assert_clean(&pc.received()).await;
    // The sender says so, and can send the same items again.
    let here = mac.wait_for("cancelled here", |s| phase(s) == Some(TransferPhase::Cancelled)).await;
    assert!(here.transfer().cloned().unwrap().failure.unwrap().contains("still on this"));
    pc.dismiss().await;
    mac.send_again().await.unwrap();
    pc.wait_for("offered again", |s| phase(s) == Some(TransferPhase::Offered)).await;
}

#[tokio::test]
async fn cancelling_on_the_receiver_cleans_up_both_sides() {
    let (mut mac, mut pc) = paired_pair().await;
    start_big_transfer(&mut mac, &mut pc).await;
    pc.cancel().await;
    let sent = mac.wait_for("sender told", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(sent.transfer().cloned().unwrap().failure.unwrap().contains("cancelled"));
    assert_clean(&pc.received()).await;
}

#[tokio::test]
async fn the_other_computer_disappearing_mid_transfer_pauses_until_cancelled() {
    let (mut mac, mut pc) = paired_pair().await;
    start_big_transfer(&mut mac, &mut pc).await;
    // The sender shows what the receiver confirmed (reported every 250 ms)
    mac.wait_for("confirmed bytes", |s| s.transfer().is_some_and(|t| t.bytes > 0)).await;
    pc.core.shutdown().await;
    let sent = mac.wait_for("sender pauses", |s| phase(s) == Some(TransferPhase::Paused)).await;
    let t = sent.transfer().cloned().unwrap();
    assert!(t.bytes > 0 && t.paused_since_ms.is_some() && t.failure.is_none());
    mac.cancel().await;
    mac.wait_for("cancelled", |s| phase(s) == Some(TransferPhase::Cancelled)).await;
}

#[tokio::test]
async fn not_enough_free_space_is_refused_before_anything_is_sent() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |c| c.fault_free_space = Some(1024)).await;
    pair(&mut mac, &mut pc).await;
    let src = mac.dir.path().join("out/a.bin");
    write_file(&src, 64 * 1024, 1);
    mac.core.send(&pc.id().await, vec![src]).await.unwrap();
    let sent = mac.wait_for("refused", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(sent.transfer().cloned().unwrap().failure.unwrap().contains("space"));
}

#[tokio::test]
async fn a_disk_filling_up_mid_transfer_stops_cleanly() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |c| c.fault_write_limit = Some(3 * 1024 * 1024)).await;
    pair(&mut mac, &mut pc).await;
    let src = mac.dir.path().join("out/a.bin");
    write_file(&src, 8 * 1024 * 1024, 1);
    mac.core.send(&pc.id().await, vec![src]).await.unwrap();
    pc.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    pc.accept(false).await;
    let got = pc.wait_for("receiver fails", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(got.transfer().cloned().unwrap().failure.unwrap().contains("space"), "says the disk is full");
    let sent = mac.wait_for("sender told", |s| phase(s) == Some(TransferPhase::Failed)).await;
    assert!(sent.transfer().cloned().unwrap().failure.unwrap().contains("space"));
    assert_clean(&pc.received()).await;
}

#[tokio::test]
async fn an_unpaired_device_cannot_send() {
    let mac = start("MacBook Pro", |_| {}).await;
    let pc = start("Desk PC", |_| {}).await;
    pc.core.connect(&mac.addr()).await.unwrap();
    let src = pc.dir.path().join("a.txt");
    write_file(&src, 10, 1);
    let err = pc.core.send(&mac.id().await, vec![src]).await.unwrap_err();
    assert!(err.contains("Pair"), "{err}");
}

/// Review 04 (privacy): Details never calls a computer that isn't paired "connected"
#[tokio::test]
async fn details_doesnt_call_an_unpaired_computer_connected() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pc.core.connect(&mac.addr()).await.unwrap();
    for n in [&mut mac, &mut pc] {
        n.wait_for("the other one", |s| s.peers.iter().any(|p| p.online)).await;
        let logs = n.drain_logs();
        let link: Vec<_> = logs.iter().filter(|l| l.starts_with("link")).collect();
        assert!(!link.is_empty(), "{}: {logs:#?}", n.name);
        for l in link {
            assert!(!l.contains("Connected to") && l.contains("not paired"), "{}: {l}", n.name);
        }
    }
}

#[tokio::test]
async fn sending_skips_symlinks_and_reports_them() {
    let (mut mac, mut pc) = paired_pair().await;
    let root = mac.dir.path().join("out/Docs");
    write_file(&root.join("real.txt"), 10, 1);
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("link.txt")).unwrap();
    #[cfg(windows)]
    if std::os::windows::fs::symlink_file(root.join("real.txt"), root.join("link.txt")).is_err() {
        return; // symlinks need Developer Mode or admin on Windows
    }
    let (sent, got) = send_and_accept(&mut mac, &mut pc, vec![root]).await;
    assert!(pc.received().join("Docs/real.txt").exists());
    assert!(!pc.received().join("Docs/link.txt").exists());
    assert_eq!(sent.transfer().cloned().unwrap().result.unwrap().skipped, 1);
    assert_eq!(got.transfer().cloned().unwrap().result.unwrap().skipped, 1);
    // Review 04 (developer, blocks use): both sides can say which item it was
    for (who, s) in [("sender", &sent), ("receiver", &got)] {
        let listed = s.transfer().cloned().unwrap().result.unwrap().skipped_items;
        assert_eq!(listed.len(), 1, "{who}: {listed:?}");
        assert_eq!(listed[0].path, "Docs/link.txt", "{who}");
        assert!(listed[0].why.contains("link"), "{who}: {:?}", listed[0].why);
    }
    let logs = pc.drain_logs();
    assert!(logs.iter().any(|l| l.starts_with("skip") && l.contains("Docs/link.txt")), "receiver's Details: {logs:#?}");
    let _ = SystemTime::now();
}

/// Brief §4.9 / §6.5: a Mac sends awkward names to a PC (Windows rules on the receiver).
#[cfg(unix)]
#[tokio::test]
async fn nasty_names_arrive_safe_for_windows_and_every_rename_is_reported() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |c| c.name_rules = Some(portal_core::fsafe::Target::Windows)).await;
    pair(&mut mac, &mut pc).await;
    let root = mac.dir.path().join("out/Nasty");
    // As macOS stores it: decomposed (NFD) accents.
    let names =
        ["Re\u{301}sume\u{301}: Q3?.pdf", "CON.txt", "trailing dot.", "a<b>c.md", "a:b.txt", "a?b.txt", "x|y\"z*.csv"];
    for (i, n) in names.iter().enumerate() {
        write_file(&root.join(n), 10 + i, i as u8);
    }
    write_file(&root.join(format!("{}.txt", "長".repeat(120))), 5, 1);

    let (sent, got) = send_and_accept(&mut mac, &mut pc, vec![root]).await;

    let dest = pc.received().join("Nasty");
    let mut arrived: Vec<String> =
        std::fs::read_dir(&dest).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    arrived.sort();
    let mut expected: Vec<String> =
        ["Résumé_ Q3_.pdf", "CON_.txt", "trailing dot", "a_b_c.md", "a_b.txt", "a_b (2).txt", "x_y_z_.csv"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    expected.push(format!("{}.txt", "長".repeat(120)));
    expected.sort();
    assert_eq!(arrived, expected);
    let r = got.transfer().cloned().unwrap().result.unwrap();
    assert_eq!(r.renamed, 7);
    assert!(r.notes.iter().any(|n| n.contains("7 names changed so they work on Windows")), "{:?}", r.notes);
    assert_eq!(sent.transfer().cloned().unwrap().result.unwrap().renamed, 7, "the sender hears about it too");
    // Review 04 (developer, blocks use): and both sides can say which names changed
    for (who, s) in [("sender", &sent), ("receiver", &got)] {
        let listed = s.transfer().cloned().unwrap().result.unwrap().renamed_items;
        assert_eq!(listed.len(), 7, "{who}: {listed:?}");
        // The path as sent (macOS keeps accents decomposed)
        let resume =
            listed.iter().find(|i| i.path.ends_with(": Q3?.pdf")).unwrap_or_else(|| panic!("{who}: {listed:#?}"));
        assert!(resume.why.contains("Résumé_ Q3_.pdf") && resume.why.contains("Windows"), "{who}: {:?}", resume.why);
    }
    let mac_logs = mac.drain_logs();
    assert!(
        mac_logs.iter().any(|l| l.starts_with("rename") && l.contains("Résumé_ Q3_.pdf")),
        "the sender's Details lists the renames too: {mac_logs:#?}"
    );
    let logs = pc.drain_logs();
    assert!(
        logs.iter().any(|l| l.starts_with("rename") && l.contains("Résumé_ Q3_.pdf") && l.contains("‘:’ and ‘?’")),
        "{logs:?}"
    );
}

/// Mission 04 §1.6: Details reads like a calm story. A transfer that goes
/// well says what happened in plain words, with nothing amber or red.
#[tokio::test]
async fn a_transfer_that_goes_well_tells_a_calm_story() {
    let (mut mac, mut pc) = paired_pair().await;
    let file = mac.dir.path().join("src/report.pdf");
    write_file(&file, 300_000, 9);
    mac.drain_lines();
    pc.drain_lines();
    send_and_accept(&mut mac, &mut pc, vec![file]).await;

    for (who, lines) in [("sender", mac.drain_lines()), ("receiver", pc.drain_lines())] {
        let alarmed: Vec<_> = lines.iter().filter(|l| matches!(l.tone, Some(Tone::Warn | Tone::Err))).collect();
        assert!(alarmed.is_empty(), "{who}: nothing went wrong, yet {alarmed:#?}");
        for l in &lines {
            // Plain words first: a sentence, not a tag soup
            assert!(l.text.chars().next().is_some_and(char::is_uppercase), "{who}: {:?}", l.text);
            assert!(!l.text.contains("  "), "{who}: {:?}", l.text);
        }
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.starts_with("Every file checked")), "{who}: {texts:#?}");
    }
}

/// Red only when something failed and needs you: a transfer cut short by
/// the other side says so once, in red, in the words its screen uses.
#[tokio::test]
async fn a_failed_transfer_says_so_once_in_red() {
    let (mac, mut pc) = paired_pair().await;
    let file = mac.dir.path().join("src/big.bin");
    write_file(&file, 40 * 1024 * 1024, 3);
    let peer = pc.id().await;
    mac.core.send(&peer, vec![file]).await.expect("send");
    pc.wait_for("offer", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Offered)).await;
    pc.accept(false).await;
    pc.wait_for("running", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Running)).await;
    // The receiver's disk is "full": its folder disappears under it
    std::fs::remove_dir_all(pc.received()).unwrap();
    std::fs::write(pc.received(), b"not a folder").unwrap();
    let s = pc
        .wait_for_within(Duration::from_secs(30), "stopped", |s| {
            matches!(phase(s), Some(TransferPhase::Failed | TransferPhase::Done))
        })
        .await;
    if phase(&s) == Some(TransferPhase::Done) {
        eprintln!("finished before the folder went: nothing to check");
        return;
    }
    let red: Vec<_> = pc.drain_lines().into_iter().filter(|l| l.tone == Some(Tone::Err)).collect();
    assert_eq!(red.len(), 1, "{red:#?}");
    assert_eq!(Some(red[0].text.clone()), s.transfer().and_then(|t| t.failure.clone()));
}

/// Mission 05 §1.6: Open only for a document, image or media file; a
/// received script or app is only ever shown in its folder.
#[tokio::test]
async fn open_is_offered_only_for_safe_single_files() {
    let (mac, mut pc) = paired_pair().await;
    pc.core.set_incoming(&mac.id().await, portal_core::model::Incoming::Always).await;
    for (name, safe) in [("plan.pdf", true), ("run.command", false), ("Tool.app.zip", false)] {
        let src = mac.dir.path().join("out").join(name);
        write_file(&src, 64, 3);
        mac.core.send(&pc.id().await, vec![src]).await.unwrap();
        let s = pc
            .wait_for("done", |s| {
                s.transfers.iter().any(|t| t.phase == TransferPhase::Done && t.items.iter().any(|i| i.name == name))
            })
            .await;
        let t = s.transfers.iter().find(|t| t.items.iter().any(|i| i.name == name)).unwrap();
        let open = t.result.as_ref().unwrap().open.clone();
        assert_eq!(open.is_some(), safe, "{name}: {open:?}");
        if let Some(p) = open {
            assert!(p.ends_with(name));
        }
        pc.dismiss().await;
        mac.dismiss().await;
    }
}
