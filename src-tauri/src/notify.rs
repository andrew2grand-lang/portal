//! Native notifications. Clicking one brings up the compact accept window
//! when an offer is waiting (Accept/Decline buttons inside notifications are
//! mobile-only in Tauri). Windows shows them only for the installed app.

use tauri::AppHandle;

pub fn show(app: &AppHandle, title: &str, body: &str) {
    imp::show(app, title, body);
}

#[cfg(not(windows))]
mod imp {
    use tauri::AppHandle;
    use tauri_plugin_notification::NotificationExt;

    pub fn show(app: &AppHandle, title: &str, body: &str) {
        // Clicking it activates the app; `macos::on_activate` opens the offer.
        if let Err(e) = app.notification().builder().title(title).body(body).show() {
            tracing::warn!("notification failed: {e}");
        }
    }
}

#[cfg(windows)]
mod imp {
    use tauri::AppHandle;
    use tauri_winrt_notification::Toast;

    pub fn show(app: &AppHandle, title: &str, body: &str) {
        // Installed builds have a Start-menu shortcut registered under the
        // bundle identifier; dev runs borrow PowerShell's identity.
        let app_id =
            if cfg!(debug_assertions) { Toast::POWERSHELL_APP_ID.to_owned() } else { app.config().identifier.clone() };
        let handle = app.clone();
        let toast = Toast::new(&app_id).title(title).text1(body).on_activated(move |_| {
            let h = handle.clone();
            let _ = handle.run_on_main_thread(move || crate::shell::open_for_attention(&h));
            Ok(())
        });
        if let Err(e) = toast.show() {
            tracing::warn!("toast failed: {e}");
        }
    }
}
