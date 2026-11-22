//! Thin Tauri bridge over `portal-core`: commands in, events out, plus the
//! desktop shell (windows, tray, notifications).

mod bridge;
mod clipboard;
mod legacy;
#[cfg(target_os = "macos")]
mod macos;
mod notify;
mod shell;
mod tray;
pub mod updates;

use std::sync::atomic::Ordering;

use portal_core::{Config, Core};
use tauri::{Manager, RunEvent, WindowEvent};

/// Launched by "Open at login": start in the background.
const HIDDEN_ARG: &str = "--hidden";
/// Check for an update at once and, if there is one, install it and restart
/// (support and the end-to-end update test; the same as pressing Restart).
const UPDATE_NOW_ARG: &str = "--update-now";

fn start_core(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(dir) = app.path().app_log_dir() {
        init_logging(&dir);
    }
    tracing::info!("{} {} starting", app.package_info().name, app.package_info().version);
    let data_dir = app.path().app_data_dir()?;
    let version = app.package_info().version.to_string();
    let mut config = Config::new(data_dir.clone(), &version);
    // The app was called Portal: its settings, paired devices and keys come
    // over on the first start (the core logs it in Details).
    let old_dir = data_dir.parent().map(|p| p.join(portal_core::consts::LEGACY_BUNDLE_ID));
    let first_start_after_rename =
        old_dir.as_ref().is_some_and(|d| d.join("identity.json").is_file()) && !data_dir.join("identity.json").exists();
    config.migrate_from = old_dir;
    let (core, events) = tauri::async_runtime::block_on(Core::start(config))?;
    if first_start_after_rename {
        legacy::move_login_item(app.handle());
    }
    app.manage(bridge::Bridge::new(core));
    tauri::async_runtime::spawn(bridge::forward(app.handle().clone(), events));
    Ok(())
}

/// The log file's name in the OS log folder (see `init_logging`)
pub(crate) const LOG_FILE: &str = "seamdrop.log";

/// Logs go to stdout and to `seamdrop.log` in the OS log folder
/// (macOS ~/Library/Logs/com.seamdrop.desktop, Windows
/// %LOCALAPPDATA%\com.seamdrop.desktop\logs), so a release build on
/// Windows, which has no console, can still be diagnosed.
fn init_logging(dir: &std::path::Path) {
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(LOG_FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 5 * 1024 * 1024) {
        let _ = std::fs::rename(&path, dir.join("seamdrop.old.log"));
    }
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            let writer = std::io::stdout.and(std::sync::Mutex::new(file));
            let _ = tracing_subscriber::fmt().with_env_filter(filter).with_ansi(false).with_writer(writer).try_init();
        }
        Err(_) => {
            let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch focuses this instance and exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| shell::open_for_attention(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .invoke_handler(tauri::generate_handler![
            bridge::portal_snapshot,
            bridge::portal_logs,
            bridge::portal_connect,
            bridge::portal_retry,
            bridge::portal_pair,
            bridge::portal_pairing,
            bridge::portal_remove_peer,
            bridge::portal_rename_peer,
            bridge::portal_set_incoming,
            bridge::portal_set_connected,
            bridge::portal_save_settings,
            bridge::portal_send,
            bridge::portal_describe,
            bridge::portal_accept,
            bridge::portal_decline,
            bridge::portal_cancel,
            bridge::portal_continue,
            bridge::portal_send_again,
            bridge::portal_fix_firewall,
            bridge::portal_dismiss,
            bridge::portal_update_status,
            bridge::portal_update_check,
            bridge::portal_update_restart,
            bridge::portal_update_later,
            bridge::portal_send_clipboard,
            bridge::portal_set_clip_sync,
            bridge::portal_clip_sync_off,
            bridge::portal_pause_clip_sync,
            bridge::portal_resume_clip_sync,
            bridge::portal_set_clip_to,
            bridge::portal_copy_clip,
            bridge::portal_copy_technical_log,
            bridge::portal_open_received,
            bridge::portal_open_file,
            bridge::portal_send_text,
            bridge::portal_save_technical_log,
            bridge::portal_clear_clips,
            bridge::portal_remove_clip,
            bridge::portal_unqueue_next,
            bridge::portal_set_picture,
            bridge::portal_shelf_add,
            bridge::portal_shelf_edit,
            bridge::portal_shelf_remove,
            bridge::portal_shelf_undo,
            bridge::portal_shelf_copy,
            bridge::portal_shelf_reveal,
            bridge::portal_set_shelf_share,
            bridge::portal_shelf_put_marked,
            bridge::portal_shelf_put_received,
            bridge::portal_save_clip_image,
            bridge::portal_dismiss_clip_notice,
            bridge::portal_continue_next,
            bridge::portal_shortcut,
            bridge::portal_set_shortcut,
        ])
        // Closing the main window hides it: the app keeps receiving from the
        // menu bar / system tray. "Quit" there is the only real exit.
        .on_window_event(|window, event| {
            if window.label() == shell::MAIN
                && let WindowEvent::CloseRequested { api, .. } = event
                && !shell::QUITTING.load(Ordering::SeqCst)
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            start_core(app)?;
            tray::build(app.handle())?;
            updates::start(app.handle());
            clipboard::start(app.handle());
            if std::env::args().any(|a| a == UPDATE_NOW_ARG) {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    updates::check(&handle).await;
                    updates::restart(&handle).await;
                });
            }
            #[cfg(target_os = "macos")]
            {
                // A clicked notification activates the app: show the waiting offer.
                let handle = app.handle().clone();
                macos::on_activate(move || {
                    if shell::OFFER_PENDING.load(Ordering::SeqCst) && !shell::main_visible(&handle) {
                        shell::open_accept(&handle);
                    } else if shell::DELIVERY_PENDING.swap(false, Ordering::SeqCst) {
                        // A click on "Received …" (macOS reports only that the app
                        // became active): the window, showing the latest delivery.
                        shell::show_main(&handle);
                    }
                });
                portal_core::net::trigger_local_network_prompt();
            }
            if !std::env::args().any(|a| a == HIDDEN_ARG) {
                shell::show_main(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the app")
        .run(|app, event| match event {
            // Last window closed (or hidden): stay alive in the tray.
            RunEvent::ExitRequested { code: None, api, .. } if !shell::QUITTING.load(Ordering::SeqCst) => {
                api.prevent_exit()
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => shell::show_main(app),
            _ => {
                let _ = app;
            }
        });
}
