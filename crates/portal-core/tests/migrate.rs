//! The rename: settings, paired devices and keys come over from the old
//! app-data folder once, and the old folder is left exactly as it was.

mod common;

use std::path::Path;

use common::{pair, paired_online, start, start_in};
use portal_core::migrate;

fn files(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap()))
        .collect()
}

fn old_folder(root: &Path, receive_dir: &Path) -> std::path::PathBuf {
    let old = root.join("com.project1000.portal");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("identity.json"), br#"{"device_id":"0123456789abcdef0123456789abcdef","cert":"00"}"#)
        .unwrap();
    std::fs::write(old.join("key.bin"), b"sealed key").unwrap();
    let settings = serde_json::json!({ "device_name": "Sam's Mac", "receive_dir": receive_dir, "skip_junk": true });
    std::fs::write(old.join("settings.json"), serde_json::to_vec(&settings).unwrap()).unwrap();
    std::fs::write(old.join("peers.json"), br#"[{"id":"fedcba9876543210fedcba9876543210","trusted":true}]"#).unwrap();
    old
}

#[test]
fn settings_devices_and_keys_are_copied_and_the_old_folder_is_left_alone() {
    let root = tempfile::tempdir().unwrap();
    let old = old_folder(root.path(), &root.path().join("Elsewhere"));
    let before = files(&old);
    let new = root.path().join("com.seamdrop.desktop");

    let report = migrate::migrate(&old, &new).unwrap().expect("something to migrate");

    assert_eq!(files(&new), before, "every file copied as it was");
    assert_eq!(files(&old), before, "the old folder is untouched");
    assert_eq!(report.devices, 1);
    assert_eq!(report.receive_dir, None, "a folder the person chose is kept");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(new.join("key.bin")).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the key stays private");
    }
}

#[test]
fn it_runs_once() {
    let root = tempfile::tempdir().unwrap();
    let old = old_folder(root.path(), &root.path().join("Elsewhere"));
    let new = root.path().join("new");
    migrate::migrate(&old, &new).unwrap().unwrap();
    std::fs::write(new.join("settings.json"), b"{\"device_name\":\"Renamed since\"}").unwrap();

    assert!(migrate::migrate(&old, &new).unwrap().is_none(), "the new folder already has an identity");
    assert_eq!(std::fs::read(new.join("settings.json")).unwrap(), b"{\"device_name\":\"Renamed since\"}");
}

#[test]
fn nothing_to_migrate_is_not_an_error() {
    let root = tempfile::tempdir().unwrap();
    let new = root.path().join("new");
    assert!(migrate::migrate(&root.path().join("missing"), &new).unwrap().is_none());
    assert!(!new.exists(), "nothing created");
}

#[test]
fn the_old_default_receive_folder_becomes_the_new_one() {
    let root = tempfile::tempdir().unwrap();
    let old = old_folder(root.path(), &migrate::legacy_receive_dir());
    let new = root.path().join("new");

    let report = migrate::migrate(&old, &new).unwrap().unwrap();

    let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(new.join("settings.json")).unwrap()).unwrap();
    let now = portal_core::store::default_receive_dir();
    assert!(now.ends_with("SeamDrop"), "{}", now.display());
    assert_eq!(saved["receive_dir"], serde_json::json!(now));
    assert_eq!(saved["device_name"], "Sam's Mac", "the rest is kept");
    assert_eq!(report.receive_dir, Some((migrate::legacy_receive_dir(), now)));
    let old_saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(old.join("settings.json")).unwrap()).unwrap();
    assert_eq!(old_saved["receive_dir"], serde_json::json!(migrate::legacy_receive_dir()), "old folder untouched");
}

/// The real thing: a paired computer is renamed and still talks to its peer
/// without pairing again.
#[tokio::test]
async fn a_renamed_computer_keeps_its_pairing() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    let mac_id = mac.id().await;
    let pc_addr = pc.addr();
    let old = mac.dir.path().to_path_buf();
    mac.core.shutdown().await;

    let new_root = tempfile::tempdir().unwrap();
    let mut renamed = start_in("MacBook Pro", new_root, |cfg| {
        cfg.data_dir = cfg.data_dir.join("com.seamdrop.desktop");
        cfg.migrate_from = Some(old.clone());
    })
    .await;

    assert_eq!(renamed.id().await, mac_id, "same device");
    let logs = renamed.drain_logs();
    assert!(logs.iter().any(|l| l.starts_with("migrate") && l.contains("1 paired device")), "{logs:#?}");
    renamed.core.connect(&pc_addr).await.unwrap();
    renamed.wait_for("still paired and connected", paired_online).await;
    pc.wait_for("still paired and connected", paired_online).await;
    assert!(old.join("identity.json").exists(), "the old folder stays");
}
