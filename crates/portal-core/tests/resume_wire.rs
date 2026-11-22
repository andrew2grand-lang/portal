//! Resuming, driven message by message: a scripted sender on a raw link, so
//! the drops land exactly where a real sender's timing only sometimes puts
//! them (between `resume_from` and the next `file`, a repeated question
//! mid-run, a crash that leaves the saved state a moment behind).

mod common;

use common::Transfers;

use std::path::Path;
use std::time::Duration;

use common::{TestNode, start_in};
use portal_core::events::TransferPhase;
use portal_core::identity::Identity;
use portal_core::model::{Os, Role};
use portal_core::net::dial;
use portal_core::net::link::{self, Inbound, Link, LinkEvent};
use portal_core::proto::{self, B32, Hello, ItemKind, Msg, Offer, OfferItem};
use portal_core::store::{Store, TrustedPeer};
use portal_core::tls;
use tokio::sync::mpsc;

const MB: usize = 1024 * 1024;
const TID: u64 = 0xabc;

struct Conn {
    link: Link,
    events: mpsc::Receiver<LinkEvent>,
    sink: mpsc::Receiver<Inbound>,
}

/// A receiver that trusts (and always accepts from) the scripted sender.
async fn setup() -> (Identity, TestNode) {
    let sender_dir = tempfile::tempdir().unwrap();
    let sender = Identity::load_or_create(sender_dir.path()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    Store::open(dir.path())
        .unwrap()
        .upsert_peer(TrustedPeer {
            id: sender.device_id.clone(),
            name: "Sender".into(),
            os: Os::Macos,
            fingerprint: sender.fingerprint,
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
    (sender, start_in("Receiver", dir, |_| {}).await)
}

async fn connect(sender: &Identity, to: &TestNode) -> Conn {
    let theirs = Identity::load_or_create(to.dir.path()).unwrap();
    let tcp = dial::dial(to.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls = link::connect_tls(tcp, tls::client_config(sender, Some(theirs.fingerprint)).unwrap()).await.unwrap();
    let hello = Hello {
        proto: proto::PROTOCOL_VERSION,
        id: sender.device_id.clone(),
        name: "Sender".into(),
        os: Os::Macos,
        role: Role::Both,
        app: "test".into(),
        port: 1,
    };
    let (events_tx, events) = mpsc::channel(64);
    let link = link::handshake(tls, true, hello, events_tx).await.unwrap();
    let (sink_tx, sink) = mpsc::channel(64);
    link.set_sink(Some(sink_tx));
    Conn { link, events, sink }
}

/// The next message matching `pred`; None when the link closes or time runs out.
async fn next_msg(c: &mut Conn, secs: u64, pred: impl Fn(&Msg) -> bool) -> Option<Msg> {
    tokio::time::timeout(Duration::from_secs(secs), async {
        loop {
            tokio::select! {
                ev = c.events.recv() => match ev {
                    Some(LinkEvent::Msg(_, m)) if pred(&m) => return Some(m),
                    Some(LinkEvent::Closed(..)) | None => return None,
                    Some(_) => {}
                },
                m = c.sink.recv() => match m {
                    Some(Inbound::Msg(m)) if pred(&m) => return Some(m),
                    Some(_) => {}
                    None => return None,
                },
            }
        }
    })
    .await
    .unwrap_or(None)
}

fn data(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 31 % 251) as u8).collect()
}

fn is_resume_from(m: &Msg) -> bool {
    matches!(m, Msg::ResumeFrom { .. })
}

fn is_end(m: &Msg) -> bool {
    matches!(m, Msg::Cancel { .. } | Msg::Complete { .. })
}

async fn offer(c: &mut Conn, items: Vec<OfferItem>, total: u64, files: u64, dirs: u64) {
    c.link
        .send(Msg::Offer(Offer {
            tid: TID,
            items,
            total_bytes: total,
            file_count: files,
            dir_count: dirs,
            skipped: vec![],
            sender_role: Role::Both,
        }))
        .await
        .unwrap();
    assert!(next_msg(c, 5, |m| matches!(m, Msg::Accept { .. })).await.is_some(), "accepted");
}

async fn offer_one(c: &mut Conn, total: usize) {
    let item = OfferItem { name: "big.bin".into(), kind: ItemKind::File, bytes: total as u64, files: 1 };
    offer(c, vec![item], total as u64, 1, 0).await;
}

fn big(from: u64, total: usize) -> Msg {
    Msg::File { tid: TID, idx: 0, path: vec!["big.bin".into()], size: total as u64, mtime: None, from }
}

async fn send_bytes(c: &Conn, bytes: &[u8]) {
    for chunk in bytes.chunks(MB) {
        c.link.send_data(chunk.to_vec()).await.unwrap();
    }
}

async fn finish(c: &Conn, idx: u64, content: &[u8]) {
    let hash = B32(*blake3::hash(content).as_bytes());
    c.link.send_ordered(Msg::FileEnd { tid: TID, idx, blake3: hash }).await.unwrap();
}

async fn wait_phase(node: &mut TestNode, p: TransferPhase) {
    node.wait_for_within(Duration::from_secs(20), "phase", |s| s.transfer().is_some_and(|t| t.phase == p)).await;
}

fn files_in(dir: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, d: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.push((rel, std::fs::read(&p).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

fn offset(m: &Msg) -> u64 {
    match m {
        Msg::ResumeFrom { offset, .. } => *offset,
        _ => unreachable!(),
    }
}

/// Pauses a one-file transfer with 2 MiB of 4 arrived.
async fn paused_at_half(sender: &Identity, receiver: &mut TestNode, total: usize, content: &[u8]) {
    let mut c = connect(sender, receiver).await;
    offer_one(&mut c, total).await;
    c.link.send_ordered(big(0, total)).await.unwrap();
    send_bytes(&c, &content[..2 * MB]).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    c.link.close();
    wait_phase(receiver, TransferPhase::Paused).await;
}

#[tokio::test]
async fn a_drop_before_the_resumed_file_continues_keeps_what_arrived() {
    let (sender, mut receiver) = setup().await;
    let total = 4 * MB;
    let content = data(total);
    paused_at_half(&sender, &mut receiver, total, &content).await;

    // Back: the receiver says it has 2 MiB, and the link drops again before
    // the sender's `file` arrives.
    let mut c = connect(&sender, &receiver).await;
    let first = next_msg(&mut c, 10, is_resume_from).await.expect("resume_from");
    assert_eq!(offset(&first), 2 * MB as u64);
    c.link.close();
    wait_phase(&mut receiver, TransferPhase::Paused).await;

    let mut c = connect(&sender, &receiver).await;
    let second = next_msg(&mut c, 10, is_resume_from).await.expect("resume_from again");
    assert_eq!(offset(&second), 2 * MB as u64, "what arrived is still known");
    c.link.send_ordered(big(offset(&second), total)).await.unwrap();
    send_bytes(&c, &content[2 * MB..]).await;
    finish(&c, 0, &content).await;
    c.link.send_ordered(Msg::Done { tid: TID }).await.unwrap();
    let end = next_msg(&mut c, 10, is_end).await;
    assert!(matches!(end, Some(Msg::Complete { .. })), "{end:?}");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(files_in(&receiver.received()), vec![("big.bin".to_owned(), content)]);
}

#[tokio::test]
async fn a_repeated_question_mid_run_is_never_answered_with_an_old_place() {
    let (sender, mut receiver) = setup().await;
    let total = 6 * MB;
    let content = data(total);
    paused_at_half(&sender, &mut receiver, total, &content).await;

    let mut c = connect(&sender, &receiver).await;
    let first = next_msg(&mut c, 10, is_resume_from).await.expect("resume_from");
    assert_eq!(offset(&first), 2 * MB as u64);
    c.link.send_ordered(big(2 * MB as u64, total)).await.unwrap();
    send_bytes(&c, &content[2 * MB..4 * MB]).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // The sender (stalled, say) asks again on the same link: 2 MiB is no
    // longer where the receiver is.
    c.link.send(Msg::ResumeAsk { tid: TID }).await.unwrap();
    let mut at = match next_msg(&mut c, 10, is_resume_from).await {
        Some(again) => (offset(&again), c),
        // Out of step: the receiver starts a fresh connection instead.
        None => {
            wait_phase(&mut receiver, TransferPhase::Paused).await;
            let mut c = connect(&sender, &receiver).await;
            let fresh = next_msg(&mut c, 10, is_resume_from).await.expect("resume_from on a fresh link");
            (offset(&fresh), c)
        }
    };
    assert_eq!(at.0, 4 * MB as u64, "answered with where it really is");
    let c = &mut at.1;
    c.link.send_ordered(big(4 * MB as u64, total)).await.unwrap();
    send_bytes(c, &content[4 * MB..]).await;
    finish(c, 0, &content).await;
    c.link.send_ordered(Msg::Done { tid: TID }).await.unwrap();
    let end = next_msg(c, 10, is_end).await;
    assert!(matches!(end, Some(Msg::Complete { .. })), "{end:?}");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(files_in(&receiver.received()), vec![("big.bin".to_owned(), content)]);
}

#[tokio::test]
async fn a_finished_receiver_answers_a_late_question_with_complete() {
    let (sender, mut receiver) = setup().await;
    let content = data(MB);
    let mut c = connect(&sender, &receiver).await;
    offer_one(&mut c, MB).await;
    c.link.send_ordered(big(0, MB)).await.unwrap();
    send_bytes(&c, &content).await;
    finish(&c, 0, &content).await;
    c.link.send_ordered(Msg::Done { tid: TID }).await.unwrap();
    wait_phase(&mut receiver, TransferPhase::Done).await;
    c.link.close();

    // The `complete` went missing with the link: the sender asks.
    let mut c = connect(&sender, &receiver).await;
    c.link.send(Msg::ResumeAsk { tid: TID }).await.unwrap();
    let reply = next_msg(&mut c, 7, |m| is_resume_from(m) || is_end(m)).await;
    assert!(matches!(reply, Some(Msg::Complete { .. })), "{reply:?}");
}

#[tokio::test]
async fn a_saved_state_from_a_moment_before_a_crash_still_resumes() {
    let (sender, receiver) = setup().await;
    let size = MB / 2;
    let content = data(size);
    let mut c = connect(&sender, &receiver).await;
    let item = OfferItem { name: "Folder".into(), kind: ItemKind::Folder, bytes: 3 * size as u64, files: 3 };
    offer(&mut c, vec![item], 3 * size as u64, 3, 1).await;
    let file = |i: u64, from: u64| Msg::File {
        tid: TID,
        idx: i,
        path: vec!["Folder".into(), format!("f{i}.bin")],
        size: size as u64,
        mtime: None,
        from,
    };
    c.link.send_ordered(file(0, 0)).await.unwrap();
    c.link.send_data(content.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    finish(&c, 0, &content).await; // saved: 1 file done
    tokio::time::sleep(Duration::from_millis(300)).await;
    let journal = receiver.dir.path().join("transfers").join(format!("{TID:016x}.json"));
    let saved = std::fs::read(&journal).unwrap();

    // Within the next second: file 1 arrives and file 2 starts.
    c.link.send_ordered(file(1, 0)).await.unwrap();
    c.link.send_data(content.clone()).await.unwrap();
    finish(&c, 1, &content).await;
    c.link.send_ordered(file(2, 0)).await.unwrap();
    c.link.send_data(content[..size / 2].to_vec()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // A crash: the state on disk is the one from a moment ago, and staging
    // holds files it doesn't know about.
    c.link.close();
    receiver.core.shutdown().await;
    std::fs::write(&journal, &saved).unwrap();
    let TestNode { dir, port, name, .. } = receiver;
    let mut receiver = start_in(&name, dir, |cfg| cfg.port = port).await;

    let mut c = connect(&sender, &receiver).await;
    // A crash asks first (Mission 05 §1.4): Continue, then it resumes.
    let tid = receiver.tid_where("asks after the crash", |t| t.stopped_here).await;
    receiver.core.continue_transfer(tid).await;
    let rf = next_msg(&mut c, 10, is_resume_from).await.expect("resume_from");
    let Msg::ResumeFrom { file: f, offset, .. } = rf else { unreachable!() };
    for i in f..3 {
        let from = if i == f { offset } else { 0 };
        c.link.send_ordered(file(i, from)).await.unwrap();
        c.link.send_data(content[from as usize..].to_vec()).await.unwrap();
        finish(&c, i, &content).await;
    }
    c.link.send_ordered(Msg::Done { tid: TID }).await.unwrap();
    let end = next_msg(&mut c, 10, is_end).await;
    assert!(matches!(end, Some(Msg::Complete { .. })), "{end:?}");
    wait_phase(&mut receiver, TransferPhase::Done).await;
    let want: Vec<_> = (0..3).map(|i| (format!("Folder/f{i}.bin"), content.clone())).collect();
    assert_eq!(files_in(&receiver.received()), want);
}
