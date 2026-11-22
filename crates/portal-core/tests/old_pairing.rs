//! Mission 04 §2: existing pairings keep working after the update. Their
//! keys are pinned; nobody pairs again. `fixtures/paired-0.3.1` holds two
//! data folders paired by SeamDrop 0.3.1 (protocol 2, numeric comparison).

mod common;

use std::path::Path;

use common::{free_tcp_port, paired_online, set_known_addrs, start_in};

/// A copy of one fixture folder, its key sealed the way this OS stores it.
fn copy_fixture(name: &str) -> tempfile::TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/paired-0.3.1").join(name);
    let dir = tempfile::tempdir().unwrap();
    for f in ["identity.json", "settings.json", "peers.json"] {
        std::fs::copy(src.join(f), dir.path().join(f)).unwrap();
    }
    let der = std::fs::read(src.join("key.der")).unwrap();
    std::fs::write(dir.path().join("key.bin"), portal_sys::secret::protect(&der).unwrap()).unwrap();
    dir
}

fn fixture_id(name: &str) -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/paired-0.3.1").join(name).join("identity.json");
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v["device_id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn two_computers_paired_by_0_3_1_connect_without_pairing_again() {
    let (mac_dir, pc_dir) = (copy_fixture("a"), copy_fixture("b"));
    let (mac_port, pc_port) = (free_tcp_port(), free_tcp_port());
    set_known_addrs(mac_dir.path(), &[format!("127.0.0.1:{pc_port}")]);
    set_known_addrs(pc_dir.path(), &[format!("127.0.0.1:{mac_port}")]);

    let mut mac = start_in("Fixture Mac", mac_dir, |c| c.port = mac_port).await;
    let mut pc = start_in("Fixture PC", pc_dir, |c| c.port = pc_port).await;
    // The same identities as 0.3.1 made
    assert_eq!(mac.core.snapshot().await.unwrap().me.id.to_string(), fixture_id("a"));
    assert_eq!(pc.core.snapshot().await.unwrap().me.id.to_string(), fixture_id("b"));

    let s = mac.wait_for("connected", paired_online).await;
    pc.wait_for("connected", paired_online).await;
    assert!(s.pairing.is_none(), "no pairing asked for");
    let p = s.peers.iter().find(|p| p.paired).unwrap();
    assert_eq!(p.name, "Fixture PC");
    // Settings from before Mission 04 read with today's defaults
    assert!(!s.me.folder_per_device);
}
