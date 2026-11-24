//! The update check against a local test feed (Mission 03 §3): a newer
//! version, the same version, a bad signature, a signature for another
//! version, and no network. Signed with a test-only key (its public key and
//! signatures are in `fixtures/updater`).

use std::net::SocketAddr;
use std::path::PathBuf;

use portal_app_lib::updates::{BETA_FEED, Found, STABLE_FEED, feeds, fetch};
use tauri::test::{MockRuntime, mock_builder, mock_context, noop_assets};
use tauri_plugin_updater::{Updater, UpdaterExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/updater").join(name)
}

fn read(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).unwrap()
}

/// A tiny HTTP server: `/latest.json` announces `version` with `signature`;
/// anything else is the package. Returns the feed's URL.
async fn feed(version: &str, signature: &str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let manifest = serde_json::json!({
        "version": version,
        "notes": "Transfers pick up where they stopped.",
        "pub_date": "2026-11-01T12:00:00Z",
        "platforms": { "test": { "url": format!("http://{addr}/update.bin"), "signature": signature } }
    })
    .to_string();
    let package = std::fs::read(fixture("update.bin")).unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { return };
            let (manifest, package) = (manifest.clone(), package.clone());
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let (kind, body) = if buf[..n].starts_with(b"GET /latest.json") {
                    ("application/json", manifest.into_bytes())
                } else {
                    ("application/octet-stream", package)
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes()).await;
                let _ = stream.write_all(&body).await;
            });
        }
    });
    format!("http://{addr}/latest.json")
}

/// The app runs version 0.1.0 (the mock context's) and trusts the test key.
fn updater(endpoint: &str) -> (tauri::App<MockRuntime>, Updater) {
    updater_at("0.1.0", endpoint)
}

/// The same, with the app at `version`.
fn updater_at(version: &str, endpoint: &str) -> (tauri::App<MockRuntime>, Updater) {
    let mut ctx = mock_context(noop_assets());
    ctx.package_info_mut().version = version.parse().unwrap();
    ctx.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({
            "pubkey": read("test.key.pub"),
            "endpoints": ["https://example.invalid/latest.json"],
            "requireSignedVersion": true
        }),
    );
    let app = mock_builder().plugin(tauri_plugin_updater::Builder::new().build()).build(ctx).unwrap();
    let updater =
        app.updater_builder().endpoints(vec![endpoint.parse().unwrap()]).unwrap().target("test").build().unwrap();
    (app, updater)
}

#[tokio::test]
async fn a_newer_version_is_downloaded_and_checked() {
    let (_app, updater) = updater(&feed("0.2.0", &read("update.bin.sig")).await);
    let mut percents = Vec::new();
    match fetch(std::slice::from_ref(&updater), true, "0.1.0", |p| percents.push(p)).await {
        Ok(Found::Ready(update, bytes)) => {
            assert_eq!(update.version, "0.2.0");
            assert_eq!(update.body.as_deref(), Some("Transfers pick up where they stopped."));
            assert_eq!(bytes, std::fs::read(fixture("update.bin")).unwrap());
            assert_eq!(percents.last(), Some(&100));
        }
        Ok(_) => panic!("expected a ready update"),
        Err(e) => panic!("{e}"),
    }
}

#[tokio::test]
async fn with_automatic_installs_off_it_is_only_offered() {
    let (_app, updater) = updater(&feed("0.2.0", &read("update.bin.sig")).await);
    let found = fetch(std::slice::from_ref(&updater), false, "0.1.0", |_| {}).await;
    assert!(matches!(found, Ok(Found::Available(u)) if u.version == "0.2.0"));
}

#[tokio::test]
async fn the_same_version_is_up_to_date() {
    let (_app, updater) = updater(&feed("0.1.0", &read("update.bin.sig")).await);
    assert!(matches!(fetch(std::slice::from_ref(&updater), true, "0.1.0", |_| {}).await, Ok(Found::UpToDate)));
}

/// Signed for another file, or for another version than the feed announces.
#[tokio::test]
async fn a_bad_signature_is_refused_in_plain_words() {
    for sig in ["other-file.sig", "wrong-version.sig"] {
        let (_app, updater) = updater(&feed("0.2.0", &read(sig)).await);
        match fetch(std::slice::from_ref(&updater), true, "0.1.0", |_| {}).await {
            Err(e) => assert_eq!(
                e,
                "The update didn’t download correctly, so it was thrown away. Nothing on this computer changed; SeamDrop 0.1.0 keeps running and tries again later.",
                "{sig}"
            ),
            Ok(_) => panic!("{sig}: a bad signature must not pass"),
        }
    }
}

#[tokio::test]
async fn no_network_is_said_in_plain_words() {
    // A port nothing listens on.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let (_app, updater) = updater(&format!("http://{closed}/latest.json"));
    match fetch(std::slice::from_ref(&updater), true, "0.1.0", |_| {}).await {
        Err(e) => {
            assert_eq!(
                e,
                "Couldn’t reach the update server. Check the internet connection; SeamDrop tries again later."
            )
        }
        Ok(_) => panic!("nothing should be found offline"),
    }
}

/// What a check offers when installs are manual (nothing is downloaded).
async fn offered(updaters: &[Updater], current: &str) -> Option<String> {
    match fetch(updaters, false, current, |_| {}).await {
        Ok(Found::Available(u)) => Some(u.version),
        Ok(Found::UpToDate | Found::NothingYet) => None,
        Ok(Found::Ready(..)) => panic!("nothing should download"),
        Err(e) => panic!("{e}"),
    }
}

/// Mission 05 §8.2: with test builds on, both feeds are checked and the
/// higher version wins, so a beta user still gets the stable release.
#[tokio::test]
async fn with_test_builds_the_higher_of_the_two_feeds_wins() {
    let sig = read("update.bin.sig");
    for (stable, beta, wins) in [("0.2.0", "0.3.0-beta.1", "0.3.0-beta.1"), ("0.3.0", "0.3.0-beta.2", "0.3.0")] {
        let (_a, s) = updater(&feed(stable, &sig).await);
        let (_b, b) = updater(&feed(beta, &sig).await);
        assert_eq!(offered(&[s, b], "0.1.0").await.as_deref(), Some(wins), "{stable} / {beta}");
    }
}

/// Turning test builds off never goes back: on a beta, only a newer stable
/// release is offered (the updater's own comparator, unchanged).
#[tokio::test]
async fn turning_test_builds_off_never_goes_back() {
    let sig = read("update.bin.sig");
    let (_a, older) = updater_at("0.3.0-beta.1", &feed("0.2.0", &sig).await);
    assert_eq!(offered(&[older], "0.3.0-beta.1").await, None, "an older stable release isn't offered");
    let (_b, same_minor) = updater_at("0.3.0-beta.1", &feed("0.3.0", &sig).await);
    assert_eq!(
        offered(&[same_minor], "0.3.0-beta.1").await.as_deref(),
        Some("0.3.0"),
        "the stable release that passes it is"
    );
}

/// A feed that can't be read doesn't hide an update the other one has.
#[tokio::test]
async fn one_feed_down_doesnt_hide_the_other() {
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let (_a, down) = updater(&format!("http://{closed}/latest.json"));
    let (_b, up) = updater(&feed("0.2.0", &read("update.bin.sig")).await);
    assert_eq!(offered(&[down, up], "0.1.0").await.as_deref(), Some("0.2.0"));
}

/// The stable feed is the one tauri.conf.json names; the beta feed is only
/// read with test builds on.
#[test]
fn the_beta_feed_is_read_only_with_test_builds_on() {
    let conf: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(conf["plugins"]["updater"]["endpoints"], serde_json::json!([STABLE_FEED]));
    assert_eq!(feeds(false), [STABLE_FEED]);
    assert_eq!(feeds(true), [STABLE_FEED, BETA_FEED]);
}

/// A feed that isn't published yet: 404, as GitHub answers
/// `releases/latest/download/latest.json` before the first stable release.
async fn empty_feed() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { return };
            tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let _ = stream
                    .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found")
                    .await;
            });
        }
    });
    format!("http://{addr}/latest.json")
}

/// beta.2: with no stable release yet, the stable feed is empty, and that
/// isn't "the update information couldn't be read".
#[tokio::test]
async fn an_empty_stable_feed_is_nothing_yet_not_an_error() {
    let (_a, stable) = updater(&empty_feed().await);
    assert!(matches!(fetch(&[stable], false, "0.1.0", |_| {}).await, Ok(Found::NothingYet)));
}

/// Test builds on, the stable feed empty, the beta feed on this version:
/// up to date ("You're on the latest test build").
#[tokio::test]
async fn an_empty_stable_feed_beside_the_beta_one_is_up_to_date() {
    let (_a, stable) = updater_at("0.3.0-beta.1", &empty_feed().await);
    let (_b, beta) = updater_at("0.3.0-beta.1", &feed("0.3.0-beta.1", &read("update.bin.sig")).await);
    assert!(matches!(fetch(&[stable, beta], false, "0.3.0-beta.1", |_| {}).await, Ok(Found::UpToDate)));
    // …and a newer beta beside an empty stable feed is offered
    let (_c, stable) = updater_at("0.3.0-beta.1", &empty_feed().await);
    let (_d, beta) = updater_at("0.3.0-beta.1", &feed("0.3.0-beta.2", &read("update.bin.sig")).await);
    assert!(
        matches!(fetch(&[stable, beta], false, "0.3.0-beta.1", |_| {}).await, Ok(Found::Available(u)) if u.version == "0.3.0-beta.2")
    );
}

/// A real failure is still one: nothing listening beside an empty feed.
#[tokio::test]
async fn a_feed_that_cant_be_reached_is_still_an_error() {
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
    let (_a, stable) = updater(&empty_feed().await);
    let (_b, beta) = updater(&format!("http://{closed}/beta.json"));
    assert!(fetch(&[stable, beta], false, "0.1.0", |_| {}).await.is_err());
}
