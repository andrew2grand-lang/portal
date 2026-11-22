//! Where received files go (Mission 04 §1.8): one folder, `Downloads/SeamDrop`,
//! or, if chosen, a folder for each device inside it.

mod common;

use common::Transfers;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{TestNode, pair, start};
use portal_core::events::TransferPhase;
use portal_core::node::SettingsPatch;

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Sends one small file from `from` to `to` (accepted there); the receiver's
/// result. The offer already says where it will be saved.
async fn send(from: &mut TestNode, to: &mut TestNode, name: &str) -> portal_core::events::ResultView {
    let file = from.dir.path().join("src").join(name);
    write(&file, name);
    let peer = to.id().await;
    from.core.send(&peer, vec![file]).await.expect("send");
    let s = to.wait_for("offer", |s| s.transfers.iter().any(|t| t.phase == TransferPhase::Offered)).await;
    let said = s.transfer().and_then(|t| t.save_to.clone());
    to.accept(false).await;
    let s = to
        .wait_for_within(Duration::from_secs(30), "done", |s| {
            s.transfer().is_some_and(|t| t.phase == TransferPhase::Done)
        })
        .await;
    let result = s.transfer().and_then(|t| t.result.clone()).expect("result");
    assert_eq!(said, result.saved_to, "the offer said where it would be saved");
    to.dismiss().await;
    from.wait_for("sent", |s| s.transfer().is_some_and(|t| t.phase == TransferPhase::Done)).await;
    from.dismiss().await;
    result
}

fn per_device(on: bool) -> SettingsPatch {
    // Mission 05 §4: a folder per device is part of "In a SeamDrop folder"
    SettingsPatch {
        folder_per_device: Some(on),
        receive_mode: Some(portal_core::store::ReceiveMode::AppFolder),
        ..Default::default()
    }
}

#[tokio::test]
async fn one_folder_by_default_and_the_first_arrival_says_so() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    assert!(!pc.core.snapshot().await.unwrap().me.folder_per_device, "off by default");

    let first = send(&mut mac, &mut pc, "a.txt").await;
    assert_eq!(first.saved_to.as_deref().map(PathBuf::from), Some(pc.received()));
    assert!(first.first_arrival, "the first files this computer received");
    assert!(pc.received().join("a.txt").is_file());

    let second = send(&mut mac, &mut pc, "b.txt").await;
    assert!(!second.first_arrival);
}

#[tokio::test]
async fn a_folder_for_each_device_uses_the_name_saved_here_and_keeps_it() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    pc.core.update_settings(per_device(true)).await;
    pc.wait_for("on", |s| s.me.folder_per_device).await;
    // This computer calls it "Sam's laptop: work/home" (not what it calls itself)
    let mac_id = mac.id().await;
    pc.core.rename_peer(&mac_id, "Sam's laptop: work/home").await;
    pc.wait_for("renamed", |s| s.peers.iter().any(|p| p.name == "Sam's laptop: work/home")).await;

    let r = send(&mut mac, &mut pc, "a.txt").await;
    let folder = pc.received().join("From Sam's laptop_ work_home");
    assert_eq!(r.saved_to.as_deref().map(PathBuf::from), Some(folder.clone()));
    assert!(folder.join("a.txt").is_file());

    // Renamed later: the same folder
    pc.core.rename_peer(&mac_id, "Studio Mac").await;
    pc.wait_for("renamed", |s| s.peers.iter().any(|p| p.name == "Studio Mac")).await;
    let r = send(&mut mac, &mut pc, "b.txt").await;
    assert_eq!(r.saved_to.as_deref().map(PathBuf::from), Some(folder.clone()));
    assert!(folder.join("b.txt").is_file());

    // Off again: back to the one folder
    pc.core.update_settings(per_device(false)).await;
    pc.wait_for("off", |s| !s.me.folder_per_device).await;
    let r = send(&mut mac, &mut pc, "c.txt").await;
    assert_eq!(r.saved_to.as_deref().map(PathBuf::from), Some(pc.received()));
}

#[tokio::test]
async fn two_devices_with_the_same_name_get_two_folders() {
    let mut pc = start("Desk PC", |_| {}).await;
    let mut a = start("MacBook Pro", |_| {}).await;
    let mut b = start("MacBook Pro", |_| {}).await;
    pair(&mut a, &mut pc).await;
    pair(&mut b, &mut pc).await;
    pc.core.update_settings(per_device(true)).await;
    pc.wait_for("on", |s| s.me.folder_per_device).await;

    let ra = send(&mut a, &mut pc, "from-a.txt").await;
    let rb = send(&mut b, &mut pc, "from-b.txt").await;
    let (fa, fb) = (pc.received().join("From MacBook Pro"), pc.received().join("From MacBook Pro (2)"));
    assert_eq!(ra.saved_to.as_deref().map(PathBuf::from), Some(fa.clone()));
    assert_eq!(rb.saved_to.as_deref().map(PathBuf::from), Some(fb.clone()));
    assert!(fa.join("from-a.txt").is_file() && fb.join("from-b.txt").is_file());

    // And each keeps its own after a restart of the receiver
    let dir = pc.dir;
    pc.core.shutdown().await;
    let mut pc = common::start_in("Desk PC", dir, |_| {}).await;
    pc.wait_for("both back", |s| s.peers.iter().filter(|p| p.online).count() == 2).await;
    let rb = send(&mut b, &mut pc, "again-b.txt").await;
    assert_eq!(rb.saved_to.as_deref().map(PathBuf::from), Some(fb));
}
