//! Moving files: sender and receiver pipelines (brief §6.4).
//!
//! Disk reads, hashing and writes run on their own threads; bounded channels
//! give backpressure all the way from the receiver's disk to the sender's.
//! Files never pass through memory whole: 1 MiB at a time.

pub mod estimate;
pub mod journal;
pub mod plan;
pub mod recv;
pub mod send;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Bytes per data frame.
pub const CHUNK: usize = 1024 * 1024;
/// No message or data from the other side for this long pauses the transfer.
pub const STALL: Duration = Duration::from_secs(60);
/// The receiver saves what has arrived this often (and when it pauses).
pub const CHECKPOINT_EVERY: Duration = Duration::from_secs(1);
/// Receiver progress reports (also the sender's heartbeat).
pub const REPORT_EVERY: Duration = Duration::from_millis(250);
/// Free-space margin on top of the transfer's size.
pub const SPACE_MARGIN: u64 = 64 * 1024 * 1024;
/// How many skipped or renamed items a done screen and Details name.
pub const LISTED: usize = 50;

/// Why a transfer ended without delivering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The person on this side pressed Cancel.
    CancelledHere,
    /// The other side cancelled (or reported an error with this code).
    CancelledThere {
        code: Option<String>,
        msg: String,
    },
    /// The connection dropped or went silent: the transfer pauses.
    Lost,
    DiskFull,
    /// Writing was refused (permissions, Controlled Folder Access).
    AccessDenied(String),
    /// A file's BLAKE3 didn't match.
    Corrupt(String),
    /// A file on the sender couldn't be read or changed while sending.
    ReadFailed(String),
    /// The other side broke the protocol.
    Protocol(String),
    Io(String),
}

impl Failure {
    /// Code sent in `cancel { why: error }` so the other side can explain it.
    pub fn code(&self) -> &'static str {
        match self {
            Failure::CancelledHere | Failure::CancelledThere { .. } => "cancelled",
            Failure::Lost => "lost",
            Failure::DiskFull => "disk_full",
            Failure::AccessDenied(_) => "access_denied",
            Failure::Corrupt(_) => "corrupt",
            Failure::ReadFailed(_) => "read_failed",
            Failure::Protocol(_) => "protocol",
            Failure::Io(_) => "io",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Failure::AccessDenied(s)
            | Failure::Corrupt(s)
            | Failure::ReadFailed(s)
            | Failure::Protocol(s)
            | Failure::Io(s) => s.clone(),
            Failure::CancelledThere { msg, .. } => msg.clone(),
            _ => String::new(),
        }
    }

    pub(crate) fn from_io(e: &std::io::Error, what: &str) -> Failure {
        if is_disk_full(e) {
            return Failure::DiskFull;
        }
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            return Failure::AccessDenied(what.to_owned());
        }
        Failure::Io(format!("{what}: {e}"))
    }
}

pub fn is_disk_full(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::StorageFull || e.kind() == std::io::ErrorKind::QuotaExceeded {
        return true;
    }
    // ENOSPC / EDQUOT (unix), ERROR_DISK_FULL / ERROR_HANDLE_DISK_FULL (Windows)
    matches!(e.raw_os_error(), Some(code) if (cfg!(unix) && (code == 28 || code == 69 || code == 122)) || (cfg!(windows) && (code == 112 || code == 39)))
}

/// Shared between a transfer task and the node, which reads it for snapshots
/// and throttled progress events.
#[derive(Debug, Default)]
pub struct ProgressState {
    pub bytes: AtomicU64,
    /// Sender: bytes the receiver says it has (its `progress` reports), what
    /// speed and time left are measured from. Bytes merely queued for the
    /// network (up to 8 MiB) don't count as moved.
    pub acked: AtomicU64,
    pub files_done: AtomicU64,
    /// Receiver: this run has heard from the sender since it started (after
    /// that, the `resume_from` that started it is out of date).
    pub started: std::sync::atomic::AtomicBool,
    pub current: Mutex<String>,
    /// Sentences for Details and the done screen (a file that changed while
    /// paused, say), collected by the node.
    notes: Mutex<Vec<String>>,
}

impl ProgressState {
    pub fn add(&self, n: u64) {
        self.bytes.fetch_add(n, Ordering::Relaxed);
    }

    pub fn set(&self, n: u64) {
        self.bytes.store(n, Ordering::Relaxed);
    }

    pub fn note(&self, text: String) {
        self.notes.lock().unwrap_or_else(|e| e.into_inner()).push(text);
    }

    pub fn take_notes(&self) -> Vec<String> {
        std::mem::take(&mut *self.notes.lock().unwrap_or_else(|e| e.into_inner()))
    }

    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    pub fn acked(&self) -> u64 {
        self.acked.load(Ordering::Relaxed)
    }

    pub fn set_acked(&self, n: u64) {
        self.acked.store(n, Ordering::Relaxed);
    }

    pub fn set_current(&self, name: &str) {
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = name.to_owned();
    }

    pub fn current(&self) -> String {
        self.current.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

pub type Progress = Arc<ProgressState>;

/// Resolves once `cancel` turns true (holds no lock across awaits).
pub(crate) async fn cancelled(cancel: &mut tokio::sync::watch::Receiver<bool>) {
    let _ = cancel.wait_for(|c| *c).await;
}
