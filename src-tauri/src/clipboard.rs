//! Send clipboard from anywhere (Mission 03 §4): a global shortcut and the
//! tray. The work is the core's; this registers the shortcut, lets Settings
//! change it, and says what happened when the window isn't in front.

use std::str::FromStr;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// ⌃⌥⌘C on a Mac, Ctrl+Alt+Shift+C on Windows: neither is a common shortcut
/// in either system or in the apps people use most.
pub fn default_shortcut() -> &'static str {
    if cfg!(target_os = "macos") { "Control+Alt+Super+C" } else { "Control+Alt+Shift+C" }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutView {
    /// In accelerator form ("Control+Alt+Super+C"); the window shows symbols.
    pub shortcut: String,
    pub default: String,
    /// Why it isn't working (another app has it), if it isn't.
    pub error: Option<String>,
}

pub struct Shortcuts(Mutex<ShortcutView>);

/// Registers the saved shortcut (or the default) at start.
pub fn start(app: &AppHandle) {
    app.manage(Shortcuts(Mutex::new(ShortcutView {
        shortcut: default_shortcut().into(),
        default: default_shortcut().into(),
        error: None,
    })));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let saved = match app.try_state::<crate::bridge::Bridge>() {
            Some(b) => b.core.snapshot().await.ok().and_then(|s| s.me.clipboard_shortcut),
            None => None,
        };
        let wanted = saved.unwrap_or_else(|| default_shortcut().into());
        if let Err(e) = register(&app, &wanted) {
            tracing::warn!("couldn't register the clipboard shortcut {wanted}: {e}");
            set_view(&app, wanted, Some(e));
        }
    });
}

fn set_view(app: &AppHandle, shortcut: String, error: Option<String>) {
    if let Some(s) = app.try_state::<Shortcuts>() {
        let mut v = s.0.lock().unwrap_or_else(|e| e.into_inner());
        v.shortcut = shortcut;
        v.error = error;
    }
}

pub fn view(app: &AppHandle) -> ShortcutView {
    app.try_state::<Shortcuts>().map(|s| s.0.lock().unwrap_or_else(|e| e.into_inner()).clone()).unwrap_or_default()
}

/// Makes `wanted` the shortcut (replacing the current one).
pub fn register(app: &AppHandle, wanted: &str) -> Result<(), String> {
    let shortcut =
        Shortcut::from_str(wanted).map_err(|_| "That isn’t a shortcut SeamDrop can use. Try another.".to_owned())?;
    let gs = app.global_shortcut();
    let current = view(app).shortcut;
    if let Ok(old) = Shortcut::from_str(&current) {
        let _ = gs.unregister(old);
    }
    let result = gs.on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let app = app.clone();
            tauri::async_runtime::spawn(async move { send(&app).await });
        }
    });
    match result {
        Ok(()) => {
            set_view(app, wanted.to_owned(), None);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("shortcut {wanted}: {e}");
            // Keep the one that worked.
            if Shortcut::from_str(&current).is_ok() && current != wanted {
                let _ = register(app, &current);
            }
            Err("Another app uses that shortcut, or it can’t be used. Try another.".into())
        }
    }
}

/// Settings: a new shortcut ("" puts the default back); saved if it works.
pub async fn change(app: &AppHandle, wanted: String) -> Result<ShortcutView, String> {
    let wanted = if wanted.trim().is_empty() { default_shortcut().to_owned() } else { wanted };
    register(app, &wanted)?;
    if let Some(b) = app.try_state::<crate::bridge::Bridge>() {
        let saved = if wanted == default_shortcut() { String::new() } else { wanted.clone() };
        b.core
            .update_settings(portal_core::node::SettingsPatch { clipboard_shortcut: Some(saved), ..Default::default() })
            .await;
    }
    Ok(view(app))
}

/// The shortcut or the tray: send, and say what happened if the window
/// isn't in front (it shows nothing for this). Files copied in Finder or
/// Explorer go as a transfer straight away: pressing the shortcut was the
/// one step (Mission 04 §4.2).
pub async fn send(app: &AppHandle) {
    let Some(bridge) = app.try_state::<crate::bridge::Bridge>() else { return };
    // The remembered choice (Settings → Clipboard says so)
    let said = match bridge.core.send_clipboard(None).await {
        Ok(sent) if !sent.files.is_empty() => send_files(&bridge.core, sent.files).await,
        Ok(sent) => sent.said,
        Err(s) => s,
    };
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if !(crate::shell::main_visible(&app2) && crate::shell::main_focused(&app2)) {
            crate::notify::show(&app2, &app2.package_info().name.clone(), &said);
        }
    });
}

/// Files from the clipboard to the remembered, connected devices; what to say.
async fn send_files(core: &portal_core::Core, files: Vec<std::path::PathBuf>) -> String {
    let Ok(s) = core.snapshot().await else { return "SeamDrop isn’t ready yet.".into() };
    let online: Vec<_> = s.peers.iter().filter(|p| p.paired && p.online && s.me.send_to.contains(&p.id)).collect();
    if online.is_empty() {
        return "Nothing ticked is connected right now.".into();
    }
    let count = files.len();
    let results = core.send_many(online.iter().map(|p| p.id.clone()).collect(), files).await;
    let started: Vec<String> = results
        .iter()
        .filter(|(_, r)| r.is_ok())
        .filter_map(|(id, _)| online.iter().find(|p| &p.id == id).map(|p| p.name.clone()))
        .collect();
    let what = if count == 1 { "1 file".to_owned() } else { format!("{count} files") };
    match (started.is_empty(), results.iter().find_map(|(_, r)| r.as_ref().err())) {
        (false, _) => format!("Sending {what} from the clipboard to {}.", crate::updates::names(&started)),
        (true, Some(e)) => e.clone(),
        (true, None) => "Nothing was sent.".into(),
    }
}
