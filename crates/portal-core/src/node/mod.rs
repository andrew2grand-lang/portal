//! The node: one actor task that owns the app's state (peers, links, trust,
//! settings) and talks to the UI through commands and events.

mod clip;
mod connect;
mod netwatch;
mod next;
mod pair;
mod paths;
mod reach;
mod resume;
mod shelf;
mod trust;
mod xfer;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

use tokio::sync::{mpsc, oneshot};

pub use connect::parse_target;
pub use pair::PairAction;
pub use trust::SettingsPatch;

use crate::consts::{APP_NAME, DEFAULT_PORT};
use crate::events::{AddressView, Event, LinkKind, LogLine, Me, PeerView, Snapshot, Tone};
use crate::identity::Identity;
use crate::model::{DeviceId, Fingerprint, Incoming, Os, Role};
use crate::net::discovery::{self, Discovery, DiscoveryConfig, Sighting, Via};
use crate::net::iface::{self, Iface};
use crate::net::link::{self, HandshakeError, Link, LinkEvent, LinkId};
use crate::net::{dial, listen};
use crate::proto::{ErrorCode, Hello, Msg, PROTOCOL_VERSION, TransferId};
use crate::store::Store;
use crate::util::now_ms;

/// At most this many messages wait for a link to be adopted, for this long.
const EARLY_MAX: usize = 256;
const EARLY_KEEP: Duration = Duration::from_secs(10);
const TICK: Duration = Duration::from_secs(1);
const IFACE_EVERY: Duration = Duration::from_secs(3);
/// A path nobody has confirmed for this long is forgotten.
const PATH_TTL: Duration = Duration::from_secs(90);
const DIAL_TIMEOUT: Duration = Duration::from_secs(4);
/// The next address gets a try after this long even if earlier ones are
/// still connecting: a stale address mustn't hold up the right one.
const DIAL_STAGGER: Duration = Duration::from_millis(250);
/// Connections from devices we don't trust (yet) are only good for pairing:
/// at most this many per remote address, and this many in total.
pub const MAX_UNTRUSTED_PER_ADDR: usize = 4;
pub const MAX_UNTRUSTED: usize = 32;
/// An untrusted connection that isn't pairing is closed after this long.
const UNTRUSTED_IDLE: Duration = Duration::from_secs(120);

#[derive(Clone, Debug)]
pub struct Config {
    pub data_dir: PathBuf,
    /// Overrides the saved device name (tests).
    pub device_name: Option<String>,
    pub port: u16,
    pub udp_port: u16,
    pub loopback_only: bool,
    pub mdns: bool,
    pub broadcast: bool,
    pub extra_targets: Vec<SocketAddr>,
    pub app_version: String,
    /// Overrides the saved receive folder (tests).
    pub receive_dir: Option<PathBuf>,
    /// Fault injection for tests: pretend this much space is free.
    pub fault_free_space: Option<u64>,
    /// Fault injection for tests: writing past this many bytes fails as "disk full".
    pub fault_write_limit: Option<u64>,
    /// Apply another OS's naming rules to received files (tests; default: this OS).
    pub name_rules: Option<crate::fsafe::Target>,
    /// The old app's data folder: settings, paired devices and keys are
    /// copied from it on the first start (see `migrate`).
    pub migrate_from: Option<PathBuf>,
    /// How long a paused transfer waits for the other computer.
    pub resume_for: Duration,
    /// Fault injection for tests: the sender closes the link once, after
    /// sending this many bytes of a transfer.
    pub fault_cut_at: Option<u64>,
    /// Fault injection for tests: send at most this many bytes per second.
    pub fault_rate: Option<u64>,
    /// Fault injection for tests: re-read a resumed file's first part at
    /// most this many bytes per second.
    pub fault_hash_rate: Option<u64>,
    /// A running transfer that hears nothing for this long pauses.
    pub stall: Duration,
    /// A paired device shows as "isn't reachable" no sooner than this after
    /// connecting to it started (and only once real attempts failed).
    pub reach_grace: Duration,
    /// The clipboard to share (tests use one in memory).
    pub clipboard: Arc<dyn crate::clip::ClipboardIo>,
    /// Fault injection for tests: start listening this long after starting
    /// (needs a fixed `port`), like an app whose listener comes up late.
    pub fault_listen_after: Option<Duration>,
    /// Fault injection for tests: ignore discovery, so this node only ever
    /// dials stored addresses (like a firewall that lets only the other
    /// side's connections through).
    pub fault_blind: bool,
    /// A pairing that isn't finished in this time is abandoned (tests shorten it).
    pub pair_timeout: Duration,
    /// Pairing requests allowed per device key and per address in 10 minutes
    /// (tests raise it: every test computer is 127.0.0.1).
    pub pair_asks: usize,
}

impl Config {
    pub fn new(data_dir: impl Into<PathBuf>, app_version: &str) -> Self {
        Self {
            data_dir: data_dir.into(),
            device_name: None,
            port: DEFAULT_PORT,
            udp_port: DEFAULT_PORT,
            loopback_only: false,
            mdns: true,
            broadcast: true,
            extra_targets: vec![],
            app_version: app_version.to_owned(),
            receive_dir: None,
            fault_free_space: None,
            fault_write_limit: None,
            name_rules: None,
            migrate_from: None,
            resume_for: crate::transfer::journal::RESUME_FOR,
            fault_cut_at: None,
            fault_rate: None,
            fault_hash_rate: None,
            stall: crate::transfer::STALL,
            reach_grace: reach::GRACE,
            clipboard: Arc::new(crate::clip::SystemClipboard),
            fault_listen_after: None,
            fault_blind: false,
            pair_timeout: crate::pairing::TIMEOUT,
            pair_asks: crate::pairing::MAX_ATTEMPTS,
        }
    }

    /// Loopback only, random port, no discovery (tests switch parts on).
    pub fn for_tests(data_dir: &Path) -> Self {
        Self {
            port: 0,
            udp_port: 0,
            loopback_only: true,
            mdns: false,
            broadcast: false,
            clipboard: Arc::new(crate::clip::MemoryClipboard::default()),
            ..Self::new(data_dir, "test")
        }
    }
}

enum Cmd {
    Snapshot(oneshot::Sender<Snapshot>),
    Connect(String, oneshot::Sender<Result<PeerView, String>>),
    RetryDiscovery,
    NetworkChanged,
    Pair(DeviceId, oneshot::Sender<Result<(), String>>),
    Pairing(pair::PairAction, oneshot::Sender<Result<(), String>>),
    RemovePeer(DeviceId),
    RenamePeer(DeviceId, String),
    SetIncoming(DeviceId, Incoming),
    SetConnected(DeviceId, bool),
    Settings(SettingsPatch),
    Send(DeviceId, Vec<PathBuf>, oneshot::Sender<Result<(), String>>),
    SendMany(Vec<DeviceId>, Vec<PathBuf>, oneshot::Sender<Vec<(DeviceId, Result<(), String>)>>),
    Accept(TransferId, bool),
    Decline(TransferId),
    Cancel(TransferId),
    Dismiss(TransferId),
    SendClipboard(Option<Vec<DeviceId>>, oneshot::Sender<Result<crate::clip::ClipSent, String>>),
    SendText(Option<Vec<DeviceId>>, String, bool, oneshot::Sender<Result<String, String>>),
    SetClipSync(DeviceId, bool),
    ClipSyncOffAll,
    PauseClipSync(Option<Duration>),
    ResumeClipSync,
    SetClipTo(DeviceId, crate::model::ClipTo),
    SendAgain(TransferId, oneshot::Sender<Result<(), String>>),
    CopyClip(u64, oneshot::Sender<Result<(), String>>),
    CopyText(String, oneshot::Sender<Result<(), String>>),
    ClearClips,
    RemoveClip(u64),
    Shutdown(crate::transfer::journal::Stopped, oneshot::Sender<()>),
    Continue(TransferId),
    UnqueueNext(u64),
    ShelfAdd(String, String, bool, oneshot::Sender<Result<String, String>>),
    ShelfEdit(String, Option<String>, Option<String>, oneshot::Sender<Result<(), String>>),
    ShelfRemove(Option<Vec<String>>),
    ShelfUndo(oneshot::Sender<Result<(), String>>),
    ShelfCopy(String, oneshot::Sender<Result<(), String>>),
    ShelfReveal(String, oneshot::Sender<Option<String>>),
    SetShelfShare(DeviceId, bool),
    ShelfPutMarked(u64, oneshot::Sender<Result<(), String>>),
    ShelfPutReceived(u64, oneshot::Sender<Result<(), String>>),
    SaveClipImage(u64, String, oneshot::Sender<Result<String, String>>),
    DismissClipNotice,
    SetPicture(Option<crate::picture::Picture>, oneshot::Sender<Result<(), String>>),
    ContinueNext(DeviceId),
}

/// Handle to a running node. Cheap to clone.
#[derive(Clone)]
pub struct Core {
    tx: mpsc::Sender<Cmd>,
}

impl Core {
    /// Starts the node: identity, listener, discovery. Events go to the receiver.
    pub async fn start(cfg: Config) -> crate::Result<(Core, mpsc::UnboundedReceiver<Event>)> {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let (cmd_tx, cmd_rx) = mpsc::channel(64);
        let node = Node::new(cfg, events_tx).await?;
        tokio::spawn(node.run(cmd_rx));
        Ok((Core { tx: cmd_tx }, events_rx))
    }

    async fn call<T>(&self, make: impl FnOnce(oneshot::Sender<T>) -> Cmd) -> crate::Result<T> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(make(tx)).await.map_err(|_| stopped())?;
        rx.await.map_err(|_| stopped())
    }

    pub async fn snapshot(&self) -> crate::Result<Snapshot> {
        self.call(Cmd::Snapshot).await
    }

    /// "Connect by IP". Returns the device that answered, or a sentence to show.
    pub async fn connect(&self, input: &str) -> Result<PeerView, String> {
        let input = input.to_owned();
        self.call(|tx| Cmd::Connect(input, tx)).await.map_err(|e| e.to_string())?
    }

    /// "Try again": announce, probe known addresses, restart the search clock.
    pub async fn retry_discovery(&self) {
        let _ = self.tx.send(Cmd::RetryDiscovery).await;
    }

    /// The app noticed the network change (e.g. the computer woke up): check
    /// links and look again now. The core also notices on its own.
    pub async fn network_changed(&self) {
        let _ = self.tx.send(Cmd::NetworkChanged).await;
    }

    /// Asks a device that's been seen to pair. It asks its person first; the
    /// code is shown there and typed here (`pairing(PairAction::Code(…))`).
    pub async fn pair(&self, peer: &DeviceId) -> Result<(), String> {
        let peer = peer.clone();
        self.call(|tx| Cmd::Pair(peer, tx)).await.map_err(|e| e.to_string())?
    }

    /// What the person did about a pairing: Show code, Ignore, the typed
    /// code, Cancel, Dismiss, Allow pairing again. A typed code that isn't
    /// six digits says so.
    pub async fn pairing(&self, action: PairAction) -> Result<(), String> {
        self.call(|tx| Cmd::Pairing(action, tx)).await.map_err(|e| e.to_string())?
    }

    /// Stops trusting a device at once.
    pub async fn remove_peer(&self, id: &DeviceId) {
        let _ = self.tx.send(Cmd::RemovePeer(id.clone())).await;
    }

    pub async fn rename_peer(&self, id: &DeviceId, name: &str) {
        let _ = self.tx.send(Cmd::RenamePeer(id.clone(), name.to_owned())).await;
    }

    /// "Files from <device>": ask every time, always accept, or never.
    /// Saved, and told to that device so it can say so before sending.
    pub async fn set_incoming(&self, id: &DeviceId, value: Incoming) {
        let _ = self.tx.send(Cmd::SetIncoming(id.clone(), value)).await;
    }

    /// Disconnect from a paired device (false) or connect to it again (true).
    /// It stays paired either way; the choice survives restarts.
    pub async fn set_connected(&self, id: &DeviceId, connected: bool) {
        let _ = self.tx.send(Cmd::SetConnected(id.clone(), connected)).await;
    }

    pub async fn update_settings(&self, patch: SettingsPatch) {
        let _ = self.tx.send(Cmd::Settings(patch)).await;
    }

    /// Sends files and folders (absolute paths) to a paired device.
    pub async fn send(&self, peer: &DeviceId, paths: Vec<PathBuf>) -> Result<(), String> {
        let peer = peer.clone();
        self.call(|tx| Cmd::Send(peer, paths, tx)).await.map_err(|e| e.to_string())?
    }

    /// Sends the same files to several devices: one transfer each, running
    /// side by side. Says per device whether it started (and why not).
    pub async fn send_many(&self, peers: Vec<DeviceId>, paths: Vec<PathBuf>) -> Vec<(DeviceId, Result<(), String>)> {
        let fallback: Vec<_> = peers.iter().map(|p| (p.clone(), Err(stopped().to_string()))).collect();
        self.call(|tx| Cmd::SendMany(peers, paths, tx)).await.unwrap_or(fallback)
    }

    /// Accepts a waiting offer; `always` also turns on "Always accept from".
    pub async fn accept(&self, tid: TransferId, always: bool) {
        let _ = self.tx.send(Cmd::Accept(tid, always)).await;
    }

    pub async fn decline(&self, tid: TransferId) {
        let _ = self.tx.send(Cmd::Decline(tid)).await;
    }

    /// Cancels a transfer (or withdraws an offer).
    pub async fn cancel(&self, tid: TransferId) {
        let _ = self.tx.send(Cmd::Cancel(tid)).await;
    }

    /// Clears a finished, failed, declined or blocked transfer from the screen.
    pub async fn dismiss(&self, tid: TransferId) {
        let _ = self.tx.send(Cmd::Dismiss(tid)).await;
    }

    /// Send clipboard: to these devices, or (None) the remembered choice.
    /// The answer is a sentence to show either way ("Sent to Desk PC …");
    /// files on the clipboard come back as `files`, not sent.
    pub async fn send_clipboard(&self, to: Option<Vec<DeviceId>>) -> Result<crate::clip::ClipSent, String> {
        self.call(|tx| Cmd::SendClipboard(to, tx)).await.map_err(|e| e.to_string())?
    }

    /// The Shelf's text box: typed or pasted text, sent like a copy to these
    /// devices (None: the remembered choice). `private`: the other computer
    /// marks it private and shows no preview. The answer is a sentence.
    pub async fn send_text(&self, to: Option<Vec<DeviceId>>, text: String, private: bool) -> Result<String, String> {
        self.call(|tx| Cmd::SendText(to, text, private, tx)).await.map_err(|e| e.to_string())?
    }

    /// Agree (or stop agreeing) to sync the clipboard with one device. It
    /// runs once that device agrees too.
    pub async fn set_clip_sync(&self, id: &DeviceId, on: bool) {
        let _ = self.tx.send(Cmd::SetClipSync(id.clone(), on)).await;
    }

    /// "Turn off clipboard sync": with every device.
    pub async fn clip_sync_off(&self) {
        let _ = self.tx.send(Cmd::ClipSyncOffAll).await;
    }

    /// Pause sync here (local: consent stays). `None`: until SeamDrop restarts.
    pub async fn pause_clip_sync(&self, wait: Option<Duration>) {
        let _ = self.tx.send(Cmd::PauseClipSync(wait)).await;
    }

    pub async fn resume_clip_sync(&self) {
        let _ = self.tx.send(Cmd::ResumeClipSync).await;
    }

    /// Where a device's clipboard items go here: the clipboard, or the Shelf only.
    pub async fn set_clip_to(&self, id: &DeviceId, to: crate::model::ClipTo) {
        let _ = self.tx.send(Cmd::SetClipTo(id.clone(), to)).await;
    }

    /// "Send again" after a send stopped or was cancelled: the same items.
    pub async fn send_again(&self, tid: TransferId) -> Result<(), String> {
        self.call(|tx| Cmd::SendAgain(tid, tx)).await.map_err(|e| e.to_string())?
    }

    /// Copy again: puts a received item back on the clipboard.
    pub async fn copy_clip(&self, id: u64) -> Result<(), String> {
        self.call(|tx| Cmd::CopyClip(id, tx)).await.map_err(|e| e.to_string())?
    }

    /// Puts text of the app's own on the clipboard (Copy technical log);
    /// automatic sync doesn't send it on.
    pub async fn copy_text(&self, text: String) -> Result<(), String> {
        self.call(|tx| Cmd::CopyText(text, tx)).await.map_err(|e| e.to_string())?
    }

    pub async fn clear_clips(&self) {
        let _ = self.tx.send(Cmd::ClearClips).await;
    }

    /// Takes one item off the Shelf (from memory; the clipboard stays as it is).
    pub async fn remove_clip(&self, id: u64) {
        let _ = self.tx.send(Cmd::RemoveClip(id)).await;
    }

    /// Quitting: a transfer under way stops, and asks at the next launch.
    pub async fn shutdown(&self) {
        self.shutdown_for(crate::transfer::journal::Stopped::Quit).await;
    }

    /// Stopping for `why` (Quit asks at the next launch; Update continues by
    /// itself, like a dropped network).
    pub async fn shutdown_for(&self, why: crate::transfer::journal::Stopped) {
        let _ = self.call(|done| Cmd::Shutdown(why, done)).await;
    }

    /// Continue pressed on a transfer that stopped partway here.
    pub async fn continue_transfer(&self, tid: TransferId) {
        let _ = self.tx.send(Cmd::Continue(tid)).await;
    }

    /// Add to Shelf: kept here and on the devices it's shared with.
    pub async fn shelf_add(&self, name: &str, text: &str, private: bool) -> Result<(), String> {
        self.call(|tx| Cmd::ShelfAdd(name.into(), text.into(), private, tx))
            .await
            .map_err(|e| e.to_string())?
            .map(|_| ())
    }

    /// Renames or edits a Shelf item.
    pub async fn shelf_edit(&self, id: &str, name: Option<String>, text: Option<String>) -> Result<(), String> {
        self.call(|tx| Cmd::ShelfEdit(id.into(), name, text, tx)).await.map_err(|e| e.to_string())?
    }

    /// ✕ on one item (Some) or Clear Shelf (None), on every device it's shared with.
    pub async fn shelf_remove(&self, ids: Option<Vec<String>>) {
        let _ = self.tx.send(Cmd::ShelfRemove(ids)).await;
    }

    /// Undo, a few seconds after a removal.
    pub async fn shelf_undo(&self) -> Result<(), String> {
        self.call(Cmd::ShelfUndo).await.map_err(|e| e.to_string())?
    }

    /// Copy: on the clipboard (with the private marks for a private item).
    pub async fn shelf_copy(&self, id: &str) -> Result<(), String> {
        self.call(|tx| Cmd::ShelfCopy(id.into(), tx)).await.map_err(|e| e.to_string())?
    }

    /// Reveal: a private item's text, for a few seconds on screen.
    pub async fn shelf_reveal(&self, id: &str) -> Option<String> {
        self.call(|tx| Cmd::ShelfReveal(id.into(), tx)).await.ok().flatten()
    }

    /// This computer agrees (or not) to share the Shelf with `id`.
    pub async fn set_shelf_share(&self, id: &DeviceId, on: bool) {
        let _ = self.tx.send(Cmd::SetShelfShare(id.clone(), on)).await;
    }

    /// Put on Shelf from a "marked private" notice: reads that copy now, only
    /// if the clipboard still holds it (`count`).
    pub async fn shelf_put_marked(&self, count: u64) -> Result<(), String> {
        self.call(|tx| Cmd::ShelfPutMarked(count, tx)).await.map_err(|e| e.to_string())?
    }

    /// Put on Shelf from Recently received (text).
    pub async fn shelf_put_received(&self, id: u64) -> Result<(), String> {
        self.call(|tx| Cmd::ShelfPutReceived(id, tx)).await.map_err(|e| e.to_string())?
    }

    /// Save as file, for a received image: its path in the received files folder.
    pub async fn save_clip_image(&self, id: u64, name: &str) -> Result<String, String> {
        self.call(|tx| Cmd::SaveClipImage(id, name.into(), tx)).await.map_err(|e| e.to_string())?
    }

    /// The not-synced notice was seen.
    pub async fn dismiss_clip_notice(&self) {
        let _ = self.tx.send(Cmd::DismissClipNotice).await;
    }

    /// This computer's picture (None: the default), shared with paired devices.
    pub async fn set_picture(&self, picture: Option<crate::picture::Picture>) -> Result<(), String> {
        self.call(|tx| Cmd::SetPicture(picture, tx)).await.map_err(|e| e.to_string())?
    }

    /// ✕ on something waiting to go next to a busy device.
    pub async fn unqueue_next(&self, id: u64) {
        let _ = self.tx.send(Cmd::UnqueueNext(id)).await;
    }

    /// Continue on what was waiting for a device when the app was quit.
    pub async fn continue_next(&self, peer: &DeviceId) {
        let _ = self.tx.send(Cmd::ContinueNext(peer.clone())).await;
    }
}

fn stopped() -> crate::Error {
    crate::Error::Other(format!("{APP_NAME}'s core stopped"))
}

#[derive(Debug)]
struct PathEntry {
    addr: SocketAddr,
    via: Via,
    if_index: Option<u32>,
    seen: Instant,
}

#[derive(Debug)]
struct Peer {
    id: DeviceId,
    name: String,
    os: Os,
    /// What it does with our files, as it last told us.
    accepts: Option<Incoming>,
    paths: Vec<PathEntry>,
    link: Option<LinkId>,
    incompatible: Option<u32>,
}

impl Peer {
    fn new(id: DeviceId, name: String, os: Os) -> Self {
        Self { id, name, os, accepts: None, paths: Vec::new(), link: None, incompatible: None }
    }

    fn saw(&mut self, addr: SocketAddr, via: Via, if_index: Option<u32>) {
        match self.paths.iter_mut().find(|p| p.addr == addr) {
            Some(p) => {
                p.seen = Instant::now();
                p.via = via;
                p.if_index = if_index.or(p.if_index);
            }
            None => self.paths.push(PathEntry { addr, via, if_index, seen: Instant::now() }),
        }
    }
}

/// Why a dial failed, kept apart so the doctor can reason about it.
#[derive(Debug)]
#[allow(dead_code, reason = "fields are read through Debug in the log and by the doctor")]
enum DialFailure {
    /// Connection refused, timed out, or no route.
    NoAnswer(std::io::ErrorKind),
    Incompatible {
        theirs: u32,
        name: String,
        id: DeviceId,
    },
    Refused(ErrorCode),
    NotPortal(String),
}

impl From<HandshakeError> for DialFailure {
    fn from(e: HandshakeError) -> Self {
        match e {
            HandshakeError::Incompatible { theirs, name, id } => DialFailure::Incompatible { theirs, name, id },
            HandshakeError::Remote(code, _) => DialFailure::Refused(code),
            HandshakeError::Io(e) => DialFailure::NoAnswer(e.kind()),
            other => DialFailure::NotPortal(other.to_string()),
        }
    }
}

enum Purpose {
    Manual(oneshot::Sender<Result<PeerView, String>>),
    Pair(DeviceId),
    Reconnect(DeviceId),
}

enum Internal {
    #[cfg_attr(not(windows), allow(dead_code, reason = "Windows probes only"))]
    WindowsFacts(crate::doctor::WindowsFacts),
    Planned(crate::proto::TransferId, Result<crate::transfer::plan::Planned, String>),
    Sent(crate::proto::TransferId, Result<crate::proto::Summary, crate::transfer::Failure>),
    Received(crate::proto::TransferId, Result<crate::transfer::recv::Received, crate::transfer::Failure>),
    Restored(crate::proto::TransferId, Box<Result<resume::Restored, String>>),
    ClipRead(clip::Reader, crate::clip::Read),
    ClipWritten {
        from: String,
        from_id: DeviceId,
        contents: crate::clip::Contents,
        auto: bool,
        private: bool,
        written: Result<u64, String>,
    },
    ClipCopied(Option<u64>),
    Accepted(Result<Link, HandshakeError>, SocketAddr),
    Dialed {
        addr: SocketAddr,
        purpose: Purpose,
        result: Result<Link, DialFailure>,
    },
    Ifaces(Vec<Iface>),
}

struct Node {
    cfg: Config,
    identity: Identity,
    store: Store,
    port: u16,
    hello: Arc<RwLock<Hello>>,
    events: mpsc::UnboundedSender<Event>,
    link_tx: mpsc::Sender<LinkEvent>,
    link_rx: Option<mpsc::Receiver<LinkEvent>>,
    internal_tx: mpsc::Sender<Internal>,
    internal_rx: Option<mpsc::Receiver<Internal>>,
    sight_rx: Option<mpsc::Receiver<Sighting>>,
    peers: BTreeMap<DeviceId, Peer>,
    links: HashMap<LinkId, Link>,
    discovery: Option<Discovery>,
    ifaces: Arc<Vec<Iface>>,
    search_started: Instant,
    started: Instant,
    /// Set while shutting down: why transfers stop now (else the network).
    stopping: Option<crate::transfer::journal::Stopped>,
    /// What goes next to busy devices (Mission 05 §1.5).
    next: next::Queue,
    /// The Shelf and device pictures (Mission 05 §3.2, §2.3).
    shelf: shelf::ShelfState,
    next_id: u64,
    last_ifaces: Instant,
    dirty: bool,
    accept_task: Option<tokio::task::JoinHandle<()>>,
    pairing: Option<pair::PairSession>,
    limiter: crate::pairing::Limiter,
    /// Wrong pairing codes from anyone (locks pairing past the cap)
    fail_cap: crate::pairing::FailCap,
    /// The last wrong pairing code: who typed it ("Desk PC (192.168.1.31)") and when
    last_wrong: Option<(String, u64)>,
    /// Read once at start (Send feedback)
    os_version: String,
    reach: reach::Reaches,
    dialing: HashSet<DeviceId>,
    upgrade_tried: HashMap<DeviceId, Instant>,
    /// Every transfer on screen: any number across devices, at most one
    /// active per device. Finished ones stay until dismissed.
    transfers: Vec<xfer::Transfer>,
    /// Saved state of transfers, so a paused one survives a restart.
    journal: crate::transfer::journal::Journal,
    /// Test fault: (cut after this many bytes, already cut).
    cut: Option<(u64, Arc<std::sync::atomic::AtomicBool>)>,
    clip: clip::ClipState,
    facts: DoctorFacts,
    doctor: Vec<crate::doctor::Check>,
    last_doctor: Instant,
    /// When each link was adopted (idle unpaired links are closed).
    link_born: HashMap<LinkId, Instant>,
    /// Messages that arrived on a link before it was adopted, in order. A
    /// link's messages and the link itself reach the loop on different
    /// channels, and `select!` may take the messages first: they wait here
    /// and are handled once it's adopted (or dropped after a while).
    early: HashMap<LinkId, (Instant, Vec<Msg>)>,
    /// Links pinged after a network change, and when.
    probing: HashMap<LinkId, Instant>,
    /// Keys of paired devices disconnected here: turned away before the hellos.
    turned_away: Arc<RwLock<HashSet<Fingerprint>>>,
    /// Wall and monotonic time at the last tick (to notice sleep).
    last_tick: (SystemTime, Instant),
    sweep: SweepState,
}

/// Unicast sweeps of the local network while alone (see `netwatch`).
#[derive(Default)]
struct SweepState {
    alone_since: Option<Instant>,
    last: Option<Instant>,
    done: u32,
    /// The person asked to look (Pair another computer, Try again): keep
    /// sweeping for a while even with paired devices connected.
    asked: Option<Instant>,
}

/// What the node has noticed, for the connection doctor.
#[derive(Default)]
struct DoctorFacts {
    local_dial_ok: Option<Instant>,
    host_unreachable: Option<Instant>,
    known_silent: bool,
    write_denied: bool,
    /// A paired device that told us it's disconnected from us.
    remote_disconnected: Option<String>,
    /// Another copy of the app on this computer was last seen then.
    twin: Option<Instant>,
    windows: Option<crate::doctor::WindowsFacts>,
    #[cfg_attr(not(windows), allow(dead_code, reason = "Windows probes only"))]
    windows_probed: Option<Instant>,
}

impl Node {
    async fn new(cfg: Config, events: mpsc::UnboundedSender<Event>) -> crate::Result<Self> {
        let migrated = cfg.migrate_from.as_ref().map(|old| crate::migrate::migrate(old, &cfg.data_dir));
        let identity = Identity::load_or_create(&cfg.data_dir)?;
        let mut store = Store::open(&cfg.data_dir)?;
        if let Some(name) = &cfg.device_name {
            store.update_settings(|s| s.device_name = name.clone())?;
        }
        if let Some(dir) = &cfg.receive_dir {
            store.update_settings(|s| s.receive_dir = dir.clone())?;
        }
        let (listeners, port) = match cfg.fault_listen_after {
            Some(_) if cfg.port != 0 => (None, cfg.port),
            _ => {
                let l = listen::bind(cfg.port, cfg.loopback_only).await?;
                let port = l.port;
                (Some(l), port)
            }
        };
        let server_tls = crate::tls::server_config(&identity)?;
        let settings = store.settings().clone();
        let hello = Arc::new(RwLock::new(Hello {
            proto: PROTOCOL_VERSION,
            id: identity.device_id.clone(),
            name: settings.device_name.clone(),
            os: Os::current(),
            role: Role::Both,
            app: cfg.app_version.clone(),
            port,
        }));
        let (link_tx, link_rx) = mpsc::channel(256);
        let (internal_tx, internal_rx) = mpsc::channel(256);
        let (sight_tx, sight_rx) = mpsc::channel(256);

        let ifaces = Arc::new(if cfg.loopback_only { Vec::new() } else { iface::snapshot() });
        let discovery = if cfg.mdns || cfg.broadcast {
            let dcfg = DiscoveryConfig {
                id: identity.device_id.clone(),
                name: settings.device_name.clone(),
                os: Os::current(),
                port,
                udp_port: cfg.udp_port,
                mdns: cfg.mdns,
                broadcast: cfg.broadcast,
                extra_targets: cfg.extra_targets.clone(),
                loopback: cfg.loopback_only,
            };
            Some(discovery::start(dcfg, sight_tx).await?)
        } else {
            None
        };

        let journal = crate::transfer::journal::Journal::new(&cfg.data_dir);
        let cut = cfg.fault_cut_at.map(|at| (at, Arc::default()));
        let turned_away: Arc<RwLock<HashSet<Fingerprint>>> = Default::default();
        let accept_task = {
            let (hello, link_tx, internal_tx, turned_away) =
                (hello.clone(), link_tx.clone(), internal_tx.clone(), turned_away.clone());
            let (late, loopback) = (cfg.fault_listen_after, cfg.loopback_only);
            tokio::spawn(async move {
                let listeners = match listeners {
                    Some(l) => l,
                    None => {
                        tokio::time::sleep(late.unwrap_or_default()).await;
                        match listen::bind(port, loopback).await {
                            Ok(l) => l,
                            Err(e) => return tracing::error!("listening late on {port}: {e}"),
                        }
                    }
                };
                accept_loop(listeners, server_tls, hello, link_tx, internal_tx, turned_away).await
            })
        };

        let cfg_grace = cfg.reach_grace;
        let pair_asks = cfg.pair_asks;
        let mut node = Self {
            cfg,
            identity,
            store,
            port,
            hello,
            events,
            link_tx,
            link_rx: Some(link_rx),
            internal_tx,
            internal_rx: Some(internal_rx),
            sight_rx: Some(sight_rx),
            peers: BTreeMap::new(),
            links: HashMap::new(),
            discovery,
            ifaces,
            search_started: Instant::now(),
            started: Instant::now(),
            stopping: None,
            next: next::Queue::default(),
            shelf: shelf::ShelfState::default(),
            next_id: 0,
            last_ifaces: Instant::now(),
            dirty: true,
            accept_task: Some(accept_task),
            pairing: None,
            limiter: crate::pairing::Limiter::new(pair_asks),
            fail_cap: Default::default(),
            last_wrong: None,
            os_version: portal_sys::host::os_version(),
            reach: reach::Reaches::new(cfg_grace),
            dialing: HashSet::new(),
            upgrade_tried: HashMap::new(),
            transfers: Vec::new(),
            journal,
            cut,
            clip: Default::default(),
            facts: DoctorFacts::default(),
            doctor: Vec::new(),
            last_doctor: Instant::now() - Duration::from_secs(60),
            link_born: HashMap::new(),
            early: HashMap::new(),
            probing: HashMap::new(),
            turned_away,
            last_tick: (SystemTime::now(), Instant::now()),
            sweep: SweepState::default(),
        };
        match migrated {
            Some(Ok(Some(m))) => node.log_migration(&m),
            Some(Err(e)) => node.say(
                "migrate",
                "Couldn’t bring over the old app’s settings. Pair again if the other computer asks",
                Some(e.to_string()),
                Some(Tone::Warn),
            ),
            _ => {}
        }
        let now = Instant::now();
        for t in node.store.peers().iter().filter(|t| t.trusted && !t.disconnected) {
            node.reach.start(&t.id, now);
        }
        node.load_paused();
        node.load_next();
        node.load_shelf();
        node.clean_staging();
        node.sync_turned_away();
        node.say(
            "start",
            format!("{APP_NAME} started as {}", node.store.settings().device_name),
            Some(format!("port {port}, protocol {PROTOCOL_VERSION}")),
            Some(Tone::Dim),
        );
        Ok(node)
    }

    async fn run(mut self, mut cmd_rx: mpsc::Receiver<Cmd>) {
        let mut link_rx = self.link_rx.take().expect("link receiver");
        let mut internal_rx = self.internal_rx.take().expect("internal receiver");
        let mut sight_rx = self.sight_rx.take().expect("sighting receiver");
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut progress_tick = tokio::time::interval(Duration::from_millis(100));
        progress_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut clip_tick = tokio::time::interval(clip::WATCH_EVERY);
        clip_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => match cmd {
                    Some(Cmd::Shutdown(why, done)) => {
                        self.stopping = Some(why);
                        self.save_next(Some(why));
                        self.stop();
                        self.wait_for_pause(&mut internal_rx).await;
                        let _ = done.send(());
                        return;
                    }
                    Some(cmd) => self.on_cmd(cmd),
                    None => {
                        self.stop();
                        return;
                    }
                },
                Some(ev) = link_rx.recv() => self.on_link_event(ev),
                Some(i) = internal_rx.recv() => self.on_internal(i),
                Some(s) = sight_rx.recv() => self.on_sighting(s),
                _ = tick.tick() => self.on_tick(),
                // Only while files move: an idle app shouldn't wake ten times a second.
                _ = progress_tick.tick(), if self.transfer_running() => self.emit_progress(),
                // Only while automatic clipboard sync is on and someone's there.
                _ = clip_tick.tick(), if self.clip_watching() => self.clip_watch(),
            }
            if self.dirty {
                self.dirty = false;
                let snapshot = self.snapshot();
                let _ = self.events.send(Event::Snapshot(Box::new(snapshot)));
            }
        }
    }

    /// Quitting mid-transfer: the links are closed, so the transfer pauses;
    /// wait (briefly) until it has saved where it got to.
    async fn wait_for_pause(&mut self, internal_rx: &mut mpsc::Receiver<Internal>) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while self.transfer_running() {
            match tokio::time::timeout_at(deadline, internal_rx.recv()).await {
                Ok(Some(i)) => self.on_internal(i),
                _ => break,
            }
        }
    }

    fn stop(&mut self) {
        for link in self.links.values() {
            link.close();
        }
        if let Some(t) = self.accept_task.take() {
            t.abort();
        }
        self.discovery = None;
    }

    // ---- logging & snapshot ------------------------------------------------

    fn log_migration(&self, m: &crate::migrate::Migrated) {
        let devices = match m.devices {
            1 => "1 paired device".to_owned(),
            n => format!("{n} paired devices"),
        };
        self.say(
            "migrate",
            format!("Brought over the settings, {devices} and keys from Portal"),
            Some(format!("old folder left at {}", m.from.display())),
            None,
        );
        if let Some((before, after)) = &m.receive_dir {
            self.say(
                "migrate",
                format!("Received files now go to {}; earlier ones stay in {}", after.display(), before.display()),
                None,
                None,
            );
            // Partial files from the old app can't be resumed.
            let _ = std::fs::remove_dir_all(before.join(crate::consts::LEGACY_INCOMING_DIR));
        }
    }

    /// A line of the story Details tells (and of the technical log): plain
    /// words first, the technical part in `detail`. Tone: see [`Tone`].
    fn say(&self, tag: &str, text: impl Into<String>, detail: Option<String>, tone: Option<Tone>) {
        let line = LogLine { at_ms: now_ms(), tag: tag.into(), text: text.into(), detail, tone };
        match &line.detail {
            Some(d) => tracing::info!(target: "portal", "{} {} ({d})", line.tag, line.text),
            None => tracing::info!(target: "portal", "{} {}", line.tag, line.text),
        }
        let _ = self.events.send(Event::Log(line));
    }

    /// Only in the technical log (Copy technical log, the log file): what
    /// would be noise in the story.
    fn tech(&self, tag: &str, text: impl std::fmt::Display) {
        tracing::info!(target: "portal", "{tag} {text}");
    }

    fn snapshot(&self) -> Snapshot {
        let s = self.store.settings();
        Snapshot {
            me: Me {
                id: self.identity.device_id.clone(),
                name: s.device_name.clone(),
                os: Os::current(),
                port: self.port,
                proto: PROTOCOL_VERSION,
                receive_dir: s.receive_dir.display().to_string(),
                skip_junk: s.skip_junk,
                auto_update: s.auto_update,
                test_builds: s.test_builds,
                clipboard_sync: self.store.peers().iter().any(|p| self.clip_sync_with(&p.id)),
                clipboard_paused: self.clip_pause_view(),
                send_to: self.send_to(),
                appearance: s.appearance,
                folder_per_device: s.folder_per_device,
                receive_mode: s.receive_mode.unwrap_or_default(),
                keep_dates: s.keep_dates,
                receive_note: s.receive_note.clone(),
                picture: s.picture.clone(),
                pairing_locked: self.pairing_locked(),
                pairing_locked_by: self
                    .pairing_locked()
                    .then(|| self.last_wrong.as_ref().map(|w| w.0.clone()))
                    .flatten(),
                pairing_locked_at_ms: self.pairing_locked().then(|| self.last_wrong.as_ref().map(|w| w.1)).flatten(),
                app_version: self.cfg.app_version.clone(),
                os_version: self.os_version.clone(),
                clipboard_shortcut: s.clipboard_shortcut.clone(),
                addresses: self.addresses(),
            },
            peers: self.peer_views(),
            pairing: self.pairing.as_ref().map(|p| p.view(self.cfg.pair_timeout)),
            transfers: self.transfers.iter().map(|t| t.view()).collect(),
            doctor: self.doctor.clone(),
            search_ms: self.search_started.elapsed().as_millis() as u64,
            clips: self.clip_views(),
            next: self.next_views(),
            shelf: self.shelf_views(),
            clip_notice: self.clip.notice.clone(),
        }
    }

    /// This computer's Wi-Fi and Ethernet IPv4 addresses: what to type into
    /// Connect by IP on the other computer.
    fn addresses(&self) -> Vec<AddressView> {
        lan_addresses(&self.ifaces)
            .into_iter()
            .map(|(label, ip)| AddressView { label: label.into(), address: ip.to_string() })
            .collect()
    }

    fn peer_views(&self) -> Vec<PeerView> {
        let mut views: Vec<PeerView> = self.peers.values().map(|p| self.peer_view(p)).collect();
        // Trusted peers the network hasn't shown us yet still appear (offline).
        for t in self.store.peers() {
            if !self.peers.contains_key(&t.id) {
                views.push(PeerView {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    os: t.os,
                    paired: t.trusted,
                    online: false,
                    link: None,
                    addr: t.addrs.first().cloned(),
                    via: None,
                    incoming: t.incoming,
                    accepts: None,
                    incompatible: None,
                    revoked: !t.trusted,
                    disconnected: t.trusted && t.disconnected,
                    reach: self.reach_view(&t.id),
                    clip: self.clip_peer_view(&t.id),
                    shelf: self.shelf_peer_view(&t.id),
                    picture: self.shelf.pictures.get(&t.id).cloned(),
                });
            }
        }
        views
    }

    fn peer_view(&self, p: &Peer) -> PeerView {
        let trusted = self.store.peer(&p.id);
        let link = self.link_for(&p.id);
        let (addr, via, kind) = match &link {
            Some(l) => {
                let info = l.info();
                let remote = info.remote;
                let path_via = p.paths.iter().find(|x| x.addr.ip() == remote.ip()).map(|x| x.via);
                (Some(remote), path_via, Some(self.link_kind(&remote, Some(info.local.ip()), None).shown()))
            }
            None => match self.best_path(p) {
                Some(path) => {
                    (Some(path.addr), Some(path.via), Some(self.link_kind(&path.addr, None, path.if_index).shown()))
                }
                None => (None, None, None),
            },
        };
        PeerView {
            id: p.id.clone(),
            name: trusted.map(|t| t.name.clone()).unwrap_or_else(|| p.name.clone()),
            os: p.os,
            paired: trusted.is_some_and(|t| t.trusted),
            online: link.is_some(),
            link: kind,
            addr: addr.map(|a| a.to_string()),
            via,
            incoming: trusted.map(|t| t.incoming).unwrap_or_default(),
            accepts: p.accepts,
            incompatible: p.incompatible,
            revoked: trusted.is_some_and(|t| !t.trusted),
            disconnected: trusted.is_some_and(|t| t.trusted && t.disconnected),
            reach: if link.is_some() { None } else { self.reach_view(&p.id) },
            clip: self.clip_peer_view(&p.id),
            shelf: self.shelf_peer_view(&p.id),
            picture: self.shelf.pictures.get(&p.id).cloned(),
        }
    }

    /// A paired device we should be connected to (not disconnected here).
    fn wants_link(&self, id: &DeviceId) -> bool {
        self.store.peer(id).is_some_and(|t| t.trusted && !t.disconnected)
    }

    /// How connecting to a paired, unconnected device is going.
    fn reach_view(&self, id: &DeviceId) -> Option<crate::events::ReachView> {
        if !self.wants_link(id) || self.link_for(id).is_some() {
            return None;
        }
        self.reach.view(id)
    }

    fn link_kind(&self, remote: &SocketAddr, local_ip: Option<IpAddr>, if_hint: Option<u32>) -> LinkKind {
        paths::classify(&self.ifaces, remote, local_ip, if_hint)
    }

    /// Unpaired connections exist only to pair; close them when idle.
    fn close_idle_untrusted(&mut self) {
        let pairing_link = self.pairing.as_ref().and_then(|p| p.link);
        let stale: Vec<Link> = self
            .links
            .values()
            .filter(|l| {
                Some(l.id()) != pairing_link
                    && !self.link_trusted(l)
                    && self.link_born.get(&l.id()).is_some_and(|t| t.elapsed() > UNTRUSTED_IDLE)
            })
            .cloned()
            .collect();
        for l in stale {
            l.close();
        }
    }

    /// Recomputes the doctor's checklist (and refreshes Windows probes now and then).
    fn refresh_doctor(&mut self) {
        self.last_doctor = Instant::now();
        let recent = |t: Option<Instant>| t.is_some_and(|t| t.elapsed() < Duration::from_secs(120));
        #[cfg(windows)]
        if self.facts.windows_probed.is_none_or(|t| t.elapsed() > Duration::from_secs(15)) {
            self.facts.windows_probed = Some(Instant::now());
            let tx = self.internal_tx.clone();
            tokio::task::spawn_blocking(move || {
                let _ = tx.blocking_send(Internal::WindowsFacts(windows_facts()));
            });
        }
        let mut connected_names: Vec<String> = self
            .peers
            .values()
            .filter(|p| self.trusts(&p.id) && self.link_for(&p.id).is_some())
            .map(|p| self.store.peer(&p.id).map(|t| t.name.clone()).unwrap_or_else(|| p.name.clone()))
            .collect();
        connected_names.sort();
        let paired_online = !connected_names.is_empty();
        let known_addrs = self
            .store
            .peers()
            .iter()
            .filter(|p| p.trusted && !p.disconnected)
            .flat_map(|p| p.addrs.iter().filter_map(|a| a.parse().ok()))
            .collect();
        let facts = crate::doctor::Facts {
            os: Os::current(),
            ifaces: self.ifaces.to_vec(),
            searching: self.search_started.elapsed(),
            peers_seen: self.peers.len(),
            paired_online,
            connected_names,
            known_addrs,
            known_addrs_silent: self.facts.known_silent,
            host_unreachable: recent(self.facts.host_unreachable),
            // Once a local dial has worked, the permission was on: an asleep
            // computer's "no route to host" later isn't blamed on it (Review 04)
            local_dial_ok: self.facts.local_dial_ok.is_some(),
            windows: self.facts.windows.clone(),
            receive_dir: self.store.settings().receive_dir.clone(),
            write_denied: self.facts.write_denied,
            remote_disconnected: self.facts.remote_disconnected.clone(),
            twin: recent(self.facts.twin),
        };
        let checks = crate::doctor::diagnose(&facts);
        if checks != self.doctor {
            self.doctor = checks;
            self.dirty = true;
        }
    }

    /// A live link's kind and rank, as seen from this side.
    fn link_rank(&self, link: &Link) -> (LinkKind, u8) {
        let info = link.info();
        paths::rank(self.link_kind(&info.remote, Some(info.local.ip()), None), &info.remote)
    }

    /// Fastest-looking path first: cable (IPv6 link-local first) > Ethernet > Wi-Fi > other.
    fn path_rank(&self, path: &PathEntry) -> (LinkKind, u8, std::cmp::Reverse<Instant>) {
        let (kind, order) = paths::rank(self.link_kind(&path.addr, None, path.if_index), &path.addr);
        (kind, order, std::cmp::Reverse(path.seen))
    }

    fn best_path<'a>(&self, p: &'a Peer) -> Option<&'a PathEntry> {
        p.paths.iter().min_by_key(|path| self.path_rank(path))
    }

    /// The live link to a peer, if any.
    /// The live link to a peer. For a paired peer, only a link whose TLS
    /// certificate matches the pinned key counts (a device id alone proves
    /// nothing: anyone can claim one).
    fn link_for(&self, id: &DeviceId) -> Option<Link> {
        let pinned = self.store.peer(id).filter(|t| t.trusted).map(|t| t.fingerprint);
        let usable = |l: &Link| {
            !l.is_closed() && &l.info().hello.id == id && pinned.is_none_or(|fp| l.info().peer_fp.ct_eq(&fp))
        };
        let preferred = self.peers.get(id)?.link.and_then(|l| self.links.get(&l)).filter(|l| usable(l));
        preferred.or_else(|| self.links.values().find(|l| usable(l))).cloned()
    }

    fn trusts(&self, id: &DeviceId) -> bool {
        self.store.peer(id).is_some_and(|p| p.trusted)
    }

    /// Trust belongs to a key, not to a claimed id: this link's certificate
    /// must be the one pinned for the id it claims.
    fn link_trusted(&self, link: &Link) -> bool {
        let info = link.info();
        self.store.peer(&info.hello.id).filter(|t| t.trusted).is_some_and(|t| info.peer_fp.ct_eq(&t.fingerprint))
    }

    /// A device answered with another protocol version: the UI says which
    /// computer to update. Only known devices are marked (anyone can claim an id).
    fn mark_incompatible(&mut self, id: &DeviceId, theirs: u32) {
        if self.link_for(id).is_some() {
            return;
        }
        if !self.peers.contains_key(id) {
            let Some(t) = self.store.peer(id) else { return };
            self.peers.insert(id.clone(), Peer::new(id.clone(), t.name.clone(), t.os));
        }
        if let Some(p) = self.peers.get_mut(id)
            && p.incompatible != Some(theirs)
        {
            p.incompatible = Some(theirs);
            self.dirty = true;
        }
    }

    /// After pairing (or on finding a mismatched link), cut every link that
    /// claims `id` but doesn't carry its pinned key.
    fn close_impostors(&mut self, id: &DeviceId) {
        let Some(pinned) = self.store.peer(id).filter(|t| t.trusted).map(|t| t.fingerprint) else { return };
        let impostors: Vec<Link> = self
            .links
            .values()
            .filter(|l| &l.info().hello.id == id && !l.info().peer_fp.ct_eq(&pinned))
            .cloned()
            .collect();
        for l in impostors {
            l.try_send(Msg::Error { code: ErrorCode::NotPaired, msg: "key does not match pairing".into() });
            l.close();
            self.say(
                "refuse",
                format!("Closed a connection that claimed to be {} but had another key", l.info().hello.name),
                Some(l.info().remote.to_string()),
                Some(Tone::Warn),
            );
        }
        if let Some(real) = self.link_for(id).map(|l| l.id())
            && let Some(peer) = self.peers.get_mut(id)
        {
            peer.link = Some(real);
        }
    }

    // ---- inputs ------------------------------------------------------------

    fn on_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Snapshot(reply) => {
                let _ = reply.send(self.snapshot());
            }
            Cmd::Connect(input, reply) => match parse_target(&input, &self.ifaces) {
                Err(msg) => {
                    let _ = reply.send(Err(msg));
                }
                Ok(addr) => {
                    self.say("connect", format!("Connecting to {addr}"), Some("typed by hand".into()), None);
                    self.spawn_dial(vec![addr], None, Purpose::Manual(reply));
                }
            },
            Cmd::Pair(peer, reply) => {
                let _ = reply.send(self.cmd_pair(peer));
            }
            Cmd::Pairing(action, reply) => {
                let _ = reply.send(self.cmd_pairing(action));
            }
            Cmd::RemovePeer(id) => self.cmd_remove_peer(&id),
            Cmd::RenamePeer(id, name) => self.cmd_rename_peer(&id, &name),
            Cmd::SetIncoming(id, value) => self.cmd_set_incoming(&id, value),
            Cmd::SetConnected(id, value) => self.cmd_set_connected(&id, value),
            Cmd::Settings(patch) => self.cmd_settings(patch),
            Cmd::Send(peer, paths, reply) => {
                let _ = reply.send(self.cmd_send(peer, paths));
            }
            Cmd::SendMany(peers, paths, reply) => {
                let _ = reply.send(self.cmd_send_many(peers, paths));
            }
            Cmd::Accept(tid, always) => self.cmd_accept(tid, always),
            Cmd::Decline(tid) => self.cmd_decline(tid),
            Cmd::Cancel(tid) => self.cmd_cancel(tid),
            Cmd::Dismiss(tid) => self.cmd_dismiss(tid),
            Cmd::SendClipboard(to, reply) => self.cmd_send_clipboard(to, reply),
            Cmd::SendText(to, text, private, reply) => {
                let _ = reply.send(self.cmd_send_text(to, text, private));
            }
            Cmd::SetClipSync(id, on) => self.set_clip_sync(&id, on, crate::util::now_ms()),
            Cmd::ClipSyncOffAll => self.clip_sync_off_all(),
            Cmd::PauseClipSync(wait) => self.pause_clip_sync(wait),
            Cmd::ResumeClipSync => self.resume_clip_sync(),
            Cmd::SetClipTo(id, to) => {
                if let Ok(true) = self.store.set_clip_to(&id, to) {
                    self.dirty = true;
                }
            }
            Cmd::SendAgain(tid, reply) => {
                let _ = reply.send(self.cmd_send_again(tid));
            }
            Cmd::CopyClip(id, reply) => self.cmd_copy_clip(id, reply),
            Cmd::CopyText(text, reply) => {
                self.copy_own(crate::clip::Contents { text: Some(text), ..Default::default() }, false, reply)
            }
            Cmd::ClearClips => self.cmd_clear_clips(),
            Cmd::RemoveClip(id) => self.cmd_remove_clip(id),
            Cmd::NetworkChanged => self.on_network_change("The network changed", Some("reported by the app".into())),
            Cmd::RetryDiscovery => {
                self.search_started = Instant::now();
                // Look at the firewall again too (after "Fix it", say).
                #[cfg(windows)]
                {
                    self.facts.windows_probed = None;
                }
                self.reach.start_all(Instant::now());
                self.restart_sweeps();
                self.sweep.asked = Some(Instant::now());
                if let Some(d) = &self.discovery {
                    d.announce_now();
                }
                self.say("search", "Looking again", None, Some(Tone::Dim));
                self.maintain_links();
                self.dirty = true;
            }
            Cmd::Shutdown(..) => unreachable!("handled in run"),
            Cmd::Continue(tid) => self.continue_held(tid),
            Cmd::UnqueueNext(id) => self.cmd_unqueue_next(id),
            Cmd::ShelfAdd(name, text, private, reply) => {
                let _ = reply.send(self.shelf_add(name, text, private).map(|_| ()).map(|_| String::new()));
            }
            Cmd::ShelfEdit(id, name, text, reply) => {
                let _ = reply.send(self.shelf_edit(&id, name, text));
            }
            Cmd::ShelfRemove(ids) => self.shelf_remove(ids),
            Cmd::ShelfUndo(reply) => {
                let _ = reply.send(self.shelf_undo());
            }
            Cmd::ShelfCopy(id, reply) => {
                let _ = reply.send(self.shelf_copy(&id));
            }
            Cmd::ShelfReveal(id, reply) => {
                let _ = reply.send(self.shelf_text(&id).map(|(t, _)| t));
            }
            Cmd::SetShelfShare(id, on) => self.set_shelf_share(&id, on),
            Cmd::ShelfPutMarked(count, reply) => {
                let _ = reply.send(self.shelf_put_marked(count));
            }
            Cmd::ShelfPutReceived(id, reply) => {
                let _ = reply.send(self.shelf_put_received(id));
            }
            Cmd::SaveClipImage(id, name, reply) => {
                let _ = reply.send(self.save_clip_image(id, &name));
            }
            Cmd::DismissClipNotice => {
                self.clip.notice = None;
                self.dirty = true;
            }
            Cmd::SetPicture(p, reply) => {
                let r = match p.as_ref().map(crate::picture::check) {
                    Some(Err(e)) => Err(format!("That picture can’t be used ({e:?}).")),
                    _ => self.store.update_settings(|s| s.picture = p.clone()).map(|_| ()).map_err(|e| e.to_string()),
                };
                if r.is_ok() {
                    self.send_picture_all();
                    self.dirty = true;
                }
                let _ = reply.send(r);
            }
            Cmd::ContinueNext(peer) => self.cmd_continue_next(peer),
        }
    }

    fn on_sighting(&mut self, s: Sighting) {
        if self.cfg.fault_blind {
            return;
        }
        if s.id == self.identity.device_id {
            // Discovery only passes our own id on when it's another copy of
            // the app on this computer (the old app still running).
            let first = self.facts.twin.is_none_or(|t| t.elapsed() > Duration::from_secs(60));
            self.facts.twin = Some(Instant::now());
            if first {
                let what = format!("port {}, protocol {}", s.addr.port(), s.proto);
                self.say(
                    "twin",
                    format!("Another copy of {APP_NAME} is open on this computer"),
                    Some(what),
                    Some(Tone::Warn),
                );
                self.refresh_doctor();
            }
            return;
        }
        let new = !self.peers.contains_key(&s.id);
        let peer = self.peers.entry(s.id.clone()).or_insert_with(|| Peer::new(s.id.clone(), s.name.clone(), s.os));
        peer.name = s.name.clone();
        peer.os = s.os;
        // A live link proves the versions match; a stale sighting doesn't undo that.
        if peer.link.is_none() {
            peer.incompatible = (s.proto != 0 && s.proto != PROTOCOL_VERSION).then_some(s.proto);
        }
        let known_path = peer.paths.iter().any(|p| p.addr == s.addr);
        peer.saw(s.addr, s.via, s.if_index);
        // Seen just now: a paired device we're not connected to is dialled at
        // once, known address or not (it may have just opened again).
        let wanted = self.store.peer(&s.id).is_some_and(|t| t.trusted && !t.disconnected);
        if wanted && self.link_for(&s.id).is_none() && !self.dialing.contains(&s.id) {
            self.reach.soon(&s.id, Instant::now());
            self.maintain_links();
        }
        if new || !known_path {
            let via = match s.via {
                Via::Mdns => "mDNS",
                Via::Broadcast => "broadcast",
                Via::Known => "known address",
                Via::Manual => "typed address",
                Via::Inbound => "incoming",
            };
            let detail = format!("{} via {via}", s.addr);
            if new {
                self.say("found", format!("Found {} on this network", s.name), Some(detail), None);
            } else {
                self.tech("found", format!("{} also at {detail}", s.name));
            }
            self.dirty = true;
        }
    }

    fn on_internal(&mut self, i: Internal) {
        match i {
            Internal::WindowsFacts(w) => {
                self.facts.windows = Some(w);
                self.refresh_doctor();
            }
            Internal::Planned(tid, result) => self.on_planned(tid, result),
            Internal::Sent(tid, outcome) => self.on_sent(tid, outcome),
            Internal::Received(tid, outcome) => self.on_received(tid, outcome),
            Internal::Restored(tid, restored) => self.on_restored(tid, *restored),
            Internal::ClipRead(who, read) => self.on_clip_read(who, read),
            Internal::ClipWritten { from, from_id, contents, auto, private, written } => {
                self.on_clip_written(from, from_id, contents, auto, private, written)
            }
            Internal::ClipCopied(count) => self.on_clip_copied(count),
            Internal::Accepted(Ok(link), _) => {
                let _ = self.adopt(link, Via::Inbound, None);
            }
            Internal::Accepted(Err(HandshakeError::TurnedAway), from) => {
                tracing::debug!("turned away {from}: disconnected here");
            }
            Internal::Accepted(Err(HandshakeError::Incompatible { theirs, name, id }), from) => {
                self.say(
                    "refuse",
                    format!("{name} runs another version of {APP_NAME}, so they can’t connect"),
                    Some(format!("from {from}, protocol {theirs}")),
                    Some(Tone::Warn),
                );
                self.mark_incompatible(&id, theirs);
            }
            Internal::Accepted(Err(e), from) => {
                self.tech("refuse", format!("incoming from {from}: {e}"));
            }
            Internal::Dialed { addr, purpose, result } => self.on_dialed(addr, purpose, result),
            Internal::Ifaces(list) => self.on_ifaces(list),
        }
    }

    fn on_dialed(&mut self, addr: SocketAddr, purpose: Purpose, result: Result<Link, DialFailure>) {
        // Facts for the doctor: can we reach local addresses at all?
        if is_local(&addr.ip()) {
            match &result {
                Ok(_) => self.facts.local_dial_ok = Some(Instant::now()),
                Err(DialFailure::NoAnswer(
                    std::io::ErrorKind::HostUnreachable | std::io::ErrorKind::NetworkUnreachable,
                )) => self.facts.host_unreachable = Some(Instant::now()),
                Err(_) => {}
            }
        }
        if let Purpose::Reconnect(_) = &purpose {
            self.facts.known_silent = result.is_err();
        }
        match (purpose, result) {
            (Purpose::Pair(id), Ok(link)) => {
                let adopted = self.adopt(link.clone(), Via::Manual, Some(addr)).ok();
                self.pair_link_ready(&id, adopted.map(|_| link));
            }
            (Purpose::Pair(id), Err(failure)) => {
                self.say(
                    "pair",
                    format!("Couldn’t reach {} to pair", addr.ip()),
                    Some(format!("{addr}: {failure:?}")),
                    Some(Tone::Warn),
                );
                self.pair_link_ready(&id, None);
                if local_network_blocked(Os::current(), &addr, &failure)
                    && let Some(s) = self.pairing.as_mut().filter(|s| s.peer == id)
                {
                    s.failed = Some(format!(
                        "macOS is blocking {APP_NAME} from your network, so this Mac can’t reach {0}. Turn {APP_NAME} on in System Settings → Privacy & Security → Local Network, or start pairing from {0} instead.",
                        s.peer_name
                    ));
                }
            }
            (Purpose::Reconnect(id), Ok(link)) => {
                let ok = link.info().hello.id == id && self.adopt(link, Via::Known, Some(addr)).is_ok();
                self.reconnect_done(&id, ok);
            }
            (Purpose::Reconnect(id), Err(failure)) => {
                tracing::debug!("reconnect to {id:?} failed: {failure:?}");
                self.reconnect_done(&id, false);
                if self.link_for(&id).is_none() {
                    self.reach.failed(&id, reach_failure(&addr, &failure));
                    self.dirty = true;
                }
                if let DialFailure::Incompatible { theirs, id: theirs_id, .. } = &failure
                    && theirs_id == &id
                {
                    self.mark_incompatible(&id, *theirs);
                }
                if let DialFailure::Refused(ErrorCode::Disconnected) = failure {
                    let name = self.store.peer(&id).map(|p| p.name.clone()).unwrap_or_default();
                    self.on_remote_disconnected(&id, &name);
                }
            }
            (Purpose::Manual(reply), Ok(link)) => {
                let outcome = self.adopt(link, Via::Manual, Some(addr)).map(|id| {
                    let peer = self.peers.get(&id).expect("adopted peer exists");
                    self.peer_view(peer)
                });
                let _ = reply.send(outcome);
            }
            (Purpose::Manual(reply), Err(failure)) => {
                if let DialFailure::Incompatible { theirs, id, .. } = &failure {
                    self.mark_incompatible(id, *theirs);
                }
                let text = match &failure {
                    _ if local_network_blocked(Os::current(), &addr, &failure) => format!(
                        "macOS is blocking {APP_NAME} from your network. Turn {APP_NAME} on in System Settings → Privacy & Security → Local Network, or use Connect by IP on the other computer to reach this Mac instead."
                    ),
                    DialFailure::NoAnswer(_) => format!(
                        "Nothing answered at {}. Check the address, and that {APP_NAME} is open on the other computer.",
                        addr.ip()
                    ),
                    DialFailure::Incompatible { theirs, name, .. } => update_advice(*theirs, name),
                    DialFailure::Refused(code) => format!("{} refused the connection ({code:?}).", addr.ip()),
                    DialFailure::NotPortal(_) => format!("Something answered at {}, but it isn’t {APP_NAME}.", addr),
                };
                self.say(
                    "connect",
                    format!("Couldn’t connect to {addr}"),
                    Some(format!("{failure:?}")),
                    Some(Tone::Warn),
                );
                let _ = reply.send(Err(text));
            }
        }
    }

    fn on_link_event(&mut self, ev: LinkEvent) {
        match ev {
            LinkEvent::Msg(id, msg) if !self.links.contains_key(&id) => {
                let (_, msgs) = self.early.entry(id).or_insert_with(|| (Instant::now(), Vec::new()));
                if msgs.len() < EARLY_MAX {
                    msgs.push(msg);
                }
            }
            LinkEvent::Msg(id, msg) => self.on_msg(id, msg),
            LinkEvent::Closed(id, reason) => {
                self.early.remove(&id);
                let Some(link) = self.links.remove(&id) else { return };
                self.link_born.remove(&id);
                self.forget_clip_link(id);
                self.probing.remove(&id);
                let peer_id = link.info().hello.id.clone();
                let replacement = self.link_for(&peer_id).map(|l| l.id());
                if let Some(peer) = self.peers.get_mut(&peer_id)
                    && peer.link == Some(id)
                {
                    peer.link = replacement;
                    self.dirty = true;
                }
                // A paired device that's gone: connecting to it starts over
                // (the UI says "Connecting…", not "isn't reachable", at first).
                if replacement.is_none() && self.link_for(&peer_id).is_none() && self.wants_link(&peer_id) {
                    self.reach.start(&peer_id, Instant::now());
                }
                self.say(
                    "link",
                    format!("The connection to {} closed", link.info().hello.name),
                    Some(reason.to_string()),
                    Some(Tone::Dim),
                );
            }
        }
    }

    fn on_msg(&mut self, link_id: LinkId, msg: Msg) {
        let Some(link) = self.links.get(&link_id).cloned() else { return };
        let peer_id = link.info().hello.id.clone();
        let name = link.info().hello.name.clone();
        match msg {
            Msg::Status { role, name, accepts } => {
                if self.trusts(&peer_id) && !self.link_trusted(&link) {
                    return; // not the paired device's key: ignore its claims
                }
                // A Mission 01 build says Send when it won't take files
                let accepts = accepts.or((role == Role::Send).then_some(Incoming::Never));
                if let Some(p) = self.peers.get_mut(&peer_id) {
                    p.accepts = accepts;
                    p.name = name.clone();
                }
                if self.link_trusted(&link) && self.store.peer(&peer_id).is_some_and(|t| !t.renamed && t.name != name) {
                    let _ = self.store.update_peer(&peer_id, |t| t.name = name);
                }
                self.dirty = true;
                self.after_peer_accepts(&peer_id, accepts);
            }
            Msg::Welcome { paired } if self.link_trusted(&link) => self.on_welcome(&peer_id, &name, paired),
            Msg::Welcome { .. } => {}
            Msg::PairAsk
            | Msg::PairReady
            | Msg::PairKey { .. }
            | Msg::PairCheck { .. }
            | Msg::PairWrong { .. }
            | Msg::PairDone
            | Msg::PairStop => self.on_pair_msg(&link, msg),
            Msg::Error { code: ErrorCode::RateLimited, .. } => self.on_pair_msg(&link, msg),
            Msg::Error { code: ErrorCode::Disconnected, .. } if self.link_trusted(&link) => {
                let name = self.store.peer(&peer_id).map(|p| p.name.clone()).unwrap_or(name);
                self.on_remote_disconnected(&peer_id, &name);
            }
            Msg::Error { code, msg } => {
                self.say(
                    "error",
                    format!("{name} reported a problem"),
                    Some(format!("{code:?}: {msg}")),
                    Some(Tone::Dim),
                );
            }
            Msg::Superseded => {}
            other if !self.link_trusted(&link) => {
                if let Msg::Offer(o) = &other {
                    link.try_send(Msg::Decline { tid: o.tid, reason: crate::proto::DeclineReason::NotPaired });
                }
                // Brief §8: never accept anything else from an unpaired device.
                tracing::debug!("refusing {other:?} from unpaired {name}");
                link.try_send(Msg::Error { code: ErrorCode::NotPaired, msg: "pair first".into() });
            }
            Msg::Offer(offer) => self.on_offer(&link, offer),
            msg @ Msg::Clip { .. } => self.on_clip_part(&link, msg),
            Msg::ClipSync { on, at } => self.on_clip_sync(&link, on, at),
            msg @ (Msg::ResumeAsk { .. } | Msg::ResumeFrom { .. }) => self.on_resume_msg(&link, msg),
            Msg::Hold { tid, on } => self.on_hold(&link, tid, on),
            Msg::QueueHead { at } => self.on_queue_head(link.info().hello.id.clone(), at),
            Msg::ShelfShare { on, .. } => self.on_shelf_share(&link, on),
            Msg::Shelf { items } => self.on_shelf(&link, items),
            Msg::Picture { picture } => self.on_picture(&link, picture),
            Msg::Kept { tid, bytes } => self.on_kept(&link, tid, bytes),
            msg @ (Msg::Cancel { tid, .. } | Msg::Complete { tid, .. })
                if xfer::find(&self.transfers, tid)
                    .is_some_and(|t| t.phase == crate::events::TransferPhase::Paused) =>
            {
                self.on_paused_msg(&link, msg)
            }
            msg @ (Msg::Accept { .. } | Msg::Decline { .. } | Msg::Cancel { .. }) => self.on_transfer_msg(&link, msg),
            other => {
                tracing::debug!("unhandled message from {name}: {other:?}");
            }
        }
    }

    fn on_tick(&mut self) {
        // Early messages for a link that was never adopted
        self.early.retain(|_, (since, _)| since.elapsed() < EARLY_KEEP);
        // A slot that came free: what's next goes (Mission 05 §1.5).
        self.tell_next_all();
        self.start_next();
        if !self.cfg.loopback_only && self.last_ifaces.elapsed() >= IFACE_EVERY {
            self.last_ifaces = Instant::now();
            let tx = self.internal_tx.clone();
            tokio::spawn(async move {
                if let Ok(list) = tokio::task::spawn_blocking(iface::snapshot).await {
                    let _ = tx.send(Internal::Ifaces(list)).await;
                }
            });
        }
        self.watch_network();
        // What the window says ("isn't reachable"), Details says too, once
        for (id, why) in self.reach.newly_unreachable() {
            let Some(name) = self.store.peer(&id).filter(|p| p.trusted).map(|p| p.name.clone()) else { continue };
            self.say("link", format!("{name} isn’t reachable; trying again by itself"), why, Some(Tone::Warn));
        }
        self.maybe_sweep();
        self.maintain_links();
        self.upgrade_links();
        self.pairing_tick();
        self.close_idle_untrusted();
        self.resume_tick();
        self.clip_tick();
        if self.last_doctor.elapsed() >= Duration::from_secs(2) {
            self.refresh_doctor();
        }
        // Forget stale paths, and nearby (untrusted) devices nobody has seen lately.
        let before: usize = self.peers.values().map(|p| p.paths.len()).sum();
        for p in self.peers.values_mut() {
            p.paths.retain(|path| path.seen.elapsed() < PATH_TTL);
        }
        let store = &self.store;
        self.peers.retain(|id, p| p.link.is_some() || !p.paths.is_empty() || store.peer(id).is_some());
        let after: usize = self.peers.values().map(|p| p.paths.len()).sum();
        if before != after {
            self.dirty = true;
        }
    }

    // ---- links -------------------------------------------------------------

    /// Tries the addresses in order, each one a moment after the last
    /// (without waiting for it to fail), and reports the first that answers,
    /// or the last failure. Extra connections that also made it are closed.
    fn spawn_dial(&self, candidates: Vec<SocketAddr>, pin: Option<Fingerprint>, purpose: Purpose) {
        let client = match crate::tls::client_config(&self.identity, pin) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("client TLS config: {e}");
                return;
            }
        };
        let hello = self.hello.read().unwrap_or_else(|e| e.into_inner()).clone();
        let targets: Vec<_> = candidates.into_iter().map(|a| (a, local_bind_for(&self.ifaces, &a))).collect();
        let link_tx = self.link_tx.clone();
        let internal = self.internal_tx.clone();
        tokio::spawn(async move {
            let attempt = |(addr, local): (SocketAddr, Option<dial::LocalBind>)| {
                let (client, hello, link_tx) = (client.clone(), hello.clone(), link_tx.clone());
                async move { (addr, dial_one(addr, local, client, hello, link_tx).await) }
            };
            let mut queue = targets.into_iter();
            let mut running = tokio::task::JoinSet::new();
            let mut last = None;
            loop {
                if running.is_empty() {
                    match queue.next() {
                        Some(t) => {
                            running.spawn(attempt(t));
                        }
                        None => break,
                    }
                }
                tokio::select! {
                    done = running.join_next() => match done {
                        Some(Ok((addr, Ok(link)))) => {
                            running.abort_all();
                            while let Some(extra) = running.join_next().await {
                                if let Ok((_, Ok(extra))) = extra {
                                    extra.close();
                                }
                            }
                            let _ = internal.send(Internal::Dialed { addr, purpose, result: Ok(link) }).await;
                            return;
                        }
                        Some(Ok((addr, Err(e)))) => last = Some((addr, e)),
                        Some(Err(_)) | None => {}
                    },
                    _ = tokio::time::sleep(DIAL_STAGGER), if queue.len() > 0 => {
                        if let Some(t) = queue.next() {
                            running.spawn(attempt(t));
                        }
                    }
                }
            }
            if let Some((addr, e)) = last {
                let _ = internal.send(Internal::Dialed { addr, purpose, result: Err(e) }).await;
            }
        });
    }

    /// Takes ownership of a fresh link. Returns the peer's id, or a sentence
    /// explaining why the link was refused.
    /// Takes a new link in, then handles what it said before it was taken in.
    fn adopt(&mut self, link: Link, via: Via, dialed: Option<SocketAddr>) -> Result<DeviceId, String> {
        let id = link.id();
        let adopted = self.adopt_link(link, via, dialed);
        if let Some((_, msgs)) = self.early.remove(&id)
            && self.links.contains_key(&id)
        {
            for msg in msgs {
                self.on_msg(id, msg);
            }
        }
        adopted
    }

    fn adopt_link(&mut self, link: Link, via: Via, dialed: Option<SocketAddr>) -> Result<DeviceId, String> {
        let info = link.info().clone();
        let hello = &info.hello;
        if hello.id == self.identity.device_id {
            link.close();
            return Err("That’s this device’s own address. Type the other computer’s address.".into());
        }
        if let Some(trusted) = self.store.peer(&hello.id).filter(|t| t.trusted)
            && !info.peer_fp.ct_eq(&trusted.fingerprint)
        {
            link.try_send(Msg::Error { code: ErrorCode::KeyChanged, msg: "key does not match pairing".into() });
            link.close();
            self.say(
                "refuse",
                format!("{}’s security key changed since pairing, so it can’t connect. Remove it in Settings and pair again", hello.name),
                Some(format!("security key changed, from {}", info.remote)),
                Some(Tone::Err),
            );
            return Err(format!("{}’s security key has changed. Remove it in Settings and pair again.", hello.name));
        }
        // Disconnected here: stay paired, but no link (a dial or an incoming
        // connection that was already under way when Disconnect was pressed).
        if let Some(t) = self.store.peer(&hello.id).filter(|t| t.trusted && t.disconnected) {
            link.try_send(Msg::Error { code: ErrorCode::Disconnected, msg: "disconnected on the other side".into() });
            link.close();
            return Err(format!("You disconnected from {}. Press Connect first.", t.name));
        }
        // Unpaired devices may only pair: cap how many connections they hold.
        if !self.trusts(&hello.id) {
            let untrusted: Vec<&Link> =
                self.links.values().filter(|l| !l.is_closed() && !self.link_trusted(l)).collect();
            let same_addr = untrusted.iter().filter(|l| l.info().remote.ip() == info.remote.ip()).count();
            if same_addr >= MAX_UNTRUSTED_PER_ADDR || untrusted.len() >= MAX_UNTRUSTED {
                link.try_send(Msg::Error { code: ErrorCode::RateLimited, msg: "too many connections".into() });
                link.close();
                tracing::debug!("refused an unpaired connection from {}: too many", info.remote);
                return Err("Too many connections from that address. Try again in a moment.".into());
            }
        }
        let addr = dialed.unwrap_or_else(|| with_port(info.remote, hello.port));
        let if_index = match addr {
            SocketAddr::V6(v6) if v6.scope_id() != 0 => Some(v6.scope_id()),
            _ => None,
        };
        let is_leader = self.identity.device_id < hello.id;
        let peer = self
            .peers
            .entry(hello.id.clone())
            .or_insert_with(|| Peer::new(hello.id.clone(), hello.name.clone(), hello.os));
        peer.name = hello.name.clone();
        peer.os = hello.os;
        peer.accepts = (hello.role == Role::Send).then_some(Incoming::Never);
        peer.incompatible = None;
        peer.saw(addr, via, if_index);
        // Duplicates are only links with the same key; a different key claiming
        // the same id is never allowed to replace (or be replaced by) another.
        let old = peer
            .link
            .and_then(|id| self.links.get(&id))
            .filter(|l| !l.is_closed() && l.info().peer_fp.ct_eq(&info.peer_fp))
            .cloned();
        let previous_kind = old.as_ref().map(|l| self.link_rank(l).0);
        let mut keep_new = true;
        if let Some(old) = old {
            // Two links to one peer (both dialled, or an upgrade): the smaller
            // device id decides, keeping the better path (ties keep the newest).
            let busy = self.transfers.iter().any(|t| t.active() && t.link == Some(old.id()));
            if is_leader {
                keep_new = !busy && paths::keep_new_link(self.link_rank(&old), self.link_rank(&link));
                let loser = if keep_new { &old } else { &link };
                loser.try_send(Msg::Superseded);
                loser.close();
            }
        }
        let peer = self.peers.get_mut(&hello.id).expect("peer inserted above");
        if keep_new {
            peer.link = Some(link.id());
        }
        let name = hello.name.clone();
        let id = hello.id.clone();
        let trusted = self.link_trusted(&link);
        link.try_send(Msg::Welcome { paired: trusted });
        let link_id_for_status = link.id();
        self.link_born.insert(link.id(), Instant::now());
        self.links.insert(link.id(), link);
        if trusted {
            if self.facts.remote_disconnected.as_deref() == self.store.peer(&id).map(|p| p.name.as_str()) {
                self.facts.remote_disconnected = None;
                self.refresh_doctor();
            }
            self.reach.connected(&id);
            let _ = self.store.remember_addr(&id, &addr.to_string());
            if self.store.peer(&id).is_some_and(|t| !t.renamed && t.name != name) {
                let _ = self.store.update_peer(&id, |t| t.name = name.clone());
            }
            if let Some(link) = self.links.get(&link_id_for_status) {
                self.send_status(link);
            }
            if let Some(link) = self.links.get(&link_id_for_status) {
                self.send_clip_sync(link);
                self.shelf_on_link(&link.clone());
            }
            self.peer_back(&id);
            self.tell_next_on_link(&id);
            self.start_next();
        }
        let kind = self.link_kind(&info.remote, Some(info.local.ip()), if_index);
        let over = match kind {
            LinkKind::Cable => "a direct cable",
            LinkKind::Ethernet => "Ethernet",
            LinkKind::Wifi => "Wi-Fi",
            LinkKind::Other => "the network",
        };
        // "Connected" only for a paired device: anyone else is just reachable
        let text = if trusted {
            format!("Connected to {name} over {over}")
        } else {
            format!("{name} can be reached over {over}; not paired")
        };
        self.say(
            "link",
            text,
            Some(format!("{}, {}", info.remote, if info.dialer { "dialled" } else { "incoming" })),
            None,
        );
        if keep_new && kind == LinkKind::Cable && previous_kind.is_some_and(|k| k != LinkKind::Cable) {
            self.say("link", format!("Switched to the direct cable to {name}"), None, None);
        }
        self.dirty = true;
        Ok(id)
    }
}

/// Which computer needs updating when a device speaks another protocol
/// version (0: an unknown, older one).
pub fn update_advice(theirs: u32, name: &str) -> String {
    if theirs < PROTOCOL_VERSION {
        format!("Update {APP_NAME} on {name}. It runs an older version that can’t talk to this one.")
    } else {
        let me = match Os::current() {
            Os::Macos => "this Mac",
            Os::Windows => "this PC",
            _ => "this computer",
        };
        format!("Update {APP_NAME} on {me}. {name} runs a newer version that can’t talk to this one.")
    }
}

/// ("Wi-Fi" | "Ethernet", address) for each routable IPv4 address on a
/// Wi-Fi or wired interface (not 169.254: that's no use to type in).
pub(crate) fn lan_addresses(ifaces: &[Iface]) -> Vec<(&'static str, std::net::Ipv4Addr)> {
    let mut out = Vec::new();
    for i in ifaces {
        let label = match i.kind {
            iface::IfaceKind::Wifi => "Wi-Fi",
            iface::IfaceKind::Ethernet => "Ethernet",
            _ => continue,
        };
        out.extend(i.v4.iter().filter(|(ip, _)| !ip.is_link_local()).map(|(ip, _)| (label, *ip)));
    }
    out
}

/// Why a dial to a paired device failed, in plain words.
fn reach_failure(addr: &SocketAddr, f: &DialFailure) -> String {
    use std::io::ErrorKind::*;
    let ip = addr.ip();
    match f {
        DialFailure::NoAnswer(ConnectionRefused) => format!("{APP_NAME} isn’t answering at {ip}"),
        DialFailure::NoAnswer(HostUnreachable | NetworkUnreachable) => format!("{ip} can’t be reached from here"),
        DialFailure::NoAnswer(_) => format!("No answer from {ip}"),
        DialFailure::Incompatible { .. } => format!("It runs another version of {APP_NAME}"),
        DialFailure::Refused(_) => "It turned the connection down".into(),
        DialFailure::NotPortal(_) => format!("Something else answered at {ip}"),
    }
}

/// On macOS, "no route to host" to an address on the local network is how a
/// denied Local Network permission shows up (TN3179 has no API to ask).
fn local_network_blocked(os: Os, addr: &SocketAddr, failure: &DialFailure) -> bool {
    os == Os::Macos
        && is_local(&addr.ip())
        && !addr.ip().is_loopback()
        && matches!(failure, DialFailure::NoAnswer(std::io::ErrorKind::HostUnreachable))
}

/// Private, link-local or unique-local: an address on "your network".
fn is_local(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback(),
        IpAddr::V6(v6) => iface::is_link_local(ip) || (v6.segments()[0] & 0xfe00) == 0xfc00 || v6.is_loopback(),
    }
}

fn with_port(addr: SocketAddr, port: u16) -> SocketAddr {
    let mut a = addr;
    a.set_port(port);
    a
}

/// One outgoing connection: TCP, TLS (pinned when `client` says so), hellos.
async fn dial_one(
    addr: SocketAddr,
    local: Option<dial::LocalBind>,
    client: Arc<rustls::ClientConfig>,
    hello: Hello,
    link_tx: mpsc::Sender<LinkEvent>,
) -> Result<Link, DialFailure> {
    let tcp = dial::dial(addr, local, DIAL_TIMEOUT).await.map_err(|e| DialFailure::NoAnswer(e.kind()))?;
    let tls = link::connect_tls(tcp, client).await.map_err(|e| match e {
        crate::Error::Io(io) => DialFailure::NoAnswer(io.kind()),
        other => DialFailure::NotPortal(other.to_string()),
    })?;
    link::handshake(tls, true, hello, link_tx).await.map_err(DialFailure::from)
}

/// IPv4 link-local dials leave through the interface that owns a 169.254
/// address (see `net::dial`).
fn local_bind_for(ifaces: &[Iface], addr: &SocketAddr) -> Option<dial::LocalBind> {
    let SocketAddr::V4(v4) = addr else { return None };
    if !v4.ip().is_link_local() {
        return None;
    }
    let iface = iface::iface_for(ifaces, addr, None)?;
    let (ip, _) = iface.v4.iter().find(|(a, _)| a.is_link_local())?;
    Some(dial::LocalBind { ip: IpAddr::V4(*ip), if_index: iface.index })
}

async fn accept_loop(
    listeners: listen::Listeners,
    server_tls: Arc<rustls::ServerConfig>,
    hello: Arc<RwLock<Hello>>,
    link_tx: mpsc::Sender<LinkEvent>,
    internal: mpsc::Sender<Internal>,
    turned_away: Arc<RwLock<HashSet<Fingerprint>>>,
) {
    loop {
        let (tcp, from) = match listeners.accept().await {
            Ok(x) => x,
            Err(e) => {
                tracing::warn!("accept failed: {e}");
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
        };
        let _ = dial::tune(&tcp);
        let (server_tls, link_tx, internal) = (server_tls.clone(), link_tx.clone(), internal.clone());
        let mine = hello.read().unwrap_or_else(|e| e.into_inner()).clone();
        let turned_away = turned_away.clone();
        tokio::spawn(async move {
            let result = async {
                let tls = link::accept_tls(tcp, server_tls).await?;
                // A paired device disconnected here hears so before any hello.
                let blocked = link::peer_fingerprint(&tls)
                    .is_some_and(|fp| turned_away.read().unwrap_or_else(|e| e.into_inner()).contains(&fp));
                if blocked {
                    link::refuse(tls, ErrorCode::Disconnected, "disconnected on the other side").await;
                    return Err(HandshakeError::TurnedAway);
                }
                link::handshake(tls, false, mine, link_tx).await
            }
            .await;
            let _ = internal.send(Internal::Accepted(result, from)).await;
        });
    }
}

/// Windows Firewall and network-profile facts (blocking COM calls).
#[cfg(windows)]
fn windows_facts() -> crate::doctor::WindowsFacts {
    use crate::doctor::Category;
    use portal_sys::windows::{self as win, NetCategory};
    let profiles = win::network_profiles()
        .map(|list| {
            list.into_iter()
                .map(|p| {
                    let c = match p.category {
                        NetCategory::Public => Category::Public,
                        NetCategory::Private => Category::Private,
                        NetCategory::Domain => Category::Domain,
                    };
                    (p.name, c)
                })
                .collect()
        })
        .unwrap_or_default();
    let rules = std::env::current_exe().ok().and_then(|exe| win::firewall_rules_for(&exe).ok());
    crate::doctor::WindowsFacts {
        profiles,
        allow_profiles: rules.as_ref().map(|r| r.allow_profiles),
        block_rules: rules.map(|r| r.block).unwrap_or_default(),
        cfa: win::controlled_folder_access(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn only_no_route_to_a_local_address_on_a_mac_reads_as_the_local_network_permission() {
        let home: SocketAddr = "192.168.1.31:47920".parse().unwrap();
        let unreachable = DialFailure::NoAnswer(ErrorKind::HostUnreachable);
        assert!(local_network_blocked(Os::Macos, &home, &unreachable));
        assert!(!local_network_blocked(Os::Windows, &home, &unreachable), "Windows has no such permission");
        assert!(!local_network_blocked(Os::Macos, &home, &DialFailure::NoAnswer(ErrorKind::TimedOut)));
        assert!(!local_network_blocked(Os::Macos, &"81.2.69.160:47920".parse().unwrap(), &unreachable));
        assert!(!local_network_blocked(Os::Macos, &"127.0.0.1:47920".parse().unwrap(), &unreachable));
    }
}
