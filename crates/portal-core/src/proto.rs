//! SeamDrop's wire protocol: length-prefixed frames over one TLS stream.
//!
//! `[kind: u8][len: u32 big-endian][payload]`
//! - kind 1: control message, JSON (≤ 4 MiB)
//! - kind 2: file data for the file currently being streamed (≤ 1 MiB)
//!
//! Every connection starts with both sides sending `hello`. A peer on another
//! protocol version gets `error{incompatible_version}` and is disconnected.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub use crate::consts::PROTOCOL_VERSION;
use crate::model::{DeviceId, Incoming, Os, Role};
use crate::util::{from_hex, to_hex};
use crate::{Error, Result};

pub const MAX_CONTROL_LEN: u32 = 4 * 1024 * 1024;
pub const MAX_DATA_LEN: u32 = 1024 * 1024;
const KIND_CONTROL: u8 = 1;
const KIND_DATA: u8 = 2;

pub type TransferId = u64;

#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    Control(Msg),
    Data(Vec<u8>),
}

/// 32 bytes, hex on the wire (nonces, commitments, BLAKE3 digests).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct B32(pub [u8; 32]);

impl Serialize for B32 {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&to_hex(&self.0))
    }
}

impl<'de> Deserialize<'de> for B32 {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let bytes =
            from_hex(&s).filter(|b| b.len() == 32).ok_or_else(|| serde::de::Error::custom("expected 32 hex bytes"))?;
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(B32(out))
    }
}

impl std::fmt::Debug for B32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "B32({}…)", &to_hex(&self.0)[..8])
    }
}

/// A SPAKE2 share: an uncompressed SEC1 point, 65 bytes, hex on the wire.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Share(pub [u8; crate::spake2::SHARE_LEN]);

impl Serialize for Share {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&to_hex(&self.0))
    }
}

impl<'de> Deserialize<'de> for Share {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        let bytes = from_hex(&s)
            .filter(|b| b.len() == crate::spake2::SHARE_LEN)
            .ok_or_else(|| serde::de::Error::custom("expected 65 hex bytes"))?;
        let mut out = [0u8; crate::spake2::SHARE_LEN];
        out.copy_from_slice(&bytes);
        Ok(Share(out))
    }
}

impl std::fmt::Debug for Share {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Share({}…)", &to_hex(&self.0)[..8])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub proto: u32,
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    /// Always Both since Mission 02 (Mission 01 builds require the field).
    pub role: Role,
    /// App version, for the log and "Update SeamDrop" messages.
    pub app: String,
    /// The sender's listen port, so the other side can dial back.
    pub port: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    IncompatibleVersion,
    /// We don't (or no longer) trust you: pair again.
    NotPaired,
    /// You presented a different key than the one pinned for your id.
    KeyChanged,
    /// Too many pairing attempts.
    RateLimited,
    /// Malformed or out-of-order message.
    Protocol,
    /// Still paired, but the person on this side pressed Disconnect.
    Disconnected,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    File,
    Folder,
}

/// One top-level item of an offer (what the receiver's prompt lists).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OfferItem {
    pub name: String,
    pub kind: ItemKind,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Symlink,
    Junk,
    Unreadable,
    /// Sockets, devices, FIFOs.
    Special,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkipNote {
    pub path: Vec<String>,
    pub reason: SkipReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Offer {
    pub tid: TransferId,
    pub items: Vec<OfferItem>,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    /// What the sender left out (symlinks, junk), so both sides can report it.
    pub skipped: Vec<SkipNote>,
    /// Always Both since Mission 02 (Mission 01 builds require the field).
    pub sender_role: Role,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum DeclineReason {
    /// The person said no.
    User,
    /// The receiver doesn't accept files from this sender ("Files from
    /// <device>: Never"). Named role_send on the wire: Mission 01 builds
    /// read it as "set to Send" and show their own blocked screen.
    #[serde(rename = "role_send")]
    NotAccepting,
    /// Another transfer is running on this link.
    Busy,
    NoSpace {
        needed: u64,
        available: u64,
    },
    NotPaired,
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum CancelReason {
    User,
    Error { code: String, msg: String },
}

/// File modified time: seconds and nanoseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mtime {
    pub s: i64,
    pub ns: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirEntry {
    pub path: Vec<String>,
    pub mtime: Option<Mtime>,
}

/// Receiver's end-of-transfer report.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub files: u64,
    pub bytes: u64,
    /// Names changed to be safe on the receiver's OS.
    pub renamed: u64,
    pub skipped: u64,
    /// Paths over 260 characters (Windows).
    pub long_paths: u64,
    /// The folder they were saved in, for the sender's done screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_in: Option<String>,
    /// Which names changed (the first 50), so the sender can say which.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub renames: Vec<RenameNote>,
}

/// A name the receiver changed: the path as sent, as saved, and why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenameNote {
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Msg {
    Hello(Hello),
    Error {
        code: ErrorCode,
        msg: String,
    },
    Ping,
    Pong,
    Bye,
    /// "Keep the other link to me; this one is closing."
    Superseded,
    Status {
        /// For Mission 01 builds: Send when we refuse their files, else Both.
        role: Role,
        name: String,
        /// What we do with files from the device we're telling. Absent from
        /// Mission 01 builds.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accepts: Option<Incoming>,
    },
    /// Sent once after the hello exchange: do I trust you (are we paired)?
    Welcome {
        paired: bool,
    },

    /* Pairing by typed code (Mission 04 §2; `docs/PROTOCOL.md`). The asker
    is the computer where the person types; the shower shows the code. */
    /// Asker → shower: "pair with me?" (the shower asks its person first)
    PairAsk,
    /// Shower → asker: the code is on screen; type it
    PairReady,
    /// A SPAKE2 share: the asker's with each try, then the shower's answer
    PairKey {
        share: Share,
    },
    /// A key confirmation (HMAC-SHA256 over the transcript): the asker's,
    /// then the shower's
    PairCheck {
        mac: B32,
    },
    /// Shower → asker: that wasn't the code; `tries_left` more in this pairing
    PairWrong {
        tries_left: u32,
    },
    /// Asker → shower: the shower's confirmation checked out too
    PairDone,
    /// Either side: this pairing is over (cancelled, timed out, too many
    /// tries). Says nothing more.
    PairStop,

    Offer(Offer),
    Accept {
        tid: TransferId,
    },
    Decline {
        tid: TransferId,
        reason: DeclineReason,
    },
    Dirs {
        tid: TransferId,
        entries: Vec<DirEntry>,
    },
    File {
        tid: TransferId,
        idx: u64,
        path: Vec<String>,
        size: u64,
        mtime: Option<Mtime>,
        /// The data that follows starts at this byte (resuming a file).
        #[serde(default, skip_serializing_if = "is_zero")]
        from: u64,
    },
    FileEnd {
        tid: TransferId,
        idx: u64,
        blake3: B32,
    },
    Done {
        tid: TransferId,
    },
    Complete {
        tid: TransferId,
        summary: Summary,
    },
    /// Receiver → sender: bytes received so far. Also sender → receiver
    /// while it re-reads the part the receiver already has (which can take a
    /// while), so neither side takes the quiet for a stall; the receiver
    /// answers with its own.
    Progress {
        tid: TransferId,
        bytes: u64,
    },
    Cancel {
        tid: TransferId,
        reason: CancelReason,
    },
    /// Clipboard contents, in parts (`clip::parts`), on the control channel.
    /// `auto`: sent by automatic sync (no notification on arrival).
    Clip {
        id: u64,
        n: u32,
        of: u32,
        auto: bool,
        data: String,
        /// Sent as private on purpose (the Shelf's text box): the receiver
        /// marks it so clipboard history and managers leave it, and shows no
        /// preview of it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        private: bool,
    },
    /// Automatic clipboard sync turned on or off; the newest change (`at`,
    /// unix ms) wins on both computers.
    ClipSync {
        on: bool,
        at: u64,
    },
    /// Sender → receiver when a trusted link comes back: "I still have this
    /// paused transfer for you."
    ResumeAsk {
        tid: TransferId,
    },
    /// Protocol 4 (Mission 05 §1.4): either side of a paused transfer. `on`:
    /// it stopped partway here (the app was quit or crashed) and waits for
    /// the person here to continue; the other side shows "Paused: waiting
    /// for <device> to continue" and doesn't ask. `on: false`: they pressed
    /// Continue, so resume now. Not transfer traffic: it always reaches the
    /// node, even while another transfer runs on the link.
    Hold {
        tid: TransferId,
        on: bool,
    },
    /// Protocol 4 (Mission 05 §1.5): when the earliest of what this computer
    /// has waiting for you was queued (unix ms), or None. When the transfer
    /// between you ends, the one queued earliest goes first (the lower device
    /// id on a tie).
    QueueHead {
        at: Option<u64>,
    },
    /// Protocol 4 (Mission 05 §3.2): this computer agrees (or stops agreeing)
    /// to share the Shelf with you, as of `at` (ms; the newest change wins).
    /// Shared only while both agree.
    ShelfShare {
        on: bool,
        at: u64,
    },
    /// Protocol 4: Shelf records whose origin is the sender or you (never a
    /// third device's: no relaying). On link up, all of them; then each change.
    Shelf {
        items: Vec<crate::shelf::ShelfRecord>,
    },
    /// Protocol 4 (Mission 05 §2.3): this device's picture, or None for the
    /// default. Checked before decoding (`picture.rs`).
    Picture {
        picture: Option<crate::picture::Picture>,
    },
    /// Protocol 4: receiver → sender, for a paused transfer: the bytes it has
    /// kept (checked and on disk), so a sender that crashed shows a
    /// percentage it knows for sure.
    Kept {
        tid: TransferId,
        bytes: u64,
    },
    /// Receiver → sender: what already arrived and checks out, so the sender
    /// continues from there: folder entries received, files done, bytes of
    /// the next file it already has, and all bytes so far (for progress).
    /// `prefix` is the BLAKE3 of those `offset` bytes: the sender compares
    /// it with its file and starts that file again if they differ.
    ResumeFrom {
        tid: TransferId,
        dirs: u64,
        file: u64,
        offset: u64,
        bytes: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prefix: Option<B32>,
    },

    /// A message type from a newer version of the same protocol: ignored.
    #[serde(other)]
    Unknown,
}

impl Msg {
    /// Messages that belong to a running transfer's stream and must stay in
    /// order with its data frames.
    pub fn transfer_id(&self) -> Option<TransferId> {
        match self {
            Msg::Dirs { tid, .. }
            | Msg::File { tid, .. }
            | Msg::FileEnd { tid, .. }
            | Msg::Done { tid }
            | Msg::Complete { tid, .. }
            | Msg::Progress { tid, .. }
            | Msg::Cancel { tid, .. } => Some(*tid),
            _ => None,
        }
    }
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

pub fn check_version(theirs: u32) -> std::result::Result<(), ErrorCode> {
    if theirs == PROTOCOL_VERSION { Ok(()) } else { Err(ErrorCode::IncompatibleVersion) }
}

/// Serialises a frame into one buffer (header + payload).
pub fn encode(frame: &Frame) -> Result<Vec<u8>> {
    let (kind, payload): (u8, std::borrow::Cow<'_, [u8]>) = match frame {
        Frame::Control(msg) => (KIND_CONTROL, serde_json::to_vec(msg)?.into()),
        Frame::Data(bytes) => (KIND_DATA, bytes.as_slice().into()),
    };
    let max = if kind == KIND_CONTROL { MAX_CONTROL_LEN } else { MAX_DATA_LEN };
    let len = u32::try_from(payload.len())
        .ok()
        .filter(|l| *l <= max)
        .ok_or_else(|| Error::Protocol("frame too large".into()))?;
    let mut out = Vec::with_capacity(5 + payload.len());
    out.push(kind);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

pub async fn write_frame<W: AsyncWrite + Unpin>(w: &mut W, frame: &Frame) -> Result<()> {
    w.write_all(&encode(frame)?).await?;
    Ok(())
}

pub async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<Frame> {
    let mut header = [0u8; 5];
    r.read_exact(&mut header).await?;
    let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    let max = match header[0] {
        KIND_CONTROL => MAX_CONTROL_LEN,
        KIND_DATA => MAX_DATA_LEN,
        other => return Err(Error::Protocol(format!("unknown frame kind {other}"))),
    };
    if len > max {
        return Err(Error::Protocol(format!("frame of {len} bytes exceeds {max}")));
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload).await?;
    Ok(match header[0] {
        KIND_CONTROL => Frame::Control(serde_json::from_slice(&payload)?),
        _ => Frame::Data(payload),
    })
}
