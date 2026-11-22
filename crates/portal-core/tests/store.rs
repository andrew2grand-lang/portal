use portal_core::model::{DeviceId, Fingerprint, Incoming, Os};
use portal_core::store::{Settings, Store, TrustedPeer};

fn peer(id: &str) -> TrustedPeer {
    TrustedPeer {
        id: DeviceId::parse(id).unwrap(),
        name: "Desk PC".into(),
        os: Os::Windows,
        fingerprint: Fingerprint::from_bytes([7; 32]),
        addrs: vec!["192.168.1.20:47920".into()],
        trusted: true,
        incoming: Incoming::Ask,
        paired_at: 1_700_000_000,
        renamed: false,
        disconnected: false,
        clip_sync: false,
        clip_sync_at: 0,
        clip_to: Default::default(),
        shelf_share: false,
        shelf_share_at: 0,
        shelf_answered: false,
        folder: None,
    }
}

const ID_A: &str = "0123456789abcdef0123456789abcdef";
const ID_B: &str = "fedcba9876543210fedcba9876543210";

#[test]
fn settings_have_the_defaults_from_the_brief() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let s = store.settings();
    assert!(s.skip_junk);
    // Mission 05 §4: new installs receive straight into Downloads
    assert_eq!(s.receive_dir, portal_core::store::downloads_dir());
    assert!(!s.device_name.trim().is_empty());
    // Mission 05 §8.2: "Get test builds" is off until switched on
    assert!(!s.test_builds);
}

#[test]
fn settings_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut store = Store::open(dir.path()).unwrap();
        store
            .update_settings(|s: &mut Settings| {
                s.device_name = "Studio Mac".into();
                s.skip_junk = false;
            })
            .unwrap();
    }
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.settings().device_name, "Studio Mac");
    assert!(!store.settings().skip_junk);
}

#[test]
fn corrupt_settings_fall_back_to_defaults() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("settings.json"), b"{not json").unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert!(store.settings().skip_junk);
}

#[test]
fn trusted_peers_can_be_added_renamed_flagged_and_removed() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut store = Store::open(dir.path()).unwrap();
        store.upsert_peer(peer(ID_A)).unwrap();
        store.upsert_peer(peer(ID_B)).unwrap();
        store.rename_peer(&DeviceId::parse(ID_A).unwrap(), "Office PC").unwrap();
        store.set_incoming(&DeviceId::parse(ID_A).unwrap(), Incoming::Never).unwrap();
        store.remove_peer(&DeviceId::parse(ID_B).unwrap()).unwrap();
    }
    let store = Store::open(dir.path()).unwrap();
    let peers = store.peers();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].name, "Office PC");
    assert_eq!(peers[0].incoming, Incoming::Never);
    assert!(store.peer(&DeviceId::parse(ID_B).unwrap()).is_none());
}

#[test]
fn upserting_a_known_peer_replaces_it_instead_of_duplicating() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.upsert_peer(peer(ID_A)).unwrap();
    let mut again = peer(ID_A);
    again.name = "Renamed".into();
    store.upsert_peer(again).unwrap();
    assert_eq!(store.peers().len(), 1);
    assert_eq!(store.peers()[0].name, "Renamed");
}

#[test]
fn device_ids_must_be_32_lowercase_hex_chars() {
    assert!(DeviceId::parse(ID_A).is_some());
    assert!(DeviceId::parse("0123").is_none());
    assert!(DeviceId::parse("0123456789ABCDEF0123456789abcdef").is_none());
    assert!(DeviceId::parse("0123456789abcdef0123456789abcdeg").is_none());
}

/// A Mission 01 peers.json / settings.json, as written on the owner's computers.
fn mission_01_files(dir: &std::path::Path, role: &str, always: [bool; 2]) {
    let peers = serde_json::json!([
        {"id": ID_A, "name": "Desk PC", "os": "windows", "fingerprint": "07".repeat(32), "addrs": [],
         "trusted": true, "always_accept": always[0], "paired_at": 1_700_000_000u64, "renamed": false, "disconnected": false},
        {"id": ID_B, "name": "Studio PC", "os": "windows", "fingerprint": "08".repeat(32), "addrs": [],
         "trusted": true, "always_accept": always[1], "paired_at": 1_700_000_000u64},
    ]);
    std::fs::write(dir.join("peers.json"), serde_json::to_vec(&peers).unwrap()).unwrap();
    let settings = serde_json::json!({"device_name": "MacBook Pro", "receive_dir": "/tmp/Portal", "role": role, "skip_junk": true});
    std::fs::write(dir.join("settings.json"), serde_json::to_vec(&settings).unwrap()).unwrap();
}

fn incoming_of(store: &Store) -> Vec<Incoming> {
    store.peers().iter().map(|p| p.incoming).collect()
}

#[test]
fn always_accept_from_mission_01_becomes_always_accept() {
    let dir = tempfile::tempdir().unwrap();
    mission_01_files(dir.path(), "both", [true, false]);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(incoming_of(&store), [Incoming::Always, Incoming::Ask]);
}

#[test]
fn a_computer_set_to_send_now_refuses_files_from_every_device() {
    let dir = tempfile::tempdir().unwrap();
    mission_01_files(dir.path(), "send", [true, false]);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(incoming_of(&store), [Incoming::Never, Incoming::Never], "Send meant it never received");
}

#[test]
fn a_computer_set_to_receive_keeps_asking() {
    let dir = tempfile::tempdir().unwrap();
    mission_01_files(dir.path(), "receive", [false, true]);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(incoming_of(&store), [Incoming::Ask, Incoming::Always], "Receive limited sending, not receiving");
}

#[test]
fn migrated_files_are_saved_once_and_the_old_role_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    mission_01_files(dir.path(), "send", [true, false]);
    drop(Store::open(dir.path()).unwrap());
    let peers = std::fs::read_to_string(dir.path().join("peers.json")).unwrap();
    let settings = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
    assert!(peers.contains("\"incoming\": \"never\""), "{peers}");
    assert!(!settings.contains("role"), "{settings}");
    // Opening again doesn't apply the old role a second time
    let mut store = Store::open(dir.path()).unwrap();
    store.set_incoming(&DeviceId::parse(ID_A).unwrap(), Incoming::Ask).unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(incoming_of(&store), [Incoming::Ask, Incoming::Never]);
}

/// Going back to a Mission 01 build mustn't lose the pairings: it needs
/// `always_accept` in every saved peer.
#[test]
fn older_builds_can_still_read_the_saved_devices() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.upsert_peer(peer(ID_A)).unwrap();
    store.upsert_peer(peer(ID_B)).unwrap();
    store.set_incoming(&DeviceId::parse(ID_A).unwrap(), Incoming::Always).unwrap();
    store.set_incoming(&DeviceId::parse(ID_B).unwrap(), Incoming::Never).unwrap();
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.path().join("peers.json")).unwrap()).unwrap();
    let always: Vec<_> = saved.as_array().unwrap().iter().map(|p| p["always_accept"].clone()).collect();
    assert_eq!(always, [serde_json::json!(true), serde_json::json!(false)]);
    // ...and that mirror never overrides the real setting when read back
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(incoming_of(&store), [Incoming::Always, Incoming::Never]);
}

#[cfg(unix)]
#[test]
fn a_migration_that_cannot_be_saved_still_opens_with_the_migrated_settings() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    mission_01_files(dir.path(), "send", [true, false]);
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let opened = Store::open(dir.path());
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    let store = opened.expect("the app still starts");
    assert_eq!(incoming_of(&store), [Incoming::Never, Incoming::Never]);
}

#[test]
fn updates_install_automatically_unless_turned_off() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("settings.json"), br#"{"device_name":"Mac","skip_junk":true}"#).unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    assert!(store.settings().auto_update, "on for settings saved before the option existed");
    store.update_settings(|s| s.auto_update = false).unwrap();
    assert!(!Store::open(dir.path()).unwrap().settings().auto_update, "kept");
}
