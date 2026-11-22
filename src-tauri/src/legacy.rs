//! The rename from Portal: if the old app opened at login, SeamDrop takes
//! that over (its own "Open at login" is turned on and the old login item is
//! removed), so the old app stops starting next to the new one. Best effort,
//! once, on the first start after the rename. The data folder is the core's
//! job (`portal_core::migrate`).

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// The old app's name, as its login item was registered.
const OLD_NAME: &str = "Portal";

pub fn move_login_item(app: &AppHandle) {
    match remove_old_login_item() {
        Ok(true) => match app.autolaunch().enable() {
            Ok(()) => tracing::info!("open at login moved over from {OLD_NAME}"),
            Err(e) => tracing::warn!("couldn't turn on open at login: {e}"),
        },
        Ok(false) => {}
        Err(e) => tracing::warn!("couldn't check {OLD_NAME}'s login item: {e}"),
    }
}

/// `~/Library/LaunchAgents/Portal.plist`, written by the old app's
/// "Open at login". Removed only if it starts the old app.
#[cfg(target_os = "macos")]
fn remove_old_login_item() -> std::io::Result<bool> {
    let Some(home) = std::env::var_os("HOME") else { return Ok(false) };
    let plist = std::path::Path::new(&home).join("Library/LaunchAgents").join(format!("{OLD_NAME}.plist"));
    match std::fs::read_to_string(&plist) {
        Ok(text) if text.contains(&format!("{OLD_NAME}.app/Contents/MacOS/{OLD_NAME}")) => {
            std::fs::remove_file(&plist)?;
            Ok(true)
        }
        Ok(_) => Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// The `Portal` value under HKCU\...\Run, written by the old app's "Open at
/// login". Removed only if it starts the old app.
#[cfg(windows)]
fn remove_old_login_item() -> std::io::Result<bool> {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    const RUN: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    let query =
        std::process::Command::new("reg").args(["query", RUN, "/v", OLD_NAME]).creation_flags(NO_WINDOW).output()?;
    let text = String::from_utf8_lossy(&query.stdout);
    if !query.status.success() || !text.to_lowercase().contains(&format!("{}.exe", OLD_NAME.to_lowercase())) {
        return Ok(false);
    }
    let delete = std::process::Command::new("reg")
        .args(["delete", RUN, "/v", OLD_NAME, "/f"])
        .creation_flags(NO_WINDOW)
        .status()?;
    Ok(delete.success())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn remove_old_login_item() -> std::io::Result<bool> {
    Ok(false)
}
