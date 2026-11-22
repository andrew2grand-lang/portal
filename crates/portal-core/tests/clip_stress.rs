//! Mission 04 §4.5: the shared clipboard under stress, with an in-memory
//! clipboard on each computer (what can't be automated is in
//! docs/MANUAL-TESTS.md: rich text from Word or Pages, a real browser's
//! images, an app that closed, a computer asleep).

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{TestNode, Transfers, pair, start};
use portal_core::clip::{Contents, MemoryClipboard, Read, TEXT_LIMIT};
use portal_core::events::TransferPhase;
use portal_core::node::SettingsPatch;

struct Device {
    node: TestNode,
    clip: Arc<MemoryClipboard>,
}

async fn device(name: &str) -> Device {
    let clip = Arc::new(MemoryClipboard::default());
    let c = clip.clone();
    Device { node: start(name, move |cfg| cfg.clipboard = c).await, clip }
}

fn text(s: &str) -> Contents {
    Contents { text: Some(s.into()), ..Default::default() }
}

fn holding(clip: &MemoryClipboard) -> Option<String> {
    match clip.current() {
        Read::Contents { contents, .. } => contents.text,
        _ => None,
    }
}

/// Waits (up to `secs`) for the clipboard to hold `want`.
async fn arrives(clip: &MemoryClipboard, want: &str, secs: u64) {
    for _ in 0..secs * 20 {
        if holding(clip).as_deref() == Some(want) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never arrived: {want:.40}; holds {:.40?}", holding(clip));
}

/// Turns automatic sync on everywhere (each agrees with every paired device).
async fn sync_all(devices: &mut [&mut Device]) {
    for d in devices.iter() {
        d.node.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    }
    let n = devices.len() - 1;
    for d in devices.iter_mut() {
        d.node
            .wait_for("everyone agreed", |s| {
                s.peers.iter().filter(|p| p.clip.as_ref().is_some_and(|c| c.mine && c.theirs)).count() == n
            })
            .await;
    }
    tokio::time::sleep(Duration::from_millis(600)).await;
}

#[tokio::test]
async fn copying_fast_and_again_ends_with_the_last_copy_and_no_echo() {
    let (mut mac, mut pc) = (device("MacBook Pro").await, device("Desk PC").await);
    pair(&mut mac.node, &mut pc.node).await;
    sync_all(&mut [&mut mac, &mut pc]).await;
    for i in 0..40 {
        mac.clip.copy(text(&format!("copy {i}")));
        tokio::time::sleep(Duration::from_millis(15)).await;
    }
    arrives(&pc.clip, "copy 39", 5).await;
    // Nothing comes back, and nothing keeps moving
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(holding(&mac.clip).as_deref(), Some("copy 39"));
    assert_eq!(holding(&pc.clip).as_deref(), Some("copy 39"));
    let shelf = pc.node.core.snapshot().await.unwrap().clips;
    assert!(shelf.len() <= 10, "the Shelf keeps ten at most");
    assert_eq!(mac.node.core.snapshot().await.unwrap().clips.len(), 0, "no echo back to the Mac");
}

#[tokio::test]
async fn very_long_text_and_every_script_arrive_exactly() {
    let (mut mac, mut pc) = (device("MacBook Pro").await, device("Desk PC").await);
    pair(&mut mac.node, &mut pc.node).await;
    let scripts =
        "Ünïcødé 日本語 中文 한국어 العربية עברית हिन्दी ไทย Ελληνικά кириллица 🙂👍🏽👨‍👩‍👧 \u{200d} \r\n tabs\tand\0nul";
    mac.clip.copy(text(scripts));
    mac.node.core.send_clipboard(None).await.unwrap();
    arrives(&pc.clip, scripts, 5).await;

    // Just under the clipboard limit, in many parts
    let long: String = scripts
        .chars()
        .cycle()
        .take_while({
            let mut bytes = 0;
            move |c| {
                bytes += c.len_utf8();
                bytes < TEXT_LIMIT - 16
            }
        })
        .collect();
    mac.clip.copy(text(&long));
    let said = mac.node.core.send_clipboard(None).await.unwrap().said;
    assert!(said.starts_with("Sent to Desk PC"), "{said}");
    arrives(&pc.clip, &long, 20).await;
}

#[tokio::test]
async fn send_clipboard_works_while_a_transfer_runs() {
    let (mut mac, mut pc) = (device("MacBook Pro").await, device("Desk PC").await);
    pair(&mut mac.node, &mut pc.node).await;
    let file = mac.node.dir.path().join("big.bin");
    std::fs::write(&file, vec![7u8; 64 * 1024 * 1024]).unwrap();
    let pc_id = pc.node.id().await;
    mac.node.core.send(&pc_id, vec![file]).await.unwrap();
    pc.node.wait_for("offer", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Offered)).await;
    pc.node.accept(false).await;
    pc.node.wait_for("running", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Running)).await;

    mac.clip.copy(text("while the file moves"));
    mac.node.core.send_clipboard(None).await.unwrap();
    arrives(&pc.clip, "while the file moves", 10).await;
    let s = pc
        .node
        .wait_for_within(Duration::from_secs(120), "file done", |s| {
            s.transfer().is_some_and(|t| matches!(t.phase, TransferPhase::Done | TransferPhase::Failed))
        })
        .await;
    assert_eq!(s.transfer().unwrap().phase, TransferPhase::Done, "{:?}", s.transfer().unwrap().failure);
}

/// Three computers syncing with each other: a copy on one reaches both
/// others once, and nobody passes it on (no loops).
#[tokio::test]
async fn three_computers_sync_without_loops() {
    let (mut a, mut b, mut c) = (device("Mac").await, device("PC").await, device("Laptop").await);
    pair(&mut a.node, &mut b.node).await;
    pair(&mut a.node, &mut c.node).await;
    pair(&mut b.node, &mut c.node).await;
    sync_all(&mut [&mut a, &mut b, &mut c]).await;

    a.clip.copy(text("from the Mac"));
    arrives(&b.clip, "from the Mac", 5).await;
    arrives(&c.clip, "from the Mac", 5).await;
    tokio::time::sleep(Duration::from_millis(2000)).await;
    for (who, d) in [("PC", &b), ("Laptop", &c)] {
        let shelf = d.node.core.snapshot().await.unwrap().clips;
        assert_eq!(shelf.len(), 1, "{who} got it exactly once: {shelf:?}");
        assert_eq!(shelf[0].from, "Mac");
    }
    assert!(a.node.core.snapshot().await.unwrap().clips.is_empty(), "nothing came back to the Mac");

    // And the other way, from the Laptop
    c.clip.copy(text("from the Laptop"));
    arrives(&a.clip, "from the Laptop", 5).await;
    arrives(&b.clip, "from the Laptop", 5).await;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(b.node.core.snapshot().await.unwrap().clips.len(), 2);
}
