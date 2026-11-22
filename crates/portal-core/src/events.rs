//! What the core tells its UI. Serialised to the webview as camelCase JSON.

use serde::Serialize;

use crate::model::{ClipTo, DeviceId, Incoming, Os};
use crate::net::discovery::Via;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    /// Full state; sent whenever anything the UI shows changes.
    Snapshot(Box<Snapshot>),
    /// One line for the Details log.
    Log(LogLine),
    /// Transfer progress, at most ~10 per second.
    Progress(ProgressView),
    /// Something the app may show as a native notification.
    Notice(Notice),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Out,
    In,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferPhase {
    /// Sender: walking the files.
    Preparing,
    /// Waiting for the receiver to accept.
    Offered,
    Running,
    Done,
    Failed,
    /// The receiver said no.
    Declined,
    /// The receiver doesn't accept files from us (see `block`).
    Blocked,
    /// The connection dropped: it continues by itself when the device is
    /// back (see `paused_since_ms`, `resuming`).
    Paused,
    /// Cancel was pressed on this computer (`failure` says what happened to
    /// the files).
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Block {
    /// The receiver's "Files from <this device>" is set to Never.
    NotAccepted,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub name: String,
    pub folder: bool,
    pub bytes: u64,
    pub files: u64,
}

/// One item a done screen can name: "Docs/link.txt", "a link (shortcut)".
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedItem {
    /// Where it sits in what was sent, with "/" between folders.
    pub path: String,
    pub why: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultView {
    /// Receiver: the folder files were saved to.
    pub saved_to: Option<String>,
    /// Receiver: first saved item, for "Show in Finder".
    pub reveal: Option<String>,
    /// Receiver: one file that's safe to Open (a document, image or media;
    /// never an app, installer, script, shortcut or archive: `open.rs`).
    pub open: Option<String>,
    pub files: u64,
    pub bytes: u64,
    pub renamed: u64,
    pub skipped: u64,
    pub long_paths: u64,
    /// Bytes that were already there after a pause and weren't sent again
    /// (the finished speed leaves them out).
    pub kept_bytes: u64,
    /// Which items were left out, and why (the first 50).
    pub skipped_items: Vec<ListedItem>,
    /// Which names changed, to what, and why (the first 50).
    pub renamed_items: Vec<ListedItem>,
    /// Plain sentences for the done screen.
    pub notes: Vec<String>,
    /// Receiver: the first files this computer ever received (the done
    /// screen offers Change folder too).
    pub first_arrival: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferView {
    pub id: String,
    /// The same id, for commands from Rust (tests, the shell).
    #[serde(skip)]
    pub tid: crate::proto::TransferId,
    pub direction: Direction,
    pub peer_id: DeviceId,
    pub peer_name: String,
    pub phase: TransferPhase,
    pub block: Option<Block>,
    pub items: Vec<ItemView>,
    pub total_bytes: u64,
    pub file_count: u64,
    pub auto_accepted: bool,
    /// Bytes moved so far.
    pub bytes: u64,
    pub elapsed_ms: Option<u64>,
    pub link: Option<LinkKind>,
    /// Running, but nothing has moved for a few seconds (the network
    /// dropped out; it either comes back or the transfer stops).
    pub stalled: bool,
    pub result: Option<ResultView>,
    pub failure: Option<String>,
    /// Receiver: the folder its files are (or will be) saved in.
    pub save_to: Option<String>,
    /// Paused since (unix ms).
    pub paused_since_ms: Option<u64>,
    /// Paused, connected again, checking what arrived before continuing.
    pub resuming: bool,
    /// Sender, stopped or cancelled: "Send again" sends the same items.
    pub can_send_again: bool,
    /// Paused because this app was quit or crashed mid-transfer: the window
    /// asks Continue or Cancel (Mission 05 §1.4).
    pub stopped_here: bool,
    /// Paused because the other computer was quit or crashed: waiting for
    /// someone there to continue.
    pub waiting_there: bool,
    /// The percentage is known for sure (a sender after a crash only knows
    /// once the receiver says what it kept).
    pub percent_known: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressView {
    pub id: String,
    /// Bytes that arrived (on the sender: what the receiver says it has).
    pub bytes: u64,
    pub total: u64,
    /// Bytes per second over the last 10 s (for Details).
    pub rate: f64,
    /// The speed to show: changes at most every 2 s, 2 significant figures;
    /// None for the first 2 s of a run.
    pub speed: Option<f64>,
    /// Time left, in words the UI spells out (`Eta::words`).
    pub eta: crate::transfer::estimate::Eta,
    pub file: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Offer,
    Received,
    Failed,
    Paused,
    /// Clipboard contents arrived (sent on purpose, not by automatic sync).
    Clipboard,
    /// Automatic clipboard sync was turned on or off from the other computer.
    ClipboardSync,
    /// A device asks to pair (Show code / Ignore in the window).
    Pairing,
    /// Paired with a device.
    Paired,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub kind: NoticeKind,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub me: Me,
    pub peers: Vec<PeerView>,
    pub pairing: Option<PairingView>,
    /// Every transfer on screen, oldest first: any number across devices,
    /// at most one active per device (Mission 04 §3).
    pub transfers: Vec<TransferView>,
    /// The connection doctor's checklist (problems first).
    pub doctor: Vec<crate::doctor::Check>,
    /// Milliseconds since discovery (re)started; the UI shows the doctor's
    /// suggestions after 10 s with nothing found.
    pub search_ms: u64,
    /// Clipboard items received, newest first (memory only).
    pub clips: Vec<ClipView>,
    /// What goes next to a busy device, earliest first (Mission 05 §1.5).
    pub next: Vec<NextView>,
    /// The Shelf, newest first (Mission 05 §3.2).
    pub shelf: Vec<ShelfView>,
    /// The latest copy that wasn't synced, and why (Mission 05 §3.1).
    pub clip_notice: Option<ClipNotice>,
}

/// Why a copy wasn't synced (Mission 05 §3.1), said once in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipNoticeKind {
    /// "Not synced: the app you copied from marked it private." [Put on Shelf]
    Private,
    /// "Too big for the clipboard." [Send as file]
    TooBig,
    /// "Couldn't read that copy. Try Send clipboard."
    Unreadable,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipNotice {
    /// New for each copy (the window shows each once).
    pub id: u64,
    pub kind: ClipNoticeKind,
    /// The clipboard's change count it's about: Put on Shelf reads the copy
    /// only while it's still this one.
    pub count: u64,
    pub at_ms: u64,
}

/// Files waiting for a busy device: "Next for Desk PC: 2 files".
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextView {
    pub id: u64,
    pub peer_id: DeviceId,
    pub peer_name: String,
    /// What was dropped, by name (files and folders).
    pub names: Vec<String>,
    /// How many of them are folders.
    pub folders: u64,
    /// Kept through a quit or a crash: waits for Continue.
    pub held: bool,
    /// The device's slot is taken by a paused transfer (not a moving one).
    pub waiting_for_paused: bool,
}

/// One received clipboard item. Its contents stay in the core.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipView {
    pub id: u64,
    pub from: String,
    /// The device it came from (None: typed here).
    pub from_id: Option<DeviceId>,
    pub at_ms: u64,
    /// "text", "rich text" or "image"
    pub kind: String,
    /// "text, 1.2 KB"
    pub summary: String,
    /// “meeting notes…”, "An image, 1200 × 800"
    pub preview: String,
    /// Still held, so Copy again works (big items are let go after a while).
    pub copyable: bool,
    /// Also put on this computer's clipboard (false: "Shelf only").
    pub pasted: bool,
    /// Sent as private on purpose: no preview, and marked private when copied
    pub private: bool,
    /// An image: a small preview (`data:` URL), for Recently received.
    pub thumb: Option<String>,
}

/// A paired device and the clipboard (Mission 04 §3–4).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipPeerView {
    /// This computer agreed to sync with it.
    pub mine: bool,
    /// It agreed to sync with this computer.
    pub theirs: bool,
    /// It asks this computer to agree (unanswered here).
    pub asks: bool,
    /// Where its clipboard items go here.
    pub to: ClipTo,
}

/// Sharing the Shelf with one paired device (Mission 05 §3.2).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfPeerView {
    /// This computer agreed.
    pub mine: bool,
    /// It agreed.
    pub theirs: bool,
    /// The person here has answered "Share the Shelf with it?".
    pub answered: bool,
}

/// One item on the Shelf. A private item's text never leaves the core here
/// (Reveal and Copy ask for it).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfView {
    pub id: String,
    /// "Vercel key", or empty.
    pub name: String,
    /// The first line (up to 80 characters); None for a private item.
    pub preview: Option<String>,
    pub private: bool,
    /// Where it was added: this computer, or a device it shares with.
    pub from_id: DeviceId,
    pub from_name: String,
    pub mine: bool,
    /// When it was added (unix ms).
    pub at_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub port: u16,
    /// Protocol version this computer speaks (compare with a peer's `incompatible`).
    pub proto: u32,
    pub receive_dir: String,
    pub skip_junk: bool,
    /// Install updates automatically (Settings → About).
    pub auto_update: bool,
    /// Get test builds (Settings → About): the beta feed is checked too.
    pub test_builds: bool,
    /// Copies sync with at least one device right now (both agreed; not
    /// paused). Per device: `PeerView::clip`.
    pub clipboard_sync: bool,
    /// Sync is paused here until then (unix ms; 0: until SeamDrop restarts).
    pub clipboard_paused: Option<u64>,
    /// Where files and the clipboard go: the remembered choice (every
    /// paired device until one is made).
    pub send_to: Vec<DeviceId>,
    /// The Send clipboard shortcut, if changed from the default.
    pub clipboard_shortcut: Option<String>,
    /// System, Light or Dark (Settings, the tray)
    pub appearance: crate::model::Appearance,
    /// "A folder for each device" (Settings)
    pub folder_per_device: bool,
    /// How received files are put away (Mission 05 §4), and their dates.
    pub receive_mode: crate::store::ReceiveMode,
    pub keep_dates: bool,
    /// What changed about received files with the update, said once.
    pub receive_note: Option<String>,
    /// This computer's picture (None: the default).
    pub picture: Option<crate::picture::Picture>,
    /// Too many wrong pairing codes: requests are turned down until the
    /// person allows them again.
    pub pairing_locked: bool,
    /// Who typed the last wrong code: "Desk PC (192.168.1.31)"
    pub pairing_locked_by: Option<String>,
    /// When (epoch ms)
    pub pairing_locked_at_ms: Option<u64>,
    /// The app's version and the OS's, for Send feedback ("SeamDrop 0.4.0 ·
    /// macOS 15.1 (24B83)")
    pub app_version: String,
    pub os_version: String,
    /// Where the other computer can reach this one (shown for "Connect by IP").
    pub addresses: Vec<AddressView>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressView {
    /// "Wi-Fi" or "Ethernet"
    pub label: String,
    pub address: String,
}

/// What kind of path a link takes. The UI shows Wi-Fi, Ethernet or Network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// A direct wired link (no router). Preferred when there is one, but
    /// not advertised: it's shown as Ethernet (see `LinkKind::shown`).
    Cable,
    /// Wired, through a router.
    Ethernet,
    Wifi,
    Other,
}

impl LinkKind {
    /// What the UI is told: a direct link is just Ethernet to it.
    pub fn shown(self) -> LinkKind {
        if self == LinkKind::Cable { LinkKind::Ethernet } else { self }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerView {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    /// In our trusted list.
    pub paired: bool,
    /// We have a live, authenticated connection.
    pub online: bool,
    pub link: Option<LinkKind>,
    /// Best address, for Details.
    pub addr: Option<String>,
    pub via: Option<Via>,
    /// What this computer does with files from it.
    pub incoming: Incoming,
    /// What it does with files from this computer, once it has told us.
    pub accepts: Option<Incoming>,
    /// Runs a different protocol version (the UI says which side to update).
    pub incompatible: Option<u32>,
    /// It was paired, but it removed us: pair again to send.
    pub revoked: bool,
    /// Paired, but Disconnect was pressed here: not connecting until Connect.
    pub disconnected: bool,
    /// Paired and not connected: how connecting is going (Mission 04 §1.1).
    /// The UI says "Connecting…" or "isn't reachable" from this alone.
    pub reach: Option<ReachView>,
    /// Paired: the clipboard with it.
    pub clip: Option<ClipPeerView>,
    /// Sharing the Shelf with it (paired devices only).
    pub shelf: Option<ShelfPeerView>,
    /// Its picture, as it sent it and once checked (Mission 05 §2.3); None:
    /// the default (a colour from its id and its system's icon).
    pub picture: Option<crate::picture::Picture>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReachState {
    /// Still within the grace period, or not enough attempts failed yet.
    Connecting,
    /// The grace period is over and real attempts failed.
    Unreachable,
}

/// What connecting is doing right now, for plain words in the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReachStep {
    /// No address to try: looking on this network.
    Looking,
    /// Trying the address it was last seen at.
    TryingKnown,
    /// Trying an address it was just found at.
    TryingFound,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachView {
    pub state: ReachState,
    /// When this connecting period started (unix ms).
    pub since_ms: u64,
    /// Dials made in this period.
    pub attempts: u32,
    /// The last failure, in plain words ("Nothing answered at 192.168.1.31").
    pub last_failure: Option<String>,
    /// When the next dial is due (unix ms), if one is booked.
    pub next_try_ms: Option<u64>,
    pub step: ReachStep,
}

/// Where a pairing by typed code is (Mission 04 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PairStep {
    /// Asker: waiting for the other computer's person (who may ignore it)
    Asking,
    /// Shower: "… wants to pair · Show code / Ignore"
    Asked,
    /// Asker: type the code now. Shower: showing it
    Code,
    /// Checking a typed code
    Checking,
    /// That wasn't the code: the asker types it again
    Wrong,
    /// "Paired with …" (for a few seconds)
    Paired,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingView {
    pub peer_id: DeviceId,
    pub peer_name: String,
    /// Its address: "MacBook Pro (192.168.1.20) wants to pair"
    pub peer_addr: String,
    /// This computer asked (its person types the code).
    pub mine: bool,
    pub step: PairStep,
    /// The shower's code, once shown. The asker never has it.
    pub code: Option<String>,
    /// Wrong codes still allowed in this pairing
    pub tries_left: u32,
    /// Why it ended without trust (shown until dismissed).
    pub failed: Option<String>,
    /// How long this step has left (the code works for 2 minutes), at the
    /// time of the snapshot.
    pub left_ms: u64,
}

/// How a line of Details reads (Mission 04 §1.6). None is the default:
/// neutral. Amber only when something is wrong and affects the person; red
/// only when something failed and needs them; dim for the quiet ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Warn,
    Err,
    Dim,
}

/// One line of the story Details tells: what happened in plain words, then
/// the technical part. Never clipboard content.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    /// Unix milliseconds; the UI formats local time.
    pub at_ms: u64,
    /// What it's about, for the technical log ("link", "verify")
    pub tag: String,
    /// Plain words: "Found Desk PC on this network"
    pub text: String,
    /// The technical part: "192.168.1.20:47920 via mDNS"
    pub detail: Option<String>,
    pub tone: Option<Tone>,
}
