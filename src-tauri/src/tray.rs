//! Menu bar (macOS) / system tray (Windows): what's happening, every paired
//! device with how it's connected, the clipboard (send, sync, pause, off),
//! a way back into the app, and Quit. A dot on the icon means files wait.
//!
//! The menu is rebuilt from the snapshot whenever its lines change (devices
//! come and go); the status line alone follows transfer progress.

use std::sync::Mutex;

use portal_core::events::{Direction, LinkKind, PairStep, PeerView, ReachState, Snapshot, TransferPhase};
use portal_core::model::Appearance;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::shell;

/// One line of the menu, as data (so a rebuild happens only when it changes).
#[derive(Clone, Debug, PartialEq)]
enum Line {
    Text(String),
    Action {
        id: &'static str,
        text: String,
        enabled: bool,
    },
    Sub {
        text: String,
        items: Vec<(&'static str, String)>,
    },
    /// A submenu with one item ticked (Appearance)
    Pick {
        text: String,
        items: Vec<(&'static str, String, bool)>,
    },
    Sep,
}

pub struct Tray {
    icon: TrayIcon,
    /// The first line, kept to follow progress without a rebuild.
    status: Mutex<Option<MenuItem<Wry>>>,
    lines: Mutex<Vec<Line>>,
    /// The update line: (text, enabled), set by `update_offer`.
    update: Mutex<(String, bool)>,
    last: Mutex<Option<Snapshot>>,
    alert: std::sync::atomic::AtomicBool,
}

fn image(alert: bool) -> Image<'static> {
    let bytes: &'static [u8] = match (cfg!(target_os = "macos"), alert) {
        (true, false) => include_bytes!("../icons/tray/mac.png"),
        (true, true) => include_bytes!("../icons/tray/mac-alert.png"),
        (false, false) => include_bytes!("../icons/tray/win.png"),
        (false, true) => include_bytes!("../icons/tray/win-alert.png"),
    };
    Image::from_bytes(bytes).expect("bundled tray icon")
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let name = app.package_info().name.clone();
    let version = format!("{name} {}", app.package_info().version);
    let starting = vec![
        Line::Text("Starting…".into()),
        Line::Action { id: "open", text: format!("Open {name}"), enabled: true },
        Line::Sep,
        Line::Action { id: "update", text: version.clone(), enabled: false },
        Line::Action { id: "quit", text: format!("Quit {name}"), enabled: true },
    ];
    let (menu, status) = menu_of(app, &starting)?;
    let icon = TrayIconBuilder::with_id("portal")
        .icon(image(false))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(&name)
        .menu(&menu)
        // macOS: the menu on click, like every menu bar item. Windows: a left
        // click opens the app (or the waiting offer); right click is the menu.
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event
                && !cfg!(target_os = "macos")
            {
                shell::open_for_attention(tray.app_handle());
            }
        })
        .build(app)?;
    app.manage(Tray {
        icon,
        status: Mutex::new(status),
        lines: Mutex::new(starting),
        update: Mutex::new((version, false)),
        last: Mutex::new(None),
        alert: false.into(),
    });
    Ok(())
}

fn on_menu(app: &AppHandle, id: &str) {
    let spawn = |f: fn(AppHandle) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>| {
        let app = app.clone();
        tauri::async_runtime::spawn(f(app));
    };
    match id {
        "open" => shell::show_main(app),
        "review" => shell::open_accept(app),
        "send_clip" => spawn(|app| Box::pin(async move { crate::clipboard::send(&app).await })),
        "update" => spawn(|app| Box::pin(async move { crate::updates::restart(&app).await })),
        "clip_pause_hour" => spawn(|app| {
            Box::pin(async move {
                with_core(&app, |c| async move { c.pause_clip_sync(Some(std::time::Duration::from_secs(3600))).await })
                    .await
            })
        }),
        "clip_pause_restart" => spawn(|app| {
            Box::pin(async move { with_core(&app, |c| async move { c.pause_clip_sync(None).await }).await })
        }),
        "clip_resume" => {
            spawn(|app| Box::pin(async move { with_core(&app, |c| async move { c.resume_clip_sync().await }).await }))
        }
        "clip_off" => {
            spawn(|app| Box::pin(async move { with_core(&app, |c| async move { c.clip_sync_off().await }).await }))
        }
        "look_system" => appearance(app, Appearance::System),
        "look_light" => appearance(app, Appearance::Light),
        "look_dark" => appearance(app, Appearance::Dark),
        "quit" => shell::quit(app),
        _ => {}
    }
}

/// The tray's Appearance switch: the same setting as Settings → Appearance.
fn appearance(app: &AppHandle, value: Appearance) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let patch = portal_core::node::SettingsPatch { appearance: Some(value), ..Default::default() };
        with_core(&app, |c| async move { c.update_settings(patch).await }).await;
    });
}

async fn with_core<F, Fut>(app: &AppHandle, f: F)
where
    F: FnOnce(portal_core::Core) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if let Some(b) = app.try_state::<crate::bridge::Bridge>() {
        f(b.core.clone()).await;
    }
}

fn menu_of(app: &AppHandle, lines: &[Line]) -> tauri::Result<(Menu<Wry>, Option<MenuItem<Wry>>)> {
    let menu = Menu::new(app)?;
    let mut status = None;
    for (i, line) in lines.iter().enumerate() {
        match line {
            Line::Text(text) => {
                let item = MenuItem::with_id(app, format!("text{i}"), text, false, None::<&str>)?;
                if i == 0 {
                    status = Some(item.clone());
                }
                menu.append(&item)?;
            }
            Line::Action { id, text, enabled } => {
                menu.append(&MenuItem::with_id(app, *id, text, *enabled, None::<&str>)?)?
            }
            Line::Sub { text, items } => {
                let children: Vec<MenuItem<Wry>> = items
                    .iter()
                    .map(|(id, t)| MenuItem::with_id(app, *id, t, true, None::<&str>))
                    .collect::<tauri::Result<_>>()?;
                let refs: Vec<&dyn IsMenuItem<Wry>> = children.iter().map(|c| c as &dyn IsMenuItem<Wry>).collect();
                menu.append(&Submenu::with_items(app, text, true, &refs)?)?;
            }
            Line::Pick { text, items } => {
                let children: Vec<CheckMenuItem<Wry>> = items
                    .iter()
                    .map(|(id, t, on)| CheckMenuItem::with_id(app, *id, t, true, *on, None::<&str>))
                    .collect::<tauri::Result<_>>()?;
                let refs: Vec<&dyn IsMenuItem<Wry>> = children.iter().map(|c| c as &dyn IsMenuItem<Wry>).collect();
                menu.append(&Submenu::with_items(app, text, true, &refs)?)?;
            }
            Line::Sep => menu.append(&PredefinedMenuItem::separator(app)?)?,
        }
    }
    Ok((menu, status))
}

fn link_word(link: Option<LinkKind>) -> &'static str {
    match link {
        Some(LinkKind::Cable | LinkKind::Ethernet) => "Ethernet",
        Some(LinkKind::Wifi) => "Wi-Fi",
        _ => "connected",
    }
}

fn unreachable(p: &PeerView) -> bool {
    p.reach.as_ref().is_some_and(|r| r.state == ReachState::Unreachable)
}

/// "Desk PC · Wi-Fi", "MacBook Air · connecting", "Studio · not reachable"
fn device_line(p: &PeerView) -> String {
    let how = if p.disconnected {
        "disconnected"
    } else if p.incompatible.is_some() {
        "needs an update"
    } else if p.online {
        link_word(p.link)
    } else if unreachable(p) {
        "not reachable"
    } else {
        "connecting"
    };
    format!("{} · {how}", p.name)
}

fn status_line(s: &Snapshot) -> String {
    let offers: Vec<&str> = s
        .transfers
        .iter()
        .filter(|t| t.direction == Direction::In && t.phase == TransferPhase::Offered)
        .map(|t| t.peer_name.as_str())
        .collect();
    let running: Vec<_> = s.transfers.iter().filter(|t| t.phase == TransferPhase::Running).collect();
    let paused: Vec<_> = s.transfers.iter().filter(|t| t.phase == TransferPhase::Paused).collect();
    let paired: Vec<&PeerView> = s.peers.iter().filter(|p| p.paired).collect();
    if let [one, rest @ ..] = offers.as_slice() {
        return if rest.is_empty() {
            format!("{one} wants to send")
        } else {
            format!("{one} and {} more want to send", rest.len())
        };
    }
    match running.as_slice() {
        [t] => {
            let pct = (t.bytes * 100).checked_div(t.total_bytes).unwrap_or(0);
            return match t.direction {
                Direction::In => format!("Receiving from {} · {pct}%", t.peer_name),
                Direction::Out => format!("Sending to {} · {pct}%", t.peer_name),
            };
        }
        [_, ..] => {
            let (done, total) = running.iter().fold((0, 0), |(d, n), t| (d + t.bytes, n + t.total_bytes));
            let pct = (done * 100).checked_div(total).unwrap_or(0);
            return format!("{} transfers · {pct}%", running.len());
        }
        [] => {}
    }
    if let [t, ..] = paused.as_slice() {
        let pct = (t.bytes * 100).checked_div(t.total_bytes).unwrap_or(0);
        return format!("Paused at {pct}% · waiting for {}", t.peer_name);
    }
    if paired.is_empty() {
        return if s.peers.is_empty() { "Looking for your other computer".into() } else { "Not paired yet".into() };
    }
    if paired.iter().any(|p| p.online) {
        return "Ready".into();
    }
    match paired.as_slice() {
        [p] if p.disconnected => format!("Disconnected from {}", p.name),
        [p] if unreachable(p) => format!("{} isn’t reachable", p.name),
        [p] => format!("Connecting to {}…", p.name),
        _ if paired.iter().all(|p| unreachable(p) || p.disconnected) => "No device reachable".into(),
        _ => "Connecting…".into(),
    }
}

fn lines_of(app: &AppHandle, s: &Snapshot, update: &(String, bool)) -> Vec<Line> {
    let name = app.package_info().name.clone();
    let offer = s.transfers.iter().any(|t| t.direction == Direction::In && t.phase == TransferPhase::Offered);
    let mut lines = vec![
        Line::Text(status_line(s)),
        Line::Action { id: "review", text: "Review incoming files…".into(), enabled: offer },
    ];
    // A device asks to pair: the question is in the window (Mission 04 §2)
    if let Some(p) = s.pairing.as_ref().filter(|p| !p.mine && p.step == PairStep::Asked && p.failed.is_none()) {
        lines.push(Line::Action { id: "open", text: format!("{} wants to pair…", p.peer_name), enabled: true });
    }
    lines.push(Line::Action { id: "open", text: format!("Open {name}"), enabled: true });
    lines.push(Line::Sep);
    let paired: Vec<&PeerView> = s.peers.iter().filter(|p| p.paired).collect();
    if paired.is_empty() {
        lines.push(Line::Text("Not paired yet".into()));
    }
    lines.extend(paired.iter().map(|p| Line::Text(device_line(p))));
    lines.push(Line::Sep);

    // Send clipboard goes to the remembered choice (Settings says so).
    let chosen: Vec<&PeerView> =
        s.me.send_to.iter().filter_map(|id| paired.iter().find(|p| &p.id == id).copied()).collect();
    let online: Vec<String> = chosen.iter().filter(|p| p.online).map(|p| p.name.clone()).collect();
    lines.push(Line::Action {
        id: "send_clip",
        text: if online.is_empty() {
            "Send clipboard".into()
        } else {
            format!("Send clipboard to {}", crate::updates::names(&online))
        },
        enabled: !online.is_empty(),
    });
    // Never silent while sync is on: who with, or that it's paused.
    let syncing: Vec<String> =
        paired.iter().filter(|p| p.clip.as_ref().is_some_and(|c| c.mine && c.theirs)).map(|p| p.name.clone()).collect();
    let mine = paired.iter().any(|p| p.clip.as_ref().is_some_and(|c| c.mine));
    let asking: Vec<String> =
        paired.iter().filter(|p| p.clip.as_ref().is_some_and(|c| c.asks)).map(|p| p.name.clone()).collect();
    let sync_text = match (s.me.clipboard_paused, syncing.is_empty()) {
        (Some(0), _) => "Clipboard sync paused until SeamDrop restarts".to_owned(),
        // The window shows when it ends, in local time
        (Some(_), _) => "Clipboard sync paused (1 hour)".to_owned(),
        (None, false) => format!("Clipboard sync on with {}", crate::updates::names(&syncing)),
        (None, true) if mine => "Clipboard sync: waiting for the other side".into(),
        (None, true) if !asking.is_empty() => format!("{} asks to sync the clipboard", crate::updates::names(&asking)),
        _ => "Clipboard sync off".into(),
    };
    lines.push(Line::Text(sync_text));
    if mine {
        if s.me.clipboard_paused.is_some() {
            lines.push(Line::Action { id: "clip_resume", text: "Resume clipboard sync".into(), enabled: true });
        } else {
            lines.push(Line::Sub {
                text: "Pause clipboard sync".into(),
                items: vec![
                    ("clip_pause_hour", "For 1 hour".into()),
                    ("clip_pause_restart", format!("Until {name} restarts")),
                ],
            });
        }
        lines.push(Line::Action { id: "clip_off", text: "Turn off clipboard sync".into(), enabled: true });
    }
    lines.push(Line::Sep);
    let look = s.me.appearance;
    lines.push(Line::Pick {
        text: "Appearance".into(),
        items: vec![
            ("look_system", "System".into(), look == Appearance::System),
            ("look_light", "Light".into(), look == Appearance::Light),
            ("look_dark", "Dark".into(), look == Appearance::Dark),
        ],
    });
    lines.push(Line::Action { id: "update", text: update.0.clone(), enabled: update.1 });
    lines.push(Line::Action { id: "quit", text: format!("Quit {name}"), enabled: true });
    lines
}

fn refresh(app: &AppHandle, tray: &Tray) {
    let Some(s) = tray.last.lock().unwrap_or_else(|e| e.into_inner()).clone() else { return };
    let update = tray.update.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let lines = lines_of(app, &s, &update);
    let mut last = tray.lines.lock().unwrap_or_else(|e| e.into_inner());
    if *last == lines {
        return;
    }
    // Only the status line changed: no rebuild (the menu may be open).
    let only_status = last.len() == lines.len() && last.iter().zip(&lines).skip(1).all(|(a, b)| a == b);
    if only_status
        && let (Some(item), Line::Text(text)) =
            (tray.status.lock().unwrap_or_else(|e| e.into_inner()).as_ref(), &lines[0])
    {
        let _ = item.set_text(text);
    } else if let Ok((menu, status)) = menu_of(app, &lines) {
        let _ = tray.icon.set_menu(Some(menu));
        *tray.status.lock().unwrap_or_else(|e| e.into_inner()) = status;
    }
    *last = lines;
}

/// Keeps the menu and the icon in step with the core.
pub fn update(app: &AppHandle, s: &Snapshot) {
    let Some(tray) = app.try_state::<Tray>() else { return };
    *tray.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(s.clone());
    refresh(app, &tray);
    let alert = s.transfers.iter().any(|t| t.direction == Direction::In && t.phase == TransferPhase::Offered);
    if tray.alert.swap(alert, std::sync::atomic::Ordering::SeqCst) != alert {
        let _ = tray.icon.set_icon(Some(image(alert)));
        #[cfg(target_os = "macos")]
        let _ = tray.icon.set_icon_as_template(true);
    }
    let status = status_line(s);
    let sync = if s.me.clipboard_sync { " · clipboard sync on" } else { "" };
    let _ = tray.icon.set_tooltip(Some(format!("{} · {status}{sync}", app.package_info().name)));
}

/// Live percentage during one transfer, without a full snapshot.
pub fn progress(app: &AppHandle, direction_in: bool, peer: &str, pct: u64) {
    if let Some(tray) = app.try_state::<Tray>()
        && let Some(item) = tray.status.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
    {
        let text = if direction_in {
            format!("Receiving from {peer} · {pct}%")
        } else {
            format!("Sending to {peer} · {pct}%")
        };
        let _ = item.set_text(text);
    }
}

/// The update line: the version, or the way to install a waiting update.
pub fn update_offer(app: &AppHandle, v: &crate::updates::UpdateView) {
    use crate::updates::State;
    let Some(tray) = app.try_state::<Tray>() else { return };
    let name = app.package_info().name.clone();
    let next = v.version.clone().unwrap_or_default();
    let line = match &v.state {
        State::Ready => (format!("Restart {name} to update to {next}"), true),
        State::Available => (format!("Update to {next} and restart {name}"), true),
        State::Downloading { percent } => (format!("Downloading {name} {next} · {percent}%"), false),
        State::Waiting { peer } => (format!("Updates when the transfer with {peer} ends"), false),
        State::Installing => (format!("Installing {name} {next}…"), false),
        _ => (format!("{name} {}", v.current), false),
    };
    *tray.update.lock().unwrap_or_else(|e| e.into_inner()) = line;
    refresh(app, &tray);
}
