//! Computers on different protocol versions (Portal: 1, SeamDrop 0.3: 2,
//! Mission 04: 3) refuse each other, and SeamDrop says which one to update.

mod common;

use std::time::Duration;

use common::{pair, start};
use portal_core::identity::Identity;
use portal_core::model::{Os, Role};
use portal_core::net::{dial, link};
use portal_core::node::update_advice;
use portal_core::proto::{Hello, PROTOCOL_VERSION};
use portal_core::tls;
use tokio::sync::mpsc;

fn old_hello(proto: u32, id: portal_core::model::DeviceId, name: &str) -> Hello {
    Hello { proto, id, name: name.into(), os: Os::Windows, role: Role::Both, app: "0.3.1".into(), port: 1 }
}

#[test]
fn the_advice_names_the_computer_that_needs_the_update() {
    assert_eq!(PROTOCOL_VERSION, 4, "Mission 05 speaks protocol 4");
    assert_eq!(
        update_advice(2, "Desk PC"),
        "Update SeamDrop on Desk PC. It runs an older version that can’t talk to this one."
    );
    let this = if cfg!(target_os = "macos") {
        "this Mac"
    } else if cfg!(windows) {
        "this PC"
    } else {
        "this computer"
    };
    assert_eq!(
        update_advice(4, "Desk PC"),
        format!("Update SeamDrop on {this}. Desk PC runs a newer version that can’t talk to this one.")
    );
}

/// Connect by IP to a computer still running the old app.
#[tokio::test]
async fn connecting_to_an_old_app_says_to_update_it() {
    let mac = start("MacBook Pro", |_| {}).await;
    let old_dir = tempfile::tempdir().unwrap();
    let old = Identity::load_or_create(old_dir.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tls::server_config(&old).unwrap();
    let id = old.device_id.clone();
    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let tls = link::accept_tls(tcp, server).await.unwrap();
        let (tx, _rx) = mpsc::channel(8);
        let _ = link::handshake(tls, false, old_hello(1, id, "Old PC"), tx).await;
    });

    let err = mac.core.connect(&addr.to_string()).await.unwrap_err();
    assert_eq!(err, "Update SeamDrop on Old PC. It runs an older version that can’t talk to this one.");
}

/// A paired computer comes back running SeamDrop 0.3 (protocol 2): it shows
/// as needing an update, not as offline.
#[tokio::test]
async fn a_paired_computer_back_on_the_old_version_is_marked_for_update() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pair(&mut mac, &mut pc).await;
    let pc_identity = Identity::load_or_create(pc.dir.path()).unwrap();
    pc.core.shutdown().await;
    mac.wait_for("pc gone", |s| s.peers.iter().all(|p| !p.online)).await;

    let tcp = dial::dial(mac.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls = link::connect_tls(tcp, tls::client_config(&pc_identity, None).unwrap()).await.unwrap();
    let (tx, _rx) = mpsc::channel(8);
    // (This side's own check runs against protocol 4; the Mac is the one that refuses.)
    let _ = link::handshake(tls, true, old_hello(2, pc_identity.device_id.clone(), "Desk PC"), tx).await;

    let s = mac
        .wait_for("marked incompatible", |s| s.peers.iter().any(|p| p.name == "Desk PC" && p.incompatible == Some(2)))
        .await;
    assert_eq!(s.me.proto, 4);
}
