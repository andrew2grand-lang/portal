//! Windows of the app: the main window (hides on close) and the compact
//! accept window that notifications and the tray open.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const MAIN: &str = "main";
pub const ACCEPT: &str = "accept";

/// Set by "Quit": the only way the app really exits.
pub static QUITTING: AtomicBool = AtomicBool::new(false);
/// An incoming offer is waiting for an answer (kept current by the bridge).
pub static OFFER_PENDING: AtomicBool = AtomicBool::new(false);

/// Something arrived while the window was hidden or behind (its notification
/// was shown): the next activation shows the window, on the done screen.
pub static DELIVERY_PENDING: AtomicBool = AtomicBool::new(false);

/// Settings → Appearance, as last applied to the windows
static APPEARANCE: std::sync::Mutex<Option<portal_core::model::Appearance>> = std::sync::Mutex::new(None);

/// The OS draws every window (title bar, and what the page's
/// `prefers-color-scheme` says) in the chosen appearance: System follows the
/// OS, Light and Dark are fixed (Mission 04 §1.7).
pub fn apply_appearance(app: &AppHandle, appearance: portal_core::model::Appearance) {
    let mut last = APPEARANCE.lock().unwrap_or_else(|e| e.into_inner());
    if *last == Some(appearance) {
        return;
    }
    *last = Some(appearance);
    for w in app.webview_windows().values() {
        let _ = w.set_theme(theme_of(appearance));
    }
}

fn theme_of(appearance: portal_core::model::Appearance) -> Option<tauri::Theme> {
    use portal_core::model::Appearance;
    match appearance {
        Appearance::System => None,
        Appearance::Light => Some(tauri::Theme::Light),
        Appearance::Dark => Some(tauri::Theme::Dark),
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn main_visible(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN).and_then(|w| w.is_visible().ok()).unwrap_or(false)
}

pub fn main_focused(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN).and_then(|w| w.is_focused().ok()).unwrap_or(false)
}

/// The compact accept / decline window (created on demand, one at a time).
pub fn open_accept(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(ACCEPT) {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let title = app.package_info().name.clone();
    let built = WebviewWindowBuilder::new(app, ACCEPT, WebviewUrl::App("index.html?view=accept".into()))
        .title(title)
        .inner_size(440.0, 560.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .always_on_top(true)
        .center()
        .focused(true)
        .theme(APPEARANCE.lock().unwrap_or_else(|e| e.into_inner()).and_then(theme_of))
        .build();
    if let Err(e) = built {
        tracing::warn!("couldn't open the accept window: {e}");
        show_main(app);
    }
}

/// Opens what the person most likely wants: the offer if one waits, else the app.
pub fn open_for_attention(app: &AppHandle) {
    if OFFER_PENDING.load(Ordering::SeqCst) && !main_visible(app) {
        open_accept(app);
    } else {
        show_main(app);
    }
}

pub fn quit(app: &AppHandle) {
    QUITTING.store(true, Ordering::SeqCst);
    // A transfer that's running pauses and saves where it got to, so it
    // continues after the next start (bounded: quitting never hangs).
    if let Some(bridge) = app.try_state::<crate::bridge::Bridge>() {
        let core = bridge.core.clone();
        tauri::async_runtime::block_on(async {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(4), core.shutdown()).await;
        });
    }
    app.exit(0);
}
