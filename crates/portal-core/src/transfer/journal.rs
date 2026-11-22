//! Paused transfers survive a restart: each transfer's state is a JSON file
//! in `<app data>/transfers/`, replaced atomically.
//!
//! - The sender keeps what it was sending (the file list as planned, with each
//!   file's size and modified time, so a changed file is noticed).
//! - The receiver keeps what has arrived and checks out: folder entries, the
//!   files that are done, and the next file's bytes with their BLAKE3, written
//!   by the writer thread so it never claims bytes that aren't on disk.
//! - A transfer that ended leaves a short note (a tombstone) for a day, so a
//!   computer that comes back asking about it hears "cancelled" or "done",
//!   not "unknown".

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::fsafe::{Namer, Target};
use crate::model::DeviceId;
use crate::proto::{B32, DirEntry, Mtime, Offer, SkipNote, Summary, TransferId};
use crate::transfer::plan::PlannedFile;

/// How long a paused transfer waits for the other computer to come back.
pub const RESUME_FOR: Duration = Duration::from_secs(24 * 60 * 60);

/// Why a transfer stopped (Mission 05 §1.4), written when it stops, before
/// the app exits. Quit and crash ask on that computer at the next launch;
/// network and update continue by themselves. A saved state with none and
/// no pause stamp is a crash (it was running and nothing stopped it cleanly).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stopped {
    Quit,
    Crash,
    Network,
    Update,
}

impl Stopped {
    /// The person decides: Continue or Cancel.
    pub fn asks(self) -> bool {
        matches!(self, Stopped::Quit | Stopped::Crash)
    }

    /// From a saved state: its reason, or crash if it was still running.
    pub fn of(stopped: Option<Stopped>, paused_at: Option<u64>) -> Stopped {
        match (stopped, paused_at) {
            (Some(s), _) => s,
            (None, None) => Stopped::Crash,
            // Saved by an older build: it paused on a drop or a quit, and
            // those always continued by themselves.
            (None, Some(_)) => Stopped::Network,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutState {
    pub tid: TransferId,
    pub peer: DeviceId,
    pub peer_name: String,
    /// What was dropped (to send again, the same way).
    pub paths: Vec<PathBuf>,
    pub offer: Offer,
    pub dirs: Vec<DirEntry>,
    pub files: Vec<PlannedFile>,
    pub skipped: Vec<SkipNote>,
    /// Bytes sent when it paused (shown until the receiver says).
    pub bytes: u64,
    /// Unix seconds; None while it runs.
    pub paused_at: Option<u64>,
    /// Why it stopped; None while it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<Stopped>,
}

/// The file being received when it paused.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Partial {
    pub idx: u64,
    /// The sender's path, and where it goes in the staging folder.
    pub wire: Vec<String>,
    pub local: Vec<String>,
    pub size: u64,
    pub mtime: Option<Mtime>,
    /// Bytes on disk that the writer had written and hashed.
    pub written: u64,
    /// BLAKE3 of those bytes.
    pub hash: B32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InState {
    pub tid: TransferId,
    pub peer: DeviceId,
    pub peer_name: String,
    pub offer: Offer,
    pub auto_accepted: bool,
    pub receive_dir: PathBuf,
    /// `<receive>/.seamdrop-incoming/<transfer>`
    pub staging: PathBuf,
    pub target: Target,
    pub namer: Namer,
    pub dir_times: Vec<(Vec<String>, Mtime)>,
    /// Folder entries received.
    pub dirs: u64,
    /// Files received, checked and closed (in order).
    pub done: u64,
    /// Bytes of those files.
    pub bytes_done: u64,
    pub partial: Option<Partial>,
    pub paused_at: Option<u64>,
    /// Why it stopped; None while it runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<Stopped>,
    /// Keep the sender's file and folder dates (Mission 05 §4; off: dated
    /// when they arrive).
    #[serde(default)]
    pub keep_dates: bool,
}

impl InState {
    /// Bytes that have arrived and are kept.
    pub fn bytes(&self) -> u64 {
        self.bytes_done + self.partial.as_ref().map_or(0, |p| p.written)
    }

    /// Where the partial file can pick up from (0: from the start).
    pub fn offset(&self) -> u64 {
        self.partial.as_ref().map_or(0, |p| p.written)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "how", rename_all = "snake_case")]
pub enum Ending {
    /// The receiver finished: its report, in case the sender missed it.
    Done {
        summary: Summary,
    },
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ended {
    pub tid: TransferId,
    pub peer: DeviceId,
    pub ending: Ending,
    pub at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "side", rename_all = "snake_case")]
pub enum Saved {
    Out(OutState),
    In(Box<InState>),
    Ended(Ended),
}

impl Saved {
    pub fn tid(&self) -> TransferId {
        match self {
            Saved::Out(s) => s.tid,
            Saved::In(s) => s.tid,
            Saved::Ended(e) => e.tid,
        }
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[derive(Clone, Debug)]
pub struct Journal {
    dir: PathBuf,
}

impl Journal {
    pub fn new(data_dir: &Path) -> Self {
        Self { dir: data_dir.join("transfers") }
    }

    pub fn path(&self, tid: TransferId) -> PathBuf {
        self.dir.join(format!("{tid:016x}.json"))
    }

    pub fn save(&self, saved: &Saved) -> std::io::Result<()> {
        write_atomic(&self.path(saved.tid()), saved)
    }

    pub fn load(&self, tid: TransferId) -> Option<Saved> {
        read(&self.path(tid))
    }

    pub fn remove(&self, tid: TransferId) {
        let _ = std::fs::remove_file(self.path(tid));
    }

    /// Replaces a transfer's state with a note of how it ended.
    pub fn end(&self, tid: TransferId, peer: &DeviceId, ending: Ending) {
        let ended = Saved::Ended(Ended { tid, peer: peer.clone(), ending, at: now_secs() });
        if let Err(e) = self.save(&ended) {
            tracing::warn!("couldn't note how transfer {tid:016x} ended: {e}");
        }
    }

    /// How a transfer that's no longer here ended, if it's remembered.
    pub fn ending(&self, tid: TransferId) -> Option<Ending> {
        match self.load(tid)? {
            Saved::Ended(e) => Some(e.ending),
            _ => None,
        }
    }

    /// Stamps a transfer as paused (and the sender's progress), and why.
    /// The first reason stays: a transfer paused by a drop and then quit
    /// still continues by itself.
    pub fn set_paused(&self, tid: TransferId, at: u64, bytes: u64, why: Stopped) {
        let Some(mut saved) = self.load(tid) else { return };
        match &mut saved {
            Saved::Out(s) => {
                s.paused_at = Some(at);
                s.bytes = bytes;
                s.stopped = Some(s.stopped.unwrap_or(why));
            }
            Saved::In(s) => {
                s.paused_at = Some(at);
                s.stopped = Some(s.stopped.unwrap_or(why));
            }
            Saved::Ended(_) => return,
        }
        if let Err(e) = self.save(&saved) {
            tracing::warn!("couldn't save the paused transfer: {e}");
        }
    }

    /// A paused transfer is running again: the pause no longer counts
    /// towards the time limit.
    pub fn set_running(&self, tid: TransferId) {
        let Some(mut saved) = self.load(tid) else { return };
        match &mut saved {
            Saved::Out(s) if s.paused_at.is_some() || s.stopped.is_some() => {
                s.paused_at = None;
                s.stopped = None;
            }
            Saved::In(s) if s.paused_at.is_some() || s.stopped.is_some() => {
                s.paused_at = None;
                s.stopped = None;
            }
            _ => return,
        }
        if let Err(e) = self.save(&saved) {
            tracing::warn!("couldn't save the running transfer: {e}");
        }
    }

    /// Every saved transfer with its file's modified time (unix seconds).
    /// Unreadable files are removed.
    pub fn load_all(&self) -> Vec<(Saved, u64)> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let modified = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            match read(&path) {
                Some(saved) => out.push((saved, modified)),
                None => {
                    tracing::warn!("removing unreadable transfer state {}", path.display());
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        out
    }
}

fn read(path: &Path) -> Option<Saved> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Writes next to the target, then renames over it: a crash never leaves
/// half a file.
pub fn write_atomic(path: &Path, saved: &Saved) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    portal_sys::fs::write_private_file(&tmp, &serde_json::to_vec(saved).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}
