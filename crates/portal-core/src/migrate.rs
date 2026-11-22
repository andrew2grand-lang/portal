//! The rename: the app was called Portal (`com.project1000.portal`). On the
//! first start of SeamDrop, its settings, paired devices and keys are copied
//! from the old app-data folder, so nobody has to pair again. The old folder
//! is never changed or deleted.

use std::path::{Path, PathBuf};

use crate::Result;
use crate::identity::{IDENTITY_FILE, KEY_FILE};

const SETTINGS_FILE: &str = "settings.json";
const PEERS_FILE: &str = "peers.json";

/// What was brought over, for the Details log.
#[derive(Debug, PartialEq)]
pub struct Migrated {
    pub from: PathBuf,
    /// Paired devices that came over.
    pub devices: usize,
    /// The old default receive folder (`Downloads/Portal`) was in use, so
    /// new files now go to `Downloads/SeamDrop`: (before, after).
    pub receive_dir: Option<(PathBuf, PathBuf)>,
}

/// The old app's default receive folder.
pub fn legacy_receive_dir() -> PathBuf {
    crate::store::downloads_dir().join("Portal")
}

/// Copies the old app's files into `new`, once: only when `new` has no
/// identity yet and `old` has one. The identity file is copied last, so a
/// copy that fails halfway is tried again on the next start.
pub fn migrate(old: &Path, new: &Path) -> Result<Option<Migrated>> {
    if new.join(IDENTITY_FILE).exists() || !old.join(IDENTITY_FILE).is_file() {
        return Ok(None);
    }
    std::fs::create_dir_all(new)?;
    let mut receive_dir = None;
    for name in [SETTINGS_FILE, PEERS_FILE, KEY_FILE, IDENTITY_FILE] {
        let Ok(mut bytes) = std::fs::read(old.join(name)) else { continue };
        if name == SETTINGS_FILE
            && let Some((updated, change)) = move_default_receive_dir(&bytes)
        {
            bytes = updated;
            receive_dir = Some(change);
        }
        portal_sys::fs::write_private_file(&new.join(name), &bytes)?;
    }
    let devices = std::fs::read(new.join(PEERS_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.as_array().map(|a| a.iter().filter(|p| p["trusted"] == true).count()))
        .unwrap_or(0);
    Ok(Some(Migrated { from: old.to_owned(), devices, receive_dir }))
}

/// Settings that still use the old default receive folder point at the new
/// default instead. Files already received stay where they are.
fn move_default_receive_dir(settings: &[u8]) -> Option<(Vec<u8>, (PathBuf, PathBuf))> {
    let mut value: serde_json::Value = serde_json::from_slice(settings).ok()?;
    let current = PathBuf::from(value.get("receive_dir")?.as_str()?);
    if current != legacy_receive_dir() {
        return None;
    }
    let now = crate::store::default_receive_dir();
    value["receive_dir"] = serde_json::json!(now);
    Some((serde_json::to_vec_pretty(&value).ok()?, (current, now)))
}
