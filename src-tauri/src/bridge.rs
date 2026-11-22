//! Commands from the webview into the core, events back out. File bytes
//! never cross this boundary: the UI only ever sends paths.

use std::collections::VecDeque;
use std::sync::Mutex;

use portal_core::Core;
use portal_core::events::{Event, ItemView, LogLine, PeerView, Snapshot};
use portal_core::model::{DeviceId, Incoming};
use portal_core::node::SettingsPatch;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;

/// Event channel the UI listens on.
pub const EVENT: &str = "portal:event";
const LOG_HISTORY: usize = 500;

pub struct Bridge {
    pub core: Core,
    logs: Mutex<VecDeque<LogLine>>,
}

impl Bridge {
    pub fn new(core: Core) -> Self {
        Self { core, logs: Mutex::new(VecDeque::with_capacity(LOG_HISTORY)) }
    }

    fn remember(&self, line: &LogLine) {
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        if logs.len() == LOG_HISTORY {
            logs.pop_front();
        }
        logs.push_back(line.clone());
    }
}

/// Forwards core events to every window (keeping recent log lines for windows
/// that open later), and keeps the tray and notifications in step.
pub async fn forward(app: AppHandle, mut events: mpsc::UnboundedReceiver<Event>) {
    use std::sync::atomic::Ordering;

    use portal_core::events::{Direction, TransferPhase};
    use tauri::Manager;

    let mut last_tray_progress = std::time::Instant::now();
    // Running transfers: id → (incoming, peer name), for tray percentages.
    let mut current: std::collections::HashMap<String, (bool, String)> = Default::default();
    while let Some(event) = events.recv().await {
        match &event {
            Event::Log(line) => {
                if let Some(bridge) = app.try_state::<Bridge>() {
                    bridge.remember(line);
                }
            }
            Event::Snapshot(s) => {
                crate::updates::on_snapshot(&app, s);
                crate::shell::apply_appearance(&app, s.me.appearance);
                let offer =
                    s.transfers.iter().any(|t| t.direction == Direction::In && t.phase == TransferPhase::Offered);
                crate::shell::OFFER_PENDING.store(offer, Ordering::SeqCst);
                current = s
                    .transfers
                    .iter()
                    .filter(|t| t.phase == TransferPhase::Running)
                    .map(|t| (t.id.clone(), (t.direction == Direction::In, t.peer_name.clone())))
                    .collect();
                let (app2, snap) = (app.clone(), s.clone());
                let _ = app.run_on_main_thread(move || crate::tray::update(&app2, &snap));
            }
            Event::Progress(p) => {
                // One running transfer: its percentage in the tray (several: the snapshot's summary)
                if last_tray_progress.elapsed() >= std::time::Duration::from_secs(1)
                    && current.len() == 1
                    && let Some((incoming, peer)) = current.get(&p.id).cloned()
                {
                    last_tray_progress = std::time::Instant::now();
                    let pct = (p.bytes * 100).checked_div(p.total).unwrap_or(0);
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || crate::tray::progress(&app2, incoming, &peer, pct));
                }
            }
            Event::Notice(n) => {
                // Only when the window can't already show it.
                let app2 = app.clone();
                let (title, body) = (n.title.clone(), n.body.clone());
                let delivery = n.kind == portal_core::events::NoticeKind::Received;
                let _ = app.run_on_main_thread(move || {
                    if !(crate::shell::main_visible(&app2) && crate::shell::main_focused(&app2)) {
                        // On the Mac a click on the notification only activates
                        // the app (decision 50), so activation shows the latest
                        // delivery (`lib.rs` `on_activate`).
                        if delivery {
                            crate::shell::DELIVERY_PENDING.store(true, std::sync::atomic::Ordering::SeqCst);
                        }
                        crate::notify::show(&app2, &title, &body);
                    }
                });
            }
        }
        if let Err(e) = app.emit(EVENT, &event) {
            tracing::warn!("emit failed: {e}");
        }
    }
}

#[tauri::command]
pub async fn portal_snapshot(bridge: State<'_, Bridge>) -> Result<Snapshot, String> {
    bridge.core.snapshot().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn portal_logs(bridge: State<'_, Bridge>) -> Vec<LogLine> {
    bridge.logs.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect()
}

#[tauri::command]
pub async fn portal_connect(bridge: State<'_, Bridge>, address: String) -> Result<PeerView, String> {
    bridge.core.connect(&address).await
}

#[tauri::command]
pub async fn portal_retry(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.retry_discovery().await;
    Ok(())
}

fn device_id(id: &str) -> Result<DeviceId, String> {
    DeviceId::parse(id).ok_or_else(|| "unknown device".to_owned())
}

#[tauri::command]
pub async fn portal_pair(bridge: State<'_, Bridge>, peer_id: String) -> Result<(), String> {
    bridge.core.pair(&device_id(&peer_id)?).await
}

/// What the person did about a pairing, from the window.
#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PairingAction {
    ShowCode,
    Ignore,
    Code { code: String },
    Cancel,
    Dismiss,
    AllowAgain,
}

#[tauri::command]
pub async fn portal_pairing(bridge: State<'_, Bridge>, action: PairingAction) -> Result<(), String> {
    use portal_core::node::PairAction;
    let action = match action {
        PairingAction::ShowCode => PairAction::ShowCode,
        PairingAction::Ignore => PairAction::Ignore,
        PairingAction::Code { code } => PairAction::Code(code),
        PairingAction::Cancel => PairAction::Cancel,
        PairingAction::Dismiss => PairAction::Dismiss,
        PairingAction::AllowAgain => PairAction::AllowAgain,
    };
    bridge.core.pairing(action).await
}

#[tauri::command]
pub async fn portal_remove_peer(bridge: State<'_, Bridge>, peer_id: String) -> Result<(), String> {
    bridge.core.remove_peer(&device_id(&peer_id)?).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_rename_peer(bridge: State<'_, Bridge>, peer_id: String, name: String) -> Result<(), String> {
    bridge.core.rename_peer(&device_id(&peer_id)?, &name).await;
    Ok(())
}

/// "Files from <device>": ask, always or never.
#[tauri::command]
pub async fn portal_set_incoming(bridge: State<'_, Bridge>, peer_id: String, value: Incoming) -> Result<(), String> {
    bridge.core.set_incoming(&device_id(&peer_id)?, value).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_set_connected(bridge: State<'_, Bridge>, peer_id: String, value: bool) -> Result<(), String> {
    bridge.core.set_connected(&device_id(&peer_id)?, value).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_save_settings(bridge: State<'_, Bridge>, patch: SettingsPatch) -> Result<(), String> {
    bridge.core.update_settings(patch).await;
    Ok(())
}

/// One device's answer to a send: started, or why not (in plain words).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    peer_id: String,
    error: Option<String>,
}

/// Sends the same paths to each device: one transfer each, side by side.
#[tauri::command]
pub async fn portal_send(
    bridge: State<'_, Bridge>,
    peer_ids: Vec<String>,
    paths: Vec<std::path::PathBuf>,
) -> Result<Vec<SendResult>, String> {
    let peers = peer_ids.iter().map(|p| device_id(p)).collect::<Result<Vec<_>, _>>()?;
    let results = bridge.core.send_many(peers, paths).await;
    Ok(results.into_iter().map(|(id, r)| SendResult { peer_id: id.to_string(), error: r.err() }).collect())
}

fn transfer_id(id: &str) -> Result<u64, String> {
    u64::from_str_radix(id, 16).map_err(|_| "unknown transfer".to_owned())
}

/// Name, kind, size and file count of each path, for the send list (None:
/// unreadable, or a link). Counted off the main thread: folders can be big.
#[tauri::command]
pub async fn portal_describe(
    bridge: State<'_, Bridge>,
    paths: Vec<std::path::PathBuf>,
) -> Result<Vec<Option<ItemView>>, String> {
    let skip_junk = bridge.core.snapshot().await.map(|s| s.me.skip_junk).unwrap_or(true);
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .iter()
            .map(|p| {
                portal_core::transfer::plan::describe(p, skip_junk).map(|i| ItemView {
                    name: i.name,
                    folder: i.kind == portal_core::proto::ItemKind::Folder,
                    bytes: i.bytes,
                    files: i.files,
                })
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn portal_accept(bridge: State<'_, Bridge>, transfer_id: String, always: bool) -> Result<(), String> {
    bridge.core.accept(transfer_id_of(&transfer_id)?, always).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_decline(bridge: State<'_, Bridge>, transfer_id: String) -> Result<(), String> {
    bridge.core.decline(transfer_id_of(&transfer_id)?).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_cancel(bridge: State<'_, Bridge>, transfer_id: String) -> Result<(), String> {
    bridge.core.cancel(transfer_id_of(&transfer_id)?).await;
    Ok(())
}

/// Continue pressed on a transfer that stopped partway when the app quit.
#[tauri::command]
pub async fn portal_continue(bridge: State<'_, Bridge>, transfer_id: String) -> Result<(), String> {
    bridge.core.continue_transfer(transfer_id_of(&transfer_id)?).await;
    Ok(())
}

fn transfer_id_of(id: &str) -> Result<u64, String> {
    transfer_id(id)
}

/// The doctor's "Fix it" for Windows Firewall: removes Block rules for this
/// program and puts the Allow rules back, elevated (Windows asks first).
#[tauri::command]
pub async fn portal_fix_firewall(app: AppHandle, bridge: State<'_, Bridge>) -> Result<String, String> {
    #[cfg(windows)]
    {
        use portal_sys::windows::{FirewallFix, fix_firewall};
        let exe =
            std::env::current_exe().map_err(|e| format!("Couldn’t find {} itself ({e}).", app.package_info().name))?;
        let name = app.package_info().name.clone();
        let outcome =
            tauri::async_runtime::spawn_blocking(move || fix_firewall(&exe, &name)).await.map_err(|e| e.to_string())?;
        // The doctor looks at the rules again on its next round.
        bridge.core.retry_discovery().await;
        match outcome {
            FirewallFix::Fixed => Ok(format!("Fixed. Windows Firewall allows {} again.", app.package_info().name)),
            FirewallFix::Declined => Err("Nothing changed: the administrator question was answered No.".into()),
            FirewallFix::Failed(why) => {
                Err(format!("Couldn’t fix it: {why}. Try again, or reinstall {}.", app.package_info().name))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (app, bridge);
        Err("Only Windows has this firewall.".into())
    }
}

/// "Send again" after a send that stopped or was cancelled: the same items.
#[tauri::command]
pub async fn portal_send_again(bridge: State<'_, Bridge>, transfer_id: String) -> Result<(), String> {
    bridge.core.send_again(transfer_id_of(&transfer_id)?).await
}

#[tauri::command]
pub async fn portal_dismiss(bridge: State<'_, Bridge>, transfer_id: String) -> Result<(), String> {
    bridge.core.dismiss(transfer_id_of(&transfer_id)?).await;
    Ok(())
}

/// The update state (Settings → About, the update notice).
#[tauri::command]
pub fn portal_update_status(app: AppHandle) -> Option<crate::updates::UpdateView> {
    use tauri::Manager;
    app.try_state::<crate::updates::Updates>().map(|u| u.view())
}

/// "Check for updates".
#[tauri::command]
pub async fn portal_update_check(app: AppHandle) -> Result<(), String> {
    crate::updates::check(&app).await;
    Ok(())
}

/// "Restart to update" / "Update and restart" (waits for a running transfer).
#[tauri::command]
pub async fn portal_update_restart(app: AppHandle) -> Result<(), String> {
    crate::updates::restart(&app).await;
    Ok(())
}

/// "Later": no notice for this version.
#[tauri::command]
pub fn portal_update_later(app: AppHandle) {
    crate::updates::later(&app);
}

/// Send clipboard (the window's key): to these devices, or (none given)
/// the remembered choice. The answer is a sentence either way; files on the
/// clipboard come back to be offered ("2 files on the clipboard · Send them
/// to Desk PC"), not sent.
#[tauri::command]
pub async fn portal_send_clipboard(
    bridge: State<'_, Bridge>,
    peer_ids: Option<Vec<String>>,
) -> Result<portal_core::clip::ClipSent, String> {
    let to = match peer_ids {
        Some(ids) => Some(ids.iter().map(|p| device_id(p)).collect::<Result<Vec<_>, _>>()?),
        None => None,
    };
    bridge.core.send_clipboard(to).await
}

/// The Shelf's text box: typed or pasted text, sent like a copy (`private`:
/// marked private on the other computer, with no preview there).
#[tauri::command]
pub async fn portal_send_text(
    bridge: State<'_, Bridge>,
    peer_ids: Option<Vec<String>>,
    text: String,
    private: Option<bool>,
) -> Result<String, String> {
    let to = match peer_ids {
        Some(ids) => Some(ids.iter().map(|p| device_id(p)).collect::<Result<Vec<_>, _>>()?),
        None => None,
    };
    bridge.core.send_text(to, text, private.unwrap_or(false)).await
}

/// Agree (or stop agreeing) to sync the clipboard with one device.
#[tauri::command]
pub async fn portal_set_clip_sync(bridge: State<'_, Bridge>, peer_id: String, on: bool) -> Result<(), String> {
    bridge.core.set_clip_sync(&device_id(&peer_id)?, on).await;
    Ok(())
}

/// "Turn off clipboard sync": with every device.
#[tauri::command]
pub async fn portal_clip_sync_off(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.clip_sync_off().await;
    Ok(())
}

/// Pause sync here for `minutes`, or (none) until SeamDrop restarts.
#[tauri::command]
pub async fn portal_pause_clip_sync(bridge: State<'_, Bridge>, minutes: Option<u64>) -> Result<(), String> {
    bridge.core.pause_clip_sync(minutes.map(|m| std::time::Duration::from_secs(m * 60))).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_resume_clip_sync(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.resume_clip_sync().await;
    Ok(())
}

/// Where a device's clipboard items go here: "clipboard" or "shelf".
#[tauri::command]
pub async fn portal_set_clip_to(
    bridge: State<'_, Bridge>,
    peer_id: String,
    to: portal_core::model::ClipTo,
) -> Result<(), String> {
    bridge.core.set_clip_to(&device_id(&peer_id)?, to).await;
    Ok(())
}

/// Copy again, from the received list.
#[tauri::command]
pub async fn portal_copy_clip(bridge: State<'_, Bridge>, id: u64) -> Result<(), String> {
    bridge.core.copy_clip(id).await
}

/// How much of the log file the technical log carries (the newest part)
const TECHNICAL_LOG_MAX: u64 = 512 * 1024;

/// The technical log, for support: the app, its version and the OS, then
/// the newest part of the log file (every line Details showed, with its
/// technical part, and what only the log file has). Never clipboard content:
/// the core never logs it.
fn technical_log(app: &AppHandle) -> String {
    use std::io::{Read, Seek, SeekFrom};
    use tauri::Manager;

    let info = app.package_info();
    let mut out =
        format!("{} {} · {} ({})\n\n", info.name, info.version, portal_sys::host::os_version(), std::env::consts::ARCH);
    let path = app.path().app_log_dir().map(|d| d.join(crate::LOG_FILE));
    let read = path.as_ref().map_err(|e| e.to_string()).and_then(|p| {
        let mut f = std::fs::File::open(p).map_err(|e| e.to_string())?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        f.seek(SeekFrom::Start(len.saturating_sub(TECHNICAL_LOG_MAX))).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        // Cut mid-file: start at the next whole line
        let start =
            if len > TECHNICAL_LOG_MAX { bytes.iter().position(|&b| b == b'\n').map_or(0, |i| i + 1) } else { 0 };
        Ok(String::from_utf8_lossy(&bytes[start..]).into_owned())
    });
    match read {
        Ok(text) => out.push_str(&text),
        Err(e) => out.push_str(&format!("(the log file couldn’t be read: {e})\n")),
    }
    out
}

/// Settings → Received files go to … → Open: the folder itself (created if
/// it isn't there yet). Only ever this folder: the page can't name another.
#[tauri::command]
pub async fn portal_open_received(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let dir = bridge.core.snapshot().await.map_err(|e| e.to_string())?.me.receive_dir;
    let _ = std::fs::create_dir_all(&dir);
    app.opener().open_path(dir, None::<&str>).map_err(|e| e.to_string())
}

/// The done screen's Open (Mission 05 §1.6): only a file a finished
/// transfer offered (the core's `result.open`), checked again here: a
/// document, image or media file, never an app, script or archive.
#[tauri::command]
pub async fn portal_open_file(app: AppHandle, bridge: State<'_, Bridge>, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let offered = bridge
        .core
        .snapshot()
        .await
        .map_err(|e| e.to_string())?
        .transfers
        .iter()
        .any(|t| t.result.as_ref().and_then(|r| r.open.as_deref()) == Some(path.as_str()));
    let p = std::path::Path::new(&path);
    if !offered || !p.is_file() || !portal_core::open::safe_to_open(&path) {
        return Err("That file can’t be opened from here. Use Show in folder.".into());
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

/// Details → Copy technical log. Automatic clipboard sync doesn't carry it.
#[tauri::command]
pub async fn portal_copy_technical_log(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.copy_text(technical_log(&app)).await
}

/// "2026-11-15 19-31 UTC" (the log file's times are UTC too)
fn utc_stamp(t: std::time::SystemTime) -> String {
    let secs = t.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (H. Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}-{:02} UTC", rest / 3600, rest % 3600 / 60)
}

/// Saves the technical log as a file in Downloads ("SeamDrop technical log
/// 2026-11-15 19-31 UTC.txt") and returns its path, for Show in Finder.
#[tauri::command]
pub fn portal_save_technical_log(app: AppHandle) -> Result<String, String> {
    use tauri::Manager;

    let dir = app.path().download_dir().map_err(|e| e.to_string())?;
    let path =
        dir.join(format!("{} technical log {}.txt", app.package_info().name, utc_stamp(std::time::SystemTime::now())));
    std::fs::write(&path, technical_log(&app)).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn portal_clear_clips(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.clear_clips().await;
    Ok(())
}

/// The Shelf (Mission 05 §3.2): Add to Shelf.
/// Settings → This computer → Picture (Mission 05 §2.3): a photo the window
/// already cropped and re-encoded as PNG, a colour and an icon, or None for
/// the default. The core checks it before keeping or sending it.
#[tauri::command]
pub async fn portal_set_picture(
    bridge: State<'_, Bridge>,
    picture: Option<portal_core::picture::Picture>,
) -> Result<(), String> {
    bridge.core.set_picture(picture).await
}

#[tauri::command]
pub async fn portal_shelf_add(
    bridge: State<'_, Bridge>,
    name: String,
    text: String,
    private: bool,
) -> Result<(), String> {
    bridge.core.shelf_add(&name, &text, private).await
}

/// Renames (or edits) a Shelf item.
#[tauri::command]
pub async fn portal_shelf_edit(
    bridge: State<'_, Bridge>,
    id: String,
    name: Option<String>,
    text: Option<String>,
) -> Result<(), String> {
    bridge.core.shelf_edit(&id, name, text).await
}

/// ✕ on items (or Clear Shelf: none named), on every device it's shared with.
#[tauri::command]
pub async fn portal_shelf_remove(bridge: State<'_, Bridge>, ids: Option<Vec<String>>) -> Result<(), String> {
    bridge.core.shelf_remove(ids).await;
    Ok(())
}

#[tauri::command]
pub async fn portal_shelf_undo(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.shelf_undo().await
}

/// Copy: on the clipboard (private items with the private marks).
#[tauri::command]
pub async fn portal_shelf_copy(bridge: State<'_, Bridge>, id: String) -> Result<(), String> {
    bridge.core.shelf_copy(&id).await
}

/// Reveal: a private item's text, for the window to show a few seconds.
#[tauri::command]
pub async fn portal_shelf_reveal(bridge: State<'_, Bridge>, id: String) -> Result<Option<String>, String> {
    Ok(bridge.core.shelf_reveal(&id).await)
}

#[tauri::command]
pub async fn portal_set_shelf_share(bridge: State<'_, Bridge>, peer: String, on: bool) -> Result<(), String> {
    bridge.core.set_shelf_share(&device_id(&peer)?, on).await;
    Ok(())
}

/// Put on Shelf from a "marked private" notice (reads it now, if it's still there).
#[tauri::command]
pub async fn portal_shelf_put_marked(bridge: State<'_, Bridge>, count: u64) -> Result<(), String> {
    bridge.core.shelf_put_marked(count).await
}

#[tauri::command]
pub async fn portal_shelf_put_received(bridge: State<'_, Bridge>, id: u64) -> Result<(), String> {
    bridge.core.shelf_put_received(id).await
}

/// Save as file, for a received image: the path it was saved at.
#[tauri::command]
pub async fn portal_save_clip_image(bridge: State<'_, Bridge>, id: u64, name: String) -> Result<String, String> {
    bridge.core.save_clip_image(id, &name).await
}

#[tauri::command]
pub async fn portal_dismiss_clip_notice(bridge: State<'_, Bridge>) -> Result<(), String> {
    bridge.core.dismiss_clip_notice().await;
    Ok(())
}

/// ✕ on files waiting for a busy device.
#[tauri::command]
pub async fn portal_unqueue_next(bridge: State<'_, Bridge>, id: u64) -> Result<(), String> {
    bridge.core.unqueue_next(id).await;
    Ok(())
}

/// Continue on what waited for a device through a quit.
#[tauri::command]
pub async fn portal_continue_next(bridge: State<'_, Bridge>, peer: String) -> Result<(), String> {
    bridge.core.continue_next(&device_id(&peer)?).await;
    Ok(())
}

/// Takes one item off the Shelf.
#[tauri::command]
pub async fn portal_remove_clip(bridge: State<'_, Bridge>, id: u64) -> Result<(), String> {
    bridge.core.remove_clip(id).await;
    Ok(())
}

/// The Send clipboard shortcut, and why it isn't working if it isn't.
#[tauri::command]
pub fn portal_shortcut(app: AppHandle) -> crate::clipboard::ShortcutView {
    crate::clipboard::view(&app)
}

/// A new shortcut ("" for the default); refused with a sentence if taken.
#[tauri::command]
pub async fn portal_set_shortcut(app: AppHandle, shortcut: String) -> Result<crate::clipboard::ShortcutView, String> {
    crate::clipboard::change(&app, shortcut).await
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn stamps_are_utc_dates_and_times() {
        assert_eq!(super::utc_stamp(UNIX_EPOCH), "1970-01-01 00-00 UTC");
        // 2026-11-15 19:31:42 UTC
        assert_eq!(super::utc_stamp(UNIX_EPOCH + Duration::from_secs(1_794_771_102)), "2026-11-15 19-31 UTC");
        // 2024-02-29 (a leap day) 23:59
        assert_eq!(super::utc_stamp(UNIX_EPOCH + Duration::from_secs(1_709_251_140)), "2024-02-29 23-59 UTC");
    }
}
