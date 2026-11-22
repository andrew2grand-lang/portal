//! The shared clipboard between two paired cores (Mission 03 §4), with an
//! in-memory clipboard on each side so tests never touch the real one.

mod common;

use common::Transfers;

use std::sync::Arc;
use std::time::Duration;

use common::{TestNode, pair, start};
use portal_core::clip::{self, Contents, MemoryClipboard, Read};
use portal_core::events::{Event, NoticeKind, TransferPhase};
use portal_core::node::SettingsPatch;

async fn pair_up() -> (TestNode, Arc<MemoryClipboard>, TestNode, Arc<MemoryClipboard>) {
    let (mac_clip, pc_clip) = (Arc::new(MemoryClipboard::default()), Arc::new(MemoryClipboard::default()));
    let (a, b) = (mac_clip.clone(), pc_clip.clone());
    let mut mac = start("MacBook Pro", move |c| c.clipboard = a).await;
    let mut pc = start("Desk PC", move |c| c.clipboard = b).await;
    pair(&mut mac, &mut pc).await;
    (mac, mac_clip, pc, pc_clip)
}

/// A paired device asks this computer to sync the clipboard.
fn asks(s: &portal_core::events::Snapshot) -> bool {
    asking(s).is_some()
}

fn asking(s: &portal_core::events::Snapshot) -> Option<String> {
    s.peers.iter().find(|p| p.clip.as_ref().is_some_and(|c| c.asks)).map(|p| p.name.clone())
}

fn text(s: &str) -> Contents {
    Contents { text: Some(s.into()), ..Default::default() }
}

/// Waits (up to 5 s) for the clipboard to hold something matching `pred`.
async fn arrives(clip: &MemoryClipboard, pred: impl Fn(&Read) -> bool) -> Read {
    for _ in 0..100 {
        let now = clip.current();
        if pred(&now) {
            return now;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never arrived; clipboard holds {:?}", clip.current());
}

fn holds(c: &Contents) -> impl Fn(&Read) -> bool + '_ {
    move |r| matches!(r, Read::Contents { contents, .. } if contents == c)
}

#[tokio::test]
async fn text_sent_from_one_computer_is_ready_to_paste_on_the_other() {
    let (mut mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac_clip.copy(text("meeting notes"));

    let said = mac.core.send_clipboard(None).await.unwrap().said;
    // Both test computers run this OS, so the paste key is this OS's.
    let paste = if cfg!(windows) { "Ctrl+V" } else { "⌘V" };
    assert_eq!(said, format!("Sent to Desk PC (text, 13 bytes). Paste it there with {paste}."));
    let got = arrives(&pc_clip, holds(&text("meeting notes"))).await;
    assert!(matches!(got, Read::Contents { ours: true, .. }), "marked as arrived, so it isn't sent back");

    // The notice, the history, and a log that never quotes it.
    let s = pc.wait_for("history", |s| !s.clips.is_empty()).await;
    assert_eq!(s.clips[0].from, "MacBook Pro");
    assert_eq!(s.clips[0].summary, "text, 13 bytes");
    let (mut notice, mut pc_logs) = (None, pc.drain_logs_keeping(|ev| matches!(ev, Event::Notice(_))));
    for ev in pc_logs.1.drain(..) {
        if let Event::Notice(n) = ev
            && n.kind == NoticeKind::Clipboard
        {
            notice = Some(n);
        }
    }
    let notice = notice.expect("a notice");
    assert_eq!(notice.title, "Clipboard from MacBook Pro");
    assert_eq!(notice.body, "“meeting notes” — ready to paste.");
    for logs in [mac.drain_logs(), std::mem::take(&mut pc_logs.0)] {
        assert!(logs.iter().any(|l| l.starts_with("clipboard") && l.contains("text, 13 bytes")), "{logs:#?}");
        assert!(!logs.iter().any(|l| l.contains("meeting")), "no contents in the log: {logs:#?}");
    }
}

#[tokio::test]
async fn rich_text_and_images_arrive_whole() {
    let (mac, mac_clip, _pc, pc_clip) = pair_up().await;
    let rich = Contents {
        text: Some("Budget".into()),
        rtf: Some(b"{\\rtf1 \\b Budget}".to_vec()),
        html: Some("<b>Budget</b>".into()),
        png: None,
    };
    mac_clip.copy(rich.clone());
    mac.core.send_clipboard(None).await.unwrap();
    arrives(&pc_clip, holds(&rich)).await;

    let png = portal_sys::clipboard::convert::png_from_rgba(64, 64, &vec![200; 64 * 64 * 4]).unwrap();
    let image = Contents { png: Some(png), ..Default::default() };
    mac_clip.copy(image.clone());
    mac.core.send_clipboard(None).await.unwrap();
    arrives(&pc_clip, holds(&image)).await;
}

#[tokio::test]
async fn an_empty_clipboard_says_what_to_do() {
    let (mac, _mac_clip, _pc, _pc_clip) = pair_up().await;
    assert_eq!(mac.core.send_clipboard(None).await.unwrap_err(), "The clipboard is empty. Copy something first.");
}

/// Mission 04 §4.2: files copied in Finder or Explorer are offered as a
/// transfer ("2 files on the clipboard · Send them to Desk PC"); nothing is
/// sent until that's pressed.
#[tokio::test]
async fn files_on_the_clipboard_are_offered_to_send() {
    let (mac, mac_clip, pc, _pc_clip) = pair_up().await;
    let (a, b) = (mac.dir.path().join("a.txt"), mac.dir.path().join("b.txt"));
    mac_clip.copy_files(vec![a.clone(), b.clone()]);
    let sent = mac.core.send_clipboard(None).await.unwrap();
    assert_eq!(sent.said, "2 files on the clipboard");
    assert_eq!(sent.files, vec![a, b]);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(pc.core.snapshot().await.unwrap().transfers.is_empty(), "nothing sent yet");
}

/// What a password manager copies never leaves the computer, and the
/// window says so in the brief's words (no "we detected"; no Send anyway).
#[tokio::test]
async fn private_content_is_never_sent() {
    let (mut mac, mac_clip, _pc, pc_clip) = pair_up().await;
    mac_clip.copy_private();
    let err = mac.core.send_clipboard(None).await.unwrap_err();
    assert_eq!(
        err,
        "The app you copied this from marked it as private, so it wasn’t sent. To send it anyway, paste it into the Shelf’s text box."
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(pc_clip.current(), Read::Empty);
    let logs = mac.drain_logs();
    assert!(
        logs.iter().any(|l| l.contains("marked it as private") && l.contains("org.nspasteboard.ConcealedType")),
        "{logs:#?}"
    );
}

/// Mission 04 §4.3: Details names what marked it, in plain words, with the
/// marker's name as the detail; a browser's private window says so.
#[tokio::test]
async fn details_says_who_marked_a_copy_private() {
    let (mut mac, mac_clip, _pc, _pc_clip) = pair_up().await;
    let cases = [
        (
            "ExcludeClipboardContentFromMonitorProcessing",
            Some("firefox.exe"),
            "Not sent: Firefox marked this copy as private, as it does in private windows (Tor Browser always does)",
        ),
        (
            "CanUploadToCloudClipboard = 0",
            Some("msedge.exe"),
            "Not sent: Edge marked this copy as private, as it does in private windows (Tor Browser always does)",
        ),
        (
            "org.nspasteboard.ConcealedType",
            Some("com.agilebits.onepassword7"),
            "Not sent: 1Password marked it as private, as password managers do",
        ),
        (
            "Clipboard Viewer Ignore",
            None,
            "Not sent: the app you copied this from marked it as private (password managers do, and browsers in private windows)",
        ),
    ];
    for (marker, source, words) in cases {
        mac_clip.copy_private_from(marker, source);
        let _ = mac.core.send_clipboard(None).await.unwrap_err();
        let lines = mac.drain_lines();
        let line = lines.iter().find(|l| l.text.starts_with("Not sent")).expect("a Details line");
        assert_eq!(line.text, words);
        assert!(line.detail.as_deref().is_some_and(|d| d.contains(marker)), "{:?}", line.detail);
    }
}

#[tokio::test]
async fn huge_text_and_big_images_go_as_a_file_instead() {
    let (mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac_clip.copy(text(&"x".repeat(clip::TEXT_LIMIT + 10)));
    let said = mac.core.send_clipboard(None).await.unwrap().said;
    assert!(said.contains("as a file"), "{said}");
    let offer = pc.wait_for("an offer", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Offered)).await;
    assert_eq!(offer.transfer().cloned().unwrap().items[0].name, "Clipboard text.txt");
    assert_eq!(pc_clip.current(), Read::Empty, "the clipboard itself is left alone");
}

#[tokio::test]
async fn automatic_sync_needs_both_computers_to_say_yes() {
    let (mut mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac_clip.copy(text("before"));
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(pc_clip.current(), Read::Empty, "off by default: nothing syncs");

    // The Mac turns it on: the PC is asked, and nothing flows until it says yes.
    mac.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    let s = pc.wait_for("asked", asks).await;
    assert_eq!(asking(&s).as_deref(), Some("MacBook Pro"));
    assert!(!s.me.clipboard_sync, "never switched on from the other computer");
    mac_clip.copy(text("not yet"));
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(pc_clip.current(), Read::Empty, "nothing flows before the PC agrees");

    // The PC says yes: copies go both ways, and what was copied before isn't sent.
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    mac.wait_for("both on", |s| s.me.clipboard_sync).await;
    let s = pc.wait_for("both on", |s| s.me.clipboard_sync).await;
    assert!(!asks(&s), "the question is answered");
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert_eq!(pc_clip.current(), Read::Empty);
    mac_clip.copy(text("first copy"));
    arrives(&pc_clip, holds(&text("first copy"))).await;
    pc_clip.copy(text("reply from the PC"));
    arrives(&mac_clip, holds(&text("reply from the PC"))).await;

    // No echo: the Mac doesn't send the PC's copy back.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let s = pc.core.snapshot().await.unwrap();
    assert_eq!(s.clips.len(), 1, "the PC received one item, nothing came back");

    // Private copies don't sync either.
    mac_clip.copy_private();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(matches!(pc_clip.current(), Read::Contents { contents, .. } if contents == text("reply from the PC")));

    // Off from either computer is off for both.
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(false), ..Default::default() }).await;
    mac.wait_for("sync off there too", |s| !s.me.clipboard_sync).await;
    mac_clip.copy(text("after"));
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(!matches!(pc_clip.current(), Read::Contents { contents, .. } if contents == text("after")));
    let logs = mac.drain_logs();
    assert!(logs.iter().any(|l| l.starts_with("clipboard") && l.contains("turned off on")), "{logs:#?}");
}

/// "Not now" on the computer that's asked turns it off on the one that asked.
#[tokio::test]
async fn saying_no_to_automatic_sync_turns_it_off_on_both() {
    let (mut mac, _mac_clip, mut pc, _pc_clip) = pair_up().await;
    mac.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    pc.wait_for("asked", asks).await;
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(false), ..Default::default() }).await;
    mac.wait_for("turned off", |s| s.peers.iter().all(|p| p.clip.as_ref().is_none_or(|c| !c.mine))).await;
    pc.wait_for("question gone", |s| !asks(s)).await;
}

/// Stands in for the paired Mac (after stopping the real one) on a raw link.
async fn fake_mac(
    mac: TestNode,
    pc: &mut TestNode,
) -> (portal_core::net::link::Link, tokio::sync::mpsc::Receiver<portal_core::net::link::LinkEvent>) {
    let identity = portal_core::identity::Identity::load_or_create(mac.dir.path()).unwrap();
    mac.core.shutdown().await;
    let tcp = portal_core::net::dial::dial(pc.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls = portal_core::net::link::connect_tls(tcp, portal_core::tls::client_config(&identity, None).unwrap())
        .await
        .unwrap();
    let hello = portal_core::proto::Hello {
        proto: portal_core::proto::PROTOCOL_VERSION,
        id: identity.device_id.clone(),
        name: "MacBook Pro".into(),
        os: portal_core::model::Os::Macos,
        role: portal_core::model::Role::Both,
        app: "test".into(),
        port: 1,
    };
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let link = portal_core::net::link::handshake(tls, true, hello, tx).await.unwrap();
    pc.wait_for("the paired Mac is back", common::paired_online).await;
    (link, rx)
}

/// Copies sent by automatic sync are refused by a computer that hasn't said
/// yes, even from its paired device (an old or misbehaving build).
#[tokio::test]
async fn automatic_copies_are_refused_without_consent() {
    let (mac, _mac_clip, mut pc, pc_clip) = pair_up().await;
    let (link, _rx) = fake_mac(mac, &mut pc).await;
    for m in clip::parts(1, &text("pushed"), true) {
        link.send(m).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(pc_clip.current(), Read::Empty);
    let logs = pc.drain_logs();
    assert!(logs.iter().any(|l| l.contains("automatic") && l.contains("off here")), "{logs:#?}");
}

/// The other computer's clock runs ten minutes ahead: a change made here
/// afterwards still counts as the newer one there.
#[tokio::test]
async fn a_change_here_is_newer_even_when_the_other_clock_runs_ahead() {
    use portal_core::net::link::LinkEvent;
    use portal_core::proto::Msg;
    let (mac, _mac_clip, mut pc, _pc_clip) = pair_up().await;
    let (link, mut rx) = fake_mac(mac, &mut pc).await;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
    let ahead = now + 10 * 60 * 1000;
    link.send(Msg::ClipSync { on: true, at: ahead }).await.unwrap();
    pc.wait_for("asked", asks).await;
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(false), ..Default::default() }).await;
    let off = tokio::time::timeout(Duration::from_secs(5), async {
        let mut said_yes = false;
        loop {
            match rx.recv().await {
                Some(LinkEvent::Msg(_, Msg::ClipSync { on: true, .. })) => said_yes = true,
                Some(LinkEvent::Msg(_, Msg::ClipSync { on: false, at })) if said_yes => return at,
                Some(_) => {}
                None => panic!("link closed"),
            }
        }
    })
    .await
    .expect("told the Mac it's off");
    assert!(off > ahead, "{off} should be after {ahead}");
}

#[tokio::test]
async fn sending_to_a_computer_that_is_away_says_so() {
    let (mac, mac_clip, pc, _pc_clip) = pair_up().await;
    pc.core.shutdown().await;
    mac_clip.copy(text("hello"));
    let mut tries = 0;
    let err = loop {
        match mac.core.send_clipboard(None).await {
            Err(e) => break e,
            Ok(_) if tries < 50 => tries += 1,
            Ok(_) => panic!("sent to a computer that's off"),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(err, "Desk PC isn’t connected right now.");
}

#[tokio::test]
async fn recently_received_keeps_the_last_five_and_can_be_cleared_or_copied_again() {
    let (mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    for i in 0..12 {
        mac_clip.copy(text(&format!("item {i}")));
        mac.core.send_clipboard(None).await.unwrap();
        arrives(&pc_clip, holds(&text(&format!("item {i}")))).await;
    }
    assert_eq!(clip::HISTORY, 5, "Mission 05 §3.1: the last 5");
    let s = pc.wait_for("five kept", |s| s.clips.len() == clip::HISTORY && s.clips[0].preview == "“item 11”").await;
    assert_eq!(s.clips.last().unwrap().preview, "“item 7”", "newest first, oldest dropped");

    pc_clip.copy(text("something else"));
    pc.core.copy_clip(s.clips[2].id).await.unwrap();
    arrives(&pc_clip, holds(&text("item 9"))).await;

    pc.core.clear_clips().await;
    pc.wait_for("cleared", |s| s.clips.is_empty()).await;
}

/// Mission 04 §1.6: Copy technical log puts the log on this computer's
/// clipboard only; automatic sync doesn't carry it to the other computer.
#[tokio::test]
async fn the_technical_log_copied_here_stays_here() {
    let (mut mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    pc.wait_for("asked", asks).await;
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    mac.wait_for("both on", |s| s.me.clipboard_sync).await;
    pc.wait_for("both on", |s| s.me.clipboard_sync).await;
    tokio::time::sleep(Duration::from_millis(1000)).await;

    mac.core.copy_text("SeamDrop technical log".into()).await.unwrap();
    assert!(holds(&text("SeamDrop technical log"))(&mac_clip.current()));
    tokio::time::sleep(Duration::from_millis(2000)).await;
    assert_eq!(pc_clip.current(), Read::Empty, "the log isn't synced");

    // Sync itself still works afterwards
    mac_clip.copy(text("a real copy"));
    arrives(&pc_clip, holds(&text("a real copy"))).await;
}

/// Mission 04 §4.1: text typed (or pasted) into the Shelf's box goes like a
/// copy: on the other computer's Shelf, and its clipboard as its setting
/// says. What's copied here stays as it is. This is also the deliberate way
/// to send something an app marked as private.
#[tokio::test]
async fn text_typed_on_the_shelf_arrives_like_a_copy() {
    let (mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac_clip.copy_private();
    let typed = "typed by hand ✓ 日本語 🙂";
    let said = mac.core.send_text(None, typed.into(), false).await.unwrap();
    assert!(said.starts_with("Sent to Desk PC"), "{said}");
    arrives(&pc_clip, holds(&text(typed))).await;
    pc.wait_for("on the shelf", |s| s.clips.first().is_some_and(|c| c.preview.contains("typed by hand"))).await;
    assert!(matches!(mac_clip.current(), Read::Private { .. }), "this computer's clipboard is untouched");
    assert_eq!(mac.core.send_text(None, "  \n ".into(), false).await.unwrap_err(), "Type something to send first.");
}

/// Review 04 (privacy): text sent as private arrives marked the way password
/// managers mark theirs, shows no preview on the Shelf, and keeps the mark
/// when copied again from there
#[tokio::test]
async fn text_sent_as_private_arrives_marked_and_without_a_preview() {
    let (mut mac, _mac_clip, mut pc, pc_clip) = pair_up().await;
    let secret = "hunter2 is the password";
    let said = mac.core.send_text(None, secret.into(), true).await.unwrap();
    assert!(said.contains("private"), "{said}");
    arrives(&pc_clip, |r| matches!(r, Read::Private { .. })).await;
    let s = pc.wait_for("on the shelf", |s| !s.clips.is_empty()).await;
    let c = s.clips[0].clone();
    assert!(c.private, "{c:?}");
    assert!(!c.preview.contains("hunter2") && c.summary.starts_with("private text"), "{c:?}");
    for logs in [mac.drain_logs(), pc.drain_logs()] {
        assert!(!logs.iter().any(|l| l.contains("hunter2")), "{logs:#?}");
    }
    pc_clip.copy(text("something else"));
    pc.core.copy_clip(c.id).await.unwrap();
    arrives(&pc_clip, |r| matches!(r, Read::Private { .. })).await;
}

/// Review 04 (privacy): one item can be taken off the Shelf, and removing a
/// device clears what it sent
#[tokio::test]
async fn one_item_can_be_removed_and_removing_a_device_clears_what_it_sent() {
    let (mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    for t in ["one", "two"] {
        mac_clip.copy(text(t));
        mac.core.send_clipboard(None).await.unwrap();
        arrives(&pc_clip, holds(&text(t))).await;
    }
    let s = pc.wait_for("two", |s| s.clips.len() == 2).await;
    pc.core.remove_clip(s.clips[0].id).await;
    let s = pc.wait_for("one left", |s| s.clips.len() == 1).await;
    assert_eq!(s.clips[0].preview, "“one”");
    pc.core.remove_peer(&mac.id().await).await;
    pc.wait_for("cleared", |s| s.clips.is_empty()).await;
    // And the other way: removed there, so what it sent here goes too
    let (mut mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac_clip.copy(text("three"));
    mac.core.send_clipboard(None).await.unwrap();
    arrives(&pc_clip, holds(&text("three"))).await;
    pc.wait_for("one", |s| s.clips.len() == 1).await;
    mac.core.remove_peer(&pc.id().await).await;
    pc.wait_for("cleared after being removed", |s| s.clips.is_empty()).await;
    let _ = mac.drain_logs();
}

/// Mission 05 §1.9: a browser's copy button (or any app) can hold the
/// clipboard while the watcher reads. That copy must still sync once it can
/// be read, and a copy that can't be read at all is said, not dropped.
#[tokio::test]
async fn a_copy_held_by_another_app_for_a_moment_still_syncs() {
    let (mut mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    pc.wait_for("asked", asks).await;
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    mac.wait_for("both on", |s| s.me.clipboard_sync).await;
    tokio::time::sleep(Duration::from_millis(800)).await;

    // The next 4 reads (about 2 s) find the clipboard busy.
    mac_clip.copy(text("VERCEL_TOKEN=abc123"));
    mac_clip.busy_for(4);
    arrives(&pc_clip, holds(&text("VERCEL_TOKEN=abc123"))).await;

    // One that stays busy past every retry: said once, not dropped silently.
    mac_clip.copy(text("never readable"));
    mac_clip.busy_for(1000);
    let said = |l: &String| l.contains("Couldn’t read that copy");
    for _ in 0..100 {
        if mac.drain_logs().iter().any(said) {
            mac_clip.busy_for(0);
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("a copy that couldn't be read was never mentioned");
}

/// Mission 05 §3.1: a copy an app marked private isn't synced, and says so
/// once (not on every tick); Put on Shelf reads it only on the click, and
/// only if the clipboard still holds that copy.
#[tokio::test]
async fn a_private_copy_is_said_once_and_can_be_put_on_the_shelf_while_it_is_still_there() {
    let (mut mac, mac_clip, mut pc, pc_clip) = pair_up().await;
    mac.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    pc.wait_for("asked", asks).await;
    pc.core.update_settings(SettingsPatch { clipboard_sync: Some(true), ..Default::default() }).await;
    mac.wait_for("both on", |s| s.me.clipboard_sync).await;
    tokio::time::sleep(Duration::from_millis(800)).await;

    mac_clip.copy_private_text("hunter2-from-1password");
    let s = mac.wait_for("said", |s| s.clip_notice.is_some()).await;
    let n = s.clip_notice.clone().unwrap();
    assert_eq!(n.kind, portal_core::events::ClipNoticeKind::Private);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(mac.core.snapshot().await.unwrap().clip_notice.unwrap().id, n.id, "said once per copy");
    assert_eq!(pc_clip.current(), Read::Empty, "not synced");

    // Still the same copy: Put on Shelf keeps it, private.
    mac.core.shelf_put_marked(n.count).await.unwrap();
    let s = mac.wait_for("on the shelf", |s| s.shelf.iter().any(|i| i.private)).await;
    let id = s.shelf.iter().find(|i| i.private).unwrap().id.clone();
    assert_eq!(mac.core.shelf_reveal(&id).await.as_deref(), Some("hunter2-from-1password"));

    // Another copy since: it's gone, and nothing is read.
    mac_clip.copy_private_text("second secret");
    let s = mac.wait_for("said again", |s| s.clip_notice.as_ref().is_some_and(|x| x.id != n.id)).await;
    let n2 = s.clip_notice.clone().unwrap();
    mac_clip.copy(text("something else entirely"));
    let err = mac.core.shelf_put_marked(n2.count).await.unwrap_err();
    assert_eq!(err, "That copy is gone; copy it again.");
}
