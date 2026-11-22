//! Settings and trusted devices, persisted as JSON in the app-data folder.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::model::{Appearance, ClipTo, DeviceId, Fingerprint, Incoming, Os};

const SETTINGS_FILE: &str = "settings.json";
const PEERS_FILE: &str = "peers.json";
/// Last-known addresses kept per peer.
pub const MAX_ADDRS: usize = 8;

/// How received files are put away (Mission 05 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMode {
    /// Straight into the folder: files and folders arrive loose (Downloads).
    #[default]
    Loose,
    /// In a SeamDrop folder (`Downloads/SeamDrop`), optionally one folder
    /// per device inside it.
    AppFolder,
    /// A new numbered folder for each delivery: "SeamDrop 012 – from Desk PC".
    Numbered,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub device_name: String,
    pub receive_dir: PathBuf,
    /// Skip .DS_Store, Thumbs.db and other OS junk (on by default).
    pub skip_junk: bool,
    /// Download updates by themselves and offer a restart (on by default).
    pub auto_update: bool,
    /// "Get test builds" (Mission 05 §8.2): also check the beta feed, off by default.
    pub test_builds: bool,
    /// Mission 03's one switch for the pair. Since Mission 04 sync is per
    /// pair of devices (`TrustedPeer::clip_sync`); a saved `true` moves to
    /// every paired device once, then this stays false.
    pub clipboard_sync: bool,
    pub clipboard_sync_at: u64,
    /// The Send clipboard shortcut, if changed from the default.
    pub clipboard_shortcut: Option<String>,
    /// Where files and the clipboard go: the devices ticked last time (the
    /// shortcut and the tray use it). Empty until a choice is made.
    pub send_to: Vec<DeviceId>,
    /// System, Light or Dark (Settings and the tray)
    pub appearance: Appearance,
    /// "A folder for each device": `SeamDrop/From MacBook Pro/…` (off by default)
    pub folder_per_device: bool,
    /// Files have arrived here before (the first done screen says where
    /// they go, with Change folder).
    pub received_before: bool,
    /// How received files are put away; None in settings saved before
    /// Mission 05 (they're moved over once, `Store::open`).
    pub receive_mode: Option<ReceiveMode>,
    /// Keep the sender's file and folder dates (off: dated when they arrive,
    /// so they're the newest in the folder).
    pub keep_dates: bool,
    /// What changed about received files with the update, said once.
    pub receive_note: Option<String>,
    /// This computer's picture, shared with paired devices (Mission 05 §2.3);
    /// None: a colour from the device id and the OS's icon.
    #[serde(default)]
    pub picture: Option<crate::picture::Picture>,
}

impl Default for Settings {
    fn default() -> Self {
        let fallback_name = match Os::current() {
            Os::Macos => "This Mac",
            Os::Windows => "This PC",
            _ => "This computer",
        };
        Self {
            device_name: portal_sys::host::computer_name().unwrap_or_else(|| fallback_name.into()),
            // New installs: straight into Downloads (Mission 05 §4)
            receive_dir: downloads_dir(),
            skip_junk: true,
            auto_update: true,
            test_builds: false,
            clipboard_sync: false,
            clipboard_sync_at: 0,
            clipboard_shortcut: None,
            send_to: Vec::new(),
            appearance: Appearance::System,
            folder_per_device: false,
            received_before: false,
            receive_mode: Some(ReceiveMode::Loose),
            keep_dates: false,
            receive_note: None,
            picture: None,
        }
    }
}

/// `Downloads/SeamDrop`: the default before Mission 05, and the "In a
/// SeamDrop folder" choice's folder.
pub fn default_receive_dir() -> PathBuf {
    downloads_dir().join(crate::consts::APP_NAME)
}

pub fn downloads_dir() -> PathBuf {
    dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads"))).unwrap_or_else(std::env::temp_dir)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    /// Pinned certificate fingerprint.
    pub fingerprint: Fingerprint,
    /// Last-known socket addresses, most recent first.
    pub addrs: Vec<String>,
    /// False once the other side revoked us; kept so the UI can say so.
    pub trusted: bool,
    /// "Files from <device>": ask every time (default), always accept, never.
    #[serde(default)]
    pub incoming: Incoming,
    /// Unix seconds.
    pub paired_at: u64,
    /// The user renamed it here, so the peer's own name changes don't apply.
    #[serde(default)]
    pub renamed: bool,
    /// The user pressed Disconnect: stay paired, but don't connect (either
    /// way) until they press Connect.
    #[serde(default)]
    pub disconnected: bool,
    /// This computer agreed to sync the clipboard automatically with it
    /// (it runs only when that device agreed too), and when that last
    /// changed (unix ms; the newest change wins on both).
    #[serde(default)]
    pub clip_sync: bool,
    #[serde(default)]
    pub clip_sync_at: u64,
    /// Where its clipboard items go here.
    #[serde(default)]
    pub clip_to: ClipTo,
    /// This computer agreed to share the Shelf with it (Mission 05 §3.2), and
    /// when that last changed (unix ms). Shared only while both agree.
    #[serde(default)]
    pub shelf_share: bool,
    #[serde(default)]
    pub shelf_share_at: u64,
    /// The person here answered "Share the Shelf with it?" (asked once: at
    /// pairing, or after the update for pairings made before).
    #[serde(default)]
    pub shelf_answered: bool,
    /// Its folder when "A folder for each device" is on ("From MacBook Pro"),
    /// chosen the first time and kept, whatever it's renamed to later.
    #[serde(default)]
    pub folder: Option<String>,
}

#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    settings: Settings,
    peers: Vec<TrustedPeer>,
}

impl Store {
    pub fn open(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let mut settings: Option<serde_json::Value> = read_json(&dir.join(SETTINGS_FILE));
        let mut peers: Option<serde_json::Value> = read_json(&dir.join(PEERS_FILE));
        // Saved before Mission 05 §4: no way of receiving recorded yet (a
        // missing field would otherwise take the new-install default).
        let before_05 = settings.as_ref().is_some_and(|v| v.get("receive_mode").is_none());
        let (settings_changed, peers_changed) = migrate_mission_01(settings.as_mut(), peers.as_mut());
        let mut store = Self {
            dir: dir.to_owned(),
            settings: settings.and_then(|v| from_value(v, SETTINGS_FILE)).unwrap_or_default(),
            peers: peers.and_then(|v| from_value(v, PEERS_FILE)).unwrap_or_default(),
        };
        // Mission 03 → 04: clipboard sync was one switch for the pair; it's
        // per pair of devices now (the other device still has to agree).
        let (settings_changed, peers_changed) = if store.settings.clipboard_sync {
            let at = store.settings.clipboard_sync_at;
            for p in store.peers.iter_mut().filter(|p| p.trusted) {
                p.clip_sync = true;
                p.clip_sync_at = at;
            }
            store.settings.clipboard_sync = false;
            (true, true)
        } else {
            (settings_changed, peers_changed)
        };
        // Mission 04 → 05: received files have three ways to go (§4). The old
        // default folder becomes "straight into Downloads"; a folder someone
        // chose stays, loose; "a folder for each device" becomes the SeamDrop
        // folder with a folder per device. Said once if anything visible changed.
        let settings_changed = if before_05 || store.settings.receive_mode.is_none() {
            let s = &mut store.settings;
            if s.folder_per_device {
                s.receive_mode = Some(ReceiveMode::AppFolder);
            } else if s.receive_dir == default_receive_dir() {
                s.receive_mode = Some(ReceiveMode::Loose);
                s.receive_dir = downloads_dir();
                s.receive_note = Some(
                    "Received files now go straight into Downloads, so new ones are easy to find. Settings → Received files has the other choices.".into(),
                );
            } else {
                s.receive_mode = Some(ReceiveMode::Loose);
            }
            true
        } else {
            settings_changed
        };
        // Saving the migration is best effort: the app starts with it either way
        if settings_changed && let Err(e) = write_json(&dir.join(SETTINGS_FILE), &store.settings) {
            tracing::warn!("couldn't save migrated settings: {e}");
        }
        if peers_changed && let Err(e) = store.save_peers() {
            tracing::warn!("couldn't save migrated devices: {e}");
        }
        Ok(store)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn update_settings(&mut self, f: impl FnOnce(&mut Settings)) -> Result<()> {
        f(&mut self.settings);
        write_json(&self.dir.join(SETTINGS_FILE), &self.settings)
    }

    pub fn peers(&self) -> &[TrustedPeer] {
        &self.peers
    }

    pub fn peer(&self, id: &DeviceId) -> Option<&TrustedPeer> {
        self.peers.iter().find(|p| &p.id == id)
    }

    pub fn upsert_peer(&mut self, peer: TrustedPeer) -> Result<()> {
        match self.peers.iter_mut().find(|p| p.id == peer.id) {
            Some(existing) => *existing = peer,
            None => self.peers.push(peer),
        }
        self.save_peers()
    }

    /// Applies `f` to a known peer and saves. Returns false if it's unknown.
    pub fn update_peer(&mut self, id: &DeviceId, f: impl FnOnce(&mut TrustedPeer)) -> Result<bool> {
        let Some(peer) = self.peers.iter_mut().find(|p| &p.id == id) else {
            return Ok(false);
        };
        f(peer);
        self.save_peers()?;
        Ok(true)
    }

    pub fn rename_peer(&mut self, id: &DeviceId, name: &str) -> Result<bool> {
        let name = name.trim().to_owned();
        self.update_peer(id, |p| {
            p.name = name;
            p.renamed = true;
        })
    }

    pub fn set_incoming(&mut self, id: &DeviceId, value: Incoming) -> Result<bool> {
        self.update_peer(id, |p| p.incoming = value)
    }

    pub fn set_disconnected(&mut self, id: &DeviceId, value: bool) -> Result<bool> {
        self.update_peer(id, |p| p.disconnected = value)
    }

    pub fn set_clip_sync(&mut self, id: &DeviceId, on: bool, at: u64) -> Result<bool> {
        self.update_peer(id, |p| {
            p.clip_sync = on;
            p.clip_sync_at = at;
        })
    }

    pub fn set_shelf_share(&mut self, id: &DeviceId, on: bool, at: u64) -> Result<bool> {
        self.update_peer(id, |p| {
            p.shelf_share = on;
            p.shelf_share_at = at;
            p.shelf_answered = true;
        })
    }

    pub fn set_clip_to(&mut self, id: &DeviceId, to: ClipTo) -> Result<bool> {
        self.update_peer(id, |p| p.clip_to = to)
    }

    /// Records a working address (most recent first, deduplicated, capped).
    pub fn remember_addr(&mut self, id: &DeviceId, addr: &str) -> Result<bool> {
        if self.peer(id).is_some_and(|p| p.addrs.first().map(String::as_str) == Some(addr)) {
            return Ok(true);
        }
        let addr = addr.to_owned();
        self.update_peer(id, |p| {
            p.addrs.retain(|a| a != &addr);
            p.addrs.insert(0, addr);
            p.addrs.truncate(MAX_ADDRS);
        })
    }

    pub fn remove_peer(&mut self, id: &DeviceId) -> Result<bool> {
        let before = self.peers.len();
        self.peers.retain(|p| &p.id != id);
        let removed = self.peers.len() != before;
        if removed {
            self.save_peers()?;
        }
        Ok(removed)
    }

    /// Also writes `always_accept` (Always accept or not) for each device:
    /// Mission 01 builds need the field, so going back keeps the pairings.
    fn save_peers(&self) -> Result<()> {
        let mut value = serde_json::to_value(&self.peers)?;
        for (v, p) in value.as_array_mut().into_iter().flatten().zip(&self.peers) {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("always_accept".into(), (p.incoming == Incoming::Always).into());
            }
        }
        write_json(&self.dir.join(PEERS_FILE), &value)
    }
}

/// Mission 01 → 02, on the saved JSON: "Always accept" becomes Always
/// accept (only for devices saved before `incoming` existed); a computer set
/// to Send never received, so it now refuses files from every device
/// (Receive only limited sending, which needs no setting). The old role is
/// dropped, so this happens once. Returns which file changed.
fn migrate_mission_01(settings: Option<&mut serde_json::Value>, peers: Option<&mut serde_json::Value>) -> (bool, bool) {
    let role = settings.and_then(|s| s.as_object_mut()).and_then(|s| s.remove("role"));
    let refuse_all = role.as_ref().and_then(|r| r.as_str()) == Some("send");
    let mut peers_changed = false;
    for p in peers.and_then(|p| p.as_array_mut()).into_iter().flatten() {
        let Some(p) = p.as_object_mut() else { continue };
        if !p.contains_key("incoming") {
            let always = p.get("always_accept") == Some(&serde_json::Value::Bool(true));
            p.insert("incoming".into(), if always { "always" } else { "ask" }.into());
            peers_changed = true;
        }
        if refuse_all {
            p.insert("incoming".into(), "never".into());
            peers_changed = true;
        }
    }
    (role.is_some(), peers_changed)
}

fn from_value<T: for<'de> Deserialize<'de>>(value: serde_json::Value, file: &str) -> Option<T> {
    serde_json::from_value(value).map_err(|e| tracing::warn!("ignoring unreadable {file}: {e}")).ok()
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::warn!("ignoring unreadable {}: {e}", path.display());
            None
        }
    }
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    portal_sys::fs::write_private_file(path, &serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
