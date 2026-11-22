//! Updates (Mission 03 §3): checked quietly 10 s after launch and every 4
//! hours from the public releases repo. With "Install updates automatically"
//! on (the default) a new version is downloaded and verified in the
//! background, then offered as "Restart SeamDrop to update"; with it off, it's offered
//! first. The app never restarts while a transfer runs: it waits for it to
//! end and says so. Anything that goes wrong is said in plain words, and the
//! current version keeps running.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Error, Update, Updater, UpdaterExt};

/// Event the UI listens on for the update state.
pub const EVENT: &str = "portal:update";
const FIRST_CHECK: Duration = Duration::from_secs(10);

/// The stable feed (the same as in tauri.conf.json): the newest release
/// that isn't a pre-release.
pub const STABLE_FEED: &str =
    "https://github.com/andrew2grand-lang/seamdrop-releases/releases/latest/download/latest.json";
/// The test-build feed (Mission 05 §8.2): a fixed file in the public
/// releases repo that each beta's publish replaces. The stable feed never
/// sees a beta.
pub const BETA_FEED: &str = "https://raw.githubusercontent.com/andrew2grand-lang/seamdrop-releases/main/beta.json";

/// The feeds to check: the stable one, and with "Get test builds" on the
/// beta one too.
pub fn feeds(test_builds: bool) -> &'static [&'static str] {
    if test_builds { &[STABLE_FEED, BETA_FEED] } else { &[STABLE_FEED] }
}
const CHECK_EVERY: Duration = Duration::from_secs(4 * 60 * 60);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum State {
    /// Not checked yet.
    Idle,
    Checking,
    UpToDate,
    /// Install automatically is off: offered, not downloaded yet.
    Available,
    Downloading {
        percent: u8,
    },
    /// Downloaded and verified: a restart installs it.
    Ready,
    /// Restart pressed while a transfer runs: it restarts when that ends.
    Waiting {
        peer: String,
    },
    Installing,
    Failed {
        message: String,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateView {
    /// The running version.
    pub current: String,
    #[serde(flatten)]
    pub state: State,
    /// The version on offer, and what's new in it.
    pub version: Option<String>,
    pub notes: Option<String>,
    /// "Later" was pressed for this version: no notice until the next one.
    pub dismissed: bool,
    /// Restart was pressed: the window shows the download and the install.
    pub requested: bool,
    /// When it last checked (epoch ms).
    pub checked_ms: Option<u64>,
}

struct Inner {
    view: UpdateView,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
    /// The version "Later" was pressed for.
    later: Option<String>,
    /// An install is under way (downloading, checking for a transfer,
    /// installing): a second request leaves it alone.
    installing: bool,
}

pub struct Updates(Mutex<Inner>);

/// What a check found.
pub enum Found {
    UpToDate,
    /// Offered, not downloaded (install automatically is off).
    Available(Update),
    /// Downloaded and its signature checked.
    Ready(Update, Vec<u8>),
}

/// Checks every feed and keeps the highest version on offer. Each check
/// uses the updater's default comparator (only a version newer than the
/// running one counts), so turning test builds off never goes back: a beta
/// stays until a stable release passes it. A feed that can't be read
/// doesn't hide an update another one has; with no update anywhere, the
/// first failure is said.
pub async fn newest(updaters: &[Updater]) -> Result<Option<Update>, Error> {
    let mut best: Option<Update> = None;
    let mut failed = None;
    for updater in updaters {
        match updater.check().await {
            Ok(Some(u)) => {
                if best.as_ref().is_none_or(|b| higher(&u.version, &b.version)) {
                    best = Some(u);
                }
            }
            Ok(None) => {}
            Err(e) => {
                failed.get_or_insert(e);
            }
        }
    }
    match (best, failed) {
        (Some(u), _) => Ok(Some(u)),
        (None, Some(e)) => Err(e),
        (None, None) => Ok(None),
    }
}

/// `a` is a higher version than `b` (semver: 0.5.0 is above 0.5.0-beta.2).
fn higher(a: &str, b: &str) -> bool {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(a), Ok(b)) => a > b,
        (Ok(_), Err(_)) => true,
        _ => false,
    }
}

/// Checks the feeds and, when `download`, fetches and verifies the newest
/// update. Errors come back as a sentence for the window.
pub async fn fetch(
    updaters: &[Updater],
    download: bool,
    current: &str,
    mut on_percent: impl FnMut(u8),
) -> Result<Found, String> {
    let update = match newest(updaters).await {
        Ok(None) => return Ok(Found::UpToDate),
        Ok(Some(u)) => u,
        Err(e) => return Err(plain(&e, current)),
    };
    if !download {
        return Ok(Found::Available(update));
    }
    let mut got = 0usize;
    let bytes = update
        .download(
            |n, total| {
                got += n;
                if let Some(total) = total.filter(|t| *t > 0) {
                    on_percent(((got as u64 * 100) / total).min(100) as u8);
                }
            },
            || {},
        )
        .await
        .map_err(|e| plain(&e, current))?;
    Ok(Found::Ready(update, bytes))
}

/// The updater's errors in plain words.
pub fn plain(e: &Error, current: &str) -> String {
    let app = portal_core::consts::APP_NAME;
    match e {
        Error::Reqwest(_) | Error::Network(_) | Error::Http(_) => {
            format!("Couldn’t reach the update server. Check the internet connection; {app} tries again later.")
        }
        Error::Minisign(_)
        | Error::Base64(_)
        | Error::SignatureUtf8(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion => format!(
            "The update didn’t download correctly, so it was thrown away. Nothing on this computer changed; {app} {current} keeps running and tries again later."
        ),
        Error::ReleaseNotFound
        | Error::TargetNotFound(_)
        | Error::TargetsNotFound(_)
        | Error::Serialization(_)
        | Error::Semver(_)
        | Error::UrlParse(_) => format!("The update information couldn’t be read. {app} tries again later."),
        other => format!("Couldn’t update ({other}). {app} {current} keeps running."),
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl Updates {
    pub fn new(current: String) -> Self {
        Self(Mutex::new(Inner {
            view: UpdateView {
                current,
                state: State::Idle,
                version: None,
                notes: None,
                dismissed: false,
                requested: false,
                checked_ms: None,
            },
            update: None,
            bytes: None,
            later: None,
            installing: false,
        }))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn view(&self) -> UpdateView {
        self.lock().view.clone()
    }
}

fn set(app: &AppHandle, f: impl FnOnce(&mut Inner)) {
    let Some(updates) = app.try_state::<Updates>() else { return };
    let view = {
        let mut inner = updates.lock();
        f(&mut inner);
        inner.view.dismissed = inner.view.version.is_some() && inner.later == inner.view.version;
        inner.view.clone()
    };
    if let Err(e) = app.emit(EVENT, &view) {
        tracing::warn!("emit failed: {e}");
    }
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || crate::tray::update_offer(&app2, &view));
}

/// Starts the quiet checks: soon after launch, then every few hours.
pub fn start(app: &AppHandle) {
    app.manage(Updates::new(app.package_info().version.to_string()));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            check(&app).await;
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

/// Checks now (also "Check for updates" in Settings). Leaves a download or
/// a ready update alone.
pub async fn check(app: &AppHandle) {
    let busy = app.try_state::<Updates>().is_some_and(|u| {
        matches!(
            u.lock().view.state,
            State::Checking | State::Downloading { .. } | State::Ready | State::Waiting { .. } | State::Installing
        )
    });
    if busy {
        return;
    }
    set(app, |i| i.view.state = State::Checking);
    let (auto, test_builds) = match app.try_state::<crate::bridge::Bridge>() {
        Some(b) => b.core.snapshot().await.map(|s| (s.me.auto_update, s.me.test_builds)).unwrap_or((true, false)),
        None => (true, false),
    };
    let current = app.package_info().version.to_string();
    let updaters = match updaters(app, test_builds) {
        Ok(u) => u,
        Err(e) => return set(app, |i| i.view.state = State::Failed { message: plain(&e, &current) }),
    };
    let progress_app = app.clone();
    let found = fetch(&updaters, auto, &current, |percent| {
        set(&progress_app, |i| i.view.state = State::Downloading { percent })
    })
    .await;
    set(app, |i| {
        i.view.checked_ms = Some(now_ms());
        match found {
            Ok(Found::UpToDate) => {
                i.view.state = State::UpToDate;
                i.view.version = None;
                i.view.notes = None;
            }
            Ok(Found::Available(update)) => {
                i.view.version = Some(update.version.clone());
                i.view.notes = update.body.clone();
                i.view.state = State::Available;
                i.update = Some(update);
            }
            Ok(Found::Ready(update, bytes)) => {
                i.view.version = Some(update.version.clone());
                i.view.notes = update.body.clone();
                i.view.state = State::Ready;
                i.update = Some(update);
                i.bytes = Some(bytes);
            }
            Err(message) => i.view.state = State::Failed { message },
        }
    });
    if let Some(v) = app.try_state::<Updates>().map(|u| u.view()) {
        tracing::info!("update check: {:?} {:?}", v.state, v.version);
    }
}

/// One updater per feed to check.
fn updaters(app: &AppHandle, test_builds: bool) -> Result<Vec<Updater>, Error> {
    feeds(test_builds).iter().map(|feed| app.updater_builder().endpoints(vec![feed.parse()?])?.build()).collect()
}

/// "Restart to update" / "Update and restart". During a transfer it waits.
pub async fn restart(app: &AppHandle) {
    set(app, |i| {
        i.view.requested = true;
        i.later = None;
    });
    if let Some(peer) = running_transfer(app).await {
        tracing::info!("update waits for the transfer with {peer}");
        return set(app, |i| i.view.state = State::Waiting { peer });
    }
    install_now(app).await;
}

/// "Later": no notice for this version (Settings still offers it).
pub fn later(app: &AppHandle) {
    set(app, |i| {
        i.later = i.view.version.clone();
        i.view.requested = false;
        // Not restarting after the transfer either.
        if matches!(i.view.state, State::Waiting { .. }) {
            i.view.state = if i.bytes.is_some() { State::Ready } else { State::Available };
        }
    });
}

/// The bridge calls this on every snapshot: a restart that was waiting for
/// a transfer goes ahead once nothing is moving.
pub fn on_snapshot(app: &AppHandle, s: &portal_core::events::Snapshot) {
    let waiting = app.try_state::<Updates>().is_some_and(|u| matches!(u.lock().view.state, State::Waiting { .. }));
    if waiting && !moving(s) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move { install_now(&app).await });
    }
}

/// A transfer is offered, preparing or running (a paused one resumes after
/// a restart, so it doesn't count).
fn moving(s: &portal_core::events::Snapshot) -> bool {
    !moving_with(s).is_empty()
}

/// The devices transfers are moving with (any number at once).
fn moving_with(s: &portal_core::events::Snapshot) -> Vec<String> {
    use portal_core::events::TransferPhase::*;
    s.transfers
        .iter()
        .filter(|t| matches!(t.phase, Preparing | Offered | Running))
        .map(|t| t.peer_name.clone())
        .collect()
}

/// "Desk PC", "Desk PC and MacBook Air", "Desk PC, MacBook Air and Studio"
pub fn names(list: &[String]) -> String {
    match list {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

async fn running_transfer(app: &AppHandle) -> Option<String> {
    let bridge = app.try_state::<crate::bridge::Bridge>()?;
    let s = bridge.core.snapshot().await.ok()?;
    let with = moving_with(&s);
    (!with.is_empty()).then(|| names(&with))
}

async fn install_now(app: &AppHandle) {
    let current = app.package_info().version.to_string();
    let (update, bytes) = {
        let Some(u) = app.try_state::<Updates>() else { return };
        let mut inner = u.lock();
        if inner.installing || inner.update.is_none() {
            return;
        }
        inner.installing = true;
        (inner.update.clone(), inner.bytes.take())
    };
    let Some(update) = update else { return };
    let fail = |app: &AppHandle, bytes: Option<Vec<u8>>, message: String| {
        set(app, |i| {
            i.installing = false;
            if bytes.is_some() {
                i.bytes = bytes;
            }
            i.view.state = State::Failed { message }
        })
    };
    let bytes = match bytes {
        Some(b) => b,
        None => {
            set(app, |i| i.view.state = State::Downloading { percent: 0 });
            let progress_app = app.clone();
            let mut got = 0usize;
            let downloaded = update
                .download(
                    |n, total| {
                        got += n;
                        if let Some(total) = total.filter(|t| *t > 0) {
                            let percent = ((got as u64 * 100) / total).min(100) as u8;
                            set(&progress_app, |i| i.view.state = State::Downloading { percent });
                        }
                    },
                    || {},
                )
                .await;
            match downloaded {
                Ok(b) => b,
                Err(e) => return fail(app, None, plain(&e, &current)),
            }
        }
    };
    // A transfer may have started while it downloaded: wait for it.
    if let Some(peer) = running_transfer(app).await {
        tracing::info!("update waits for the transfer with {peer}");
        return set(app, |i| {
            i.installing = false;
            i.bytes = Some(bytes);
            i.view.state = State::Waiting { peer };
        });
    }
    set(app, |i| i.view.state = State::Installing);
    // Windows: the installer closes this process itself, so stop the core
    // cleanly first. macOS: the new bundle replaces this one on disk (this
    // process keeps running), and the core stops just before the restart.
    if cfg!(windows) {
        stop_core(app).await;
    }
    if let Err(e) = update.install(&bytes) {
        tracing::warn!("update install failed: {e}");
        if cfg!(windows) {
            // The core is stopped: start again on the version that's here.
            crate::shell::QUITTING.store(true, std::sync::atomic::Ordering::SeqCst);
            app.restart();
        }
        return fail(app, Some(bytes), plain(&e, &current));
    }
    tracing::info!("installed {}; restarting", update.version);
    stop_core(app).await;
    crate::shell::QUITTING.store(true, std::sync::atomic::Ordering::SeqCst);
    app.restart();
}

/// Stop the core cleanly for the update: transfers under way pause and say
/// "update", so they continue by themselves after the restart (Mission 05
/// §1.4; a plain quit asks first).
async fn stop_core(app: &AppHandle) {
    if let Some(bridge) = app.try_state::<crate::bridge::Bridge>() {
        let why = portal_core::transfer::journal::Stopped::Update;
        let _ = tokio::time::timeout(Duration::from_secs(4), bridge.core.shutdown_for(why)).await;
    }
}
