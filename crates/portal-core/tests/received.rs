//! Where received files go, and their dates (Mission 05 §4). Two cores in
//! one process over loopback, and the settings store on its own.

mod common;

use std::path::Path;
use std::time::{Duration, SystemTime};

use common::{TestNode, pair, start};
use portal_core::events::{Snapshot, TransferPhase};
use portal_core::model::Incoming;
use portal_core::node::SettingsPatch;
use portal_core::store::{ReceiveMode, Store, default_receive_dir, downloads_dir};

fn done(s: &Snapshot, name: &str) -> bool {
    s.transfers.iter().any(|t| t.phase == TransferPhase::Done && t.items.iter().any(|i| i.name == name))
}

fn age(p: &Path) -> Duration {
    SystemTime::now().duration_since(std::fs::metadata(p).unwrap().modified().unwrap()).unwrap_or_default()
}

const OLD: i64 = 1_577_836_800; // 1 January 2020

/// `name` (a folder with a file inside, both dated 2020) from the Mac to the PC.
async fn send_old_folder(mac: &TestNode, pc: &mut TestNode, name: &str) {
    let src = mac.dir.path().join("out").join(name);
    std::fs::create_dir_all(&src).unwrap();
    let f = src.join("plan.pdf");
    std::fs::write(&f, b"plan").unwrap();
    filetime::set_file_mtime(&f, filetime::FileTime::from_unix_time(OLD, 0)).unwrap();
    filetime::set_file_mtime(&src, filetime::FileTime::from_unix_time(OLD, 0)).unwrap();
    mac.core.send(&pc.id().await, vec![src]).await.unwrap();
    pc.wait_for("arrived", |s| done(s, name)).await;
}

async fn pair_always() -> (TestNode, TestNode) {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    pc.core.set_incoming(&mac.id().await, Incoming::Always).await;
    (mac, pc)
}

#[tokio::test]
async fn files_and_folders_are_dated_when_they_arrive_unless_kept() {
    let (mac, mut pc) = pair_always().await;
    send_old_folder(&mac, &mut pc, "Site photos").await;
    let got = pc.received().join("Site photos");
    assert!(age(&got.join("plan.pdf")) < Duration::from_secs(120), "the file is new in the folder");
    assert!(age(&got) < Duration::from_secs(120), "the folder is new in the folder");

    pc.core.update_settings(SettingsPatch { keep_dates: Some(true), ..Default::default() }).await;
    pc.dismiss().await;
    send_old_folder(&mac, &mut pc, "Old photos").await;
    let got = pc.received().join("Old photos");
    let year = Duration::from_secs(365 * 24 * 3600);
    assert!(age(&got.join("plan.pdf")) > year, "Keep original dates: the sender's date");
    assert!(age(&got) > year, "and the folder's");
}

#[tokio::test]
async fn a_new_numbered_folder_for_each_delivery_keeps_sent_folders_whole() {
    let (mac, mut pc) = pair_always().await;
    pc.core.update_settings(SettingsPatch { receive_mode: Some(ReceiveMode::Numbered), ..Default::default() }).await;
    send_old_folder(&mac, &mut pc, "Site photos").await;
    pc.dismiss().await;
    send_old_folder(&mac, &mut pc, "Invoices").await;
    let first = pc.received().join("SeamDrop 001 – from MacBook Pro");
    let second = pc.received().join("SeamDrop 002 – from MacBook Pro");
    assert!(first.join("Site photos/plan.pdf").is_file(), "the sent folder keeps its own name inside");
    assert!(second.join("Invoices/plan.pdf").is_file());
}

#[tokio::test]
async fn in_a_seamdrop_folder_with_one_per_device() {
    let (mac, mut pc) = pair_always().await;
    pc.core
        .update_settings(SettingsPatch {
            receive_mode: Some(ReceiveMode::AppFolder),
            folder_per_device: Some(true),
            ..Default::default()
        })
        .await;
    send_old_folder(&mac, &mut pc, "Site photos").await;
    assert!(pc.received().join("From MacBook Pro/Site photos/plan.pdf").is_file());
}

fn settings_json(dir: &Path, json: serde_json::Value) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("settings.json"), serde_json::to_vec(&json).unwrap()).unwrap();
}

#[test]
fn new_installs_go_straight_into_downloads() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.settings().receive_mode, Some(ReceiveMode::Loose));
    assert_eq!(store.settings().receive_dir, downloads_dir());
    assert!(!store.settings().keep_dates);
    assert_eq!(store.settings().receive_note, None);
}

#[test]
fn the_update_moves_received_files_over_as_the_spec_says() {
    // The old default (Downloads/SeamDrop): straight into Downloads, said once.
    let dir = tempfile::tempdir().unwrap();
    settings_json(dir.path(), serde_json::json!({ "device_name": "Desk PC", "receive_dir": default_receive_dir() }));
    let s = Store::open(dir.path()).unwrap().settings().clone();
    assert_eq!((s.receive_mode, s.receive_dir.clone()), (Some(ReceiveMode::Loose), downloads_dir()));
    assert!(s.receive_note.is_some(), "said once");
    // …and only once: the next start leaves it.
    let s2 = Store::open(dir.path()).unwrap().settings().clone();
    assert_eq!(s2.receive_mode, Some(ReceiveMode::Loose));

    // A chosen folder (the owner's points at Downloads): the same folder, loose, nothing to say.
    let dir = tempfile::tempdir().unwrap();
    let chosen = downloads_dir().join("From the Mac");
    settings_json(dir.path(), serde_json::json!({ "device_name": "Desk PC", "receive_dir": chosen }));
    let s = Store::open(dir.path()).unwrap().settings().clone();
    assert_eq!((s.receive_mode, s.receive_dir.clone()), (Some(ReceiveMode::Loose), chosen));
    assert_eq!(s.receive_note, None);

    // "A folder for each device" was on: the SeamDrop folder, one per device.
    let dir = tempfile::tempdir().unwrap();
    settings_json(
        dir.path(),
        serde_json::json!({ "device_name": "Desk PC", "receive_dir": default_receive_dir(), "folder_per_device": true }),
    );
    let s = Store::open(dir.path()).unwrap().settings().clone();
    assert_eq!(
        (s.receive_mode, s.receive_dir.clone(), s.folder_per_device),
        (Some(ReceiveMode::AppFolder), default_receive_dir(), true)
    );
}
