//! Transfers on the node: offer → accept → stream → done. Any number at
//! once across devices, at most one active (preparing, offered, running or
//! paused) per device (Mission 04 §3). Every command names its transfer.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::watch;

use super::{Internal, Node};
use crate::consts::{APP_NAME, INCOMING_DIR};
use crate::events::{
    Block, Direction, Event, ItemView, LinkKind, ListedItem, Notice, NoticeKind, ProgressView, ResultView, Tone,
    TransferPhase, TransferView,
};
use crate::model::{DeviceId, Incoming, Os};
use crate::proto::{CancelReason, DeclineReason, ItemKind, Msg, Offer, Summary, TransferId};
use crate::transfer::journal::{Ending, InState, Saved};
use crate::transfer::plan::{self, Planned};
use crate::transfer::{Failure, LISTED, Progress, ProgressState, SPACE_MARGIN, recv};
use crate::util::random_bytes;

/// Nothing moved for this long while running: show that we're waiting.
/// Nothing moved for this long: the window says it's waiting (the estimator
/// hides the speed at the same moment, `estimate::STOPPED`).
const STALLED_AFTER: Duration = crate::transfer::estimate::STOPPED;

pub(super) struct Transfer {
    pub tid: TransferId,
    pub dir: Direction,
    pub peer: DeviceId,
    pub peer_name: String,
    pub link: Option<crate::net::link::LinkId>,
    pub offer: Option<Offer>,
    pub planned: Option<Planned>,
    /// Sender: what was dropped, so a blocked send can start again.
    pub paths: Vec<PathBuf>,
    pub phase: TransferPhase,
    pub block: Option<Block>,
    pub auto_accepted: bool,
    started: Instant,
    elapsed: Option<Duration>,
    pub cancel: Option<watch::Sender<bool>>,
    cancelled_here: bool,
    pub progress: Progress,
    result: Option<ResultView>,
    pub failure: Option<String>,
    pub link_kind: Option<LinkKind>,
    /// Speed and time left (restarted every time it runs, so a resume's jump
    /// never counts as speed).
    estimate: Option<crate::transfer::estimate::Estimator>,
    /// The time base for `estimate`.
    created: Instant,
    /// When 10 % and 90 % of the bytes had moved (speed over the middle 80 %,
    /// the way the brief measures it).
    marks: (Option<Instant>, Option<Instant>),
    /// When the byte count last changed, and to what.
    moved: (Instant, u64),
    pub stalled: bool,
    /// Keeps the computer from idle sleep while running.
    pub awake: Option<portal_sys::power::KeepAwake>,
    /// Unix seconds when it paused (it continues when the device is back).
    pub paused_at: Option<u64>,
    /// Time spent paused, left out of the elapsed time.
    paused_for: Duration,
    /// Receiver: checking what already arrived before continuing.
    pub resuming: bool,
    /// When this side last asked to continue (asked again until answered).
    pub asked: Option<Instant>,
    /// Receiver: the `resume_from` it sent, repeated if the sender asks again.
    pub resume_from: Option<Msg>,
    /// Sender: the receiver asked to continue on a new link while the run
    /// on the old one was still ending: (link, where to start, bytes it has).
    pub pending: Option<(crate::net::link::LinkId, crate::transfer::send::Start, u64)>,
    /// Sentences for the done screen (a file sent again, say).
    pub notes: Vec<String>,
    /// Bytes that had already arrived when it last continued after a pause.
    pub kept: Option<u64>,
    /// Receiver: the folder its files are saved in (said before they arrive)
    pub save_to: Option<PathBuf>,
    /// Quit or crashed here mid-transfer: waits for Continue or Cancel.
    pub stopped_here: bool,
    /// The other computer was quit or crashed: waits for it to continue.
    pub waiting_there: bool,
    /// False while the bytes kept are a guess (a sender after a crash).
    pub percent_known: bool,
    /// Started from what was next for this device: refused as busy, it
    /// goes back to the front.
    pub from_next: Option<super::next::NextUp>,
}

impl Transfer {
    pub(super) fn new(tid: TransferId, dir: Direction, peer: DeviceId, peer_name: String) -> Self {
        Self {
            tid,
            dir,
            peer,
            peer_name,
            link: None,
            offer: None,
            planned: None,
            paths: Vec::new(),
            phase: TransferPhase::Preparing,
            block: None,
            auto_accepted: false,
            started: Instant::now(),
            elapsed: None,
            cancel: None,
            cancelled_here: false,
            progress: Arc::new(ProgressState::default()),
            result: None,
            failure: None,
            link_kind: None,
            estimate: None,
            created: Instant::now(),
            marks: (None, None),
            moved: (Instant::now(), 0),
            stalled: false,
            awake: None,
            paused_at: None,
            paused_for: Duration::ZERO,
            resuming: false,
            asked: None,
            resume_from: None,
            pending: None,
            notes: Vec::new(),
            kept: None,
            save_to: None,
            stopped_here: false,
            waiting_there: false,
            percent_known: true,
            from_next: None,
        }
    }

    /// Running from now on: keep the computer awake until it ends.
    pub(super) fn run(&mut self) {
        let resumed = match self.paused_at {
            Some(at) if self.phase == TransferPhase::Paused => {
                self.paused_for += Duration::from_secs(crate::transfer::journal::now_secs().saturating_sub(at));
                true
            }
            _ => {
                self.started = Instant::now();
                false
            }
        };
        self.phase = TransferPhase::Running;
        self.paused_at = None;
        self.resuming = false;
        self.asked = None;
        self.moved = (Instant::now(), self.moved_bytes());
        let total = self.total();
        let mut estimate = crate::transfer::estimate::Estimator::new(total, self.created.elapsed(), self.moved_bytes());
        if resumed {
            // "Checking what already arrived…" until bytes move again.
            estimate.restart(self.created.elapsed(), self.moved_bytes());
        }
        self.estimate = Some(estimate);
        let verb = if self.dir == Direction::Out { "Sending files to" } else { "Receiving files from" };
        self.awake =
            portal_sys::power::KeepAwake::new(&format!("{}: {verb} {}", crate::consts::APP_NAME, self.peer_name));
    }

    /// Whole percent moved so far ("dropped at 42%").
    pub(super) fn percent(&self) -> u64 {
        (self.moved_bytes() * 100).checked_div(self.total()).unwrap_or(0).min(100)
    }

    /// Bytes that have arrived: on the sender, what the receiver says it
    /// has; on the receiver, what came in.
    pub(super) fn moved_bytes(&self) -> u64 {
        match self.dir {
            Direction::Out => self.progress.acked(),
            Direction::In => self.progress.bytes(),
        }
    }

    pub(super) fn total(&self) -> u64 {
        self.offer.as_ref().map(|o| o.total_bytes).unwrap_or(0)
    }

    /// Holds the one transfer slot (a paused transfer does too).
    pub fn active(&self) -> bool {
        matches!(
            self.phase,
            TransferPhase::Preparing | TransferPhase::Offered | TransferPhase::Running | TransferPhase::Paused
        )
    }

    pub fn view(&self) -> TransferView {
        let offer = self.offer.as_ref();
        TransferView {
            id: format!("{:016x}", self.tid),
            tid: self.tid,
            direction: self.dir,
            peer_id: self.peer.clone(),
            peer_name: self.peer_name.clone(),
            phase: self.phase,
            block: self.block,
            items: offer
                .map(|o| {
                    o.items
                        .iter()
                        .map(|i| ItemView {
                            name: i.name.clone(),
                            folder: i.kind == ItemKind::Folder,
                            bytes: i.bytes,
                            files: i.files,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            total_bytes: offer.map(|o| o.total_bytes).unwrap_or(0),
            file_count: offer.map(|o| o.file_count).unwrap_or(0),
            auto_accepted: self.auto_accepted,
            bytes: self.moved_bytes(),
            elapsed_ms: self.elapsed.map(|d| d.as_millis() as u64),
            link: self.link_kind.map(LinkKind::shown),
            stalled: self.stalled && self.phase == TransferPhase::Running,
            result: self.result.clone(),
            failure: self.failure.clone(),
            save_to: match &self.result {
                Some(r) => r.saved_to.clone(),
                None => self.save_to.as_ref().map(|p| p.display().to_string()),
            },
            paused_since_ms: self.paused_at.filter(|_| self.phase == TransferPhase::Paused).map(|s| s * 1000),
            stopped_here: self.stopped_here && self.phase == TransferPhase::Paused,
            waiting_there: self.waiting_there && self.phase == TransferPhase::Paused,
            percent_known: self.percent_known,
            resuming: self.resuming,
            can_send_again: self.dir == Direction::Out
                && matches!(self.phase, TransferPhase::Failed | TransferPhase::Cancelled)
                && !self.paths.is_empty(),
        }
    }
}

fn new_tid() -> TransferId {
    u64::from_be_bytes(random_bytes::<8>())
}

fn os_word(os: Os) -> &'static str {
    match os {
        Os::Macos => "macOS",
        Os::Windows => "Windows",
        Os::Linux => "Linux",
        Os::Other => "this computer",
    }
}

pub(super) fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

pub(super) fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1000.0 && u < UNITS.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 { format!("{bytes} bytes") } else { format!("{v:.1} {}", UNITS[u]) }
}

/// The transfer with this id (borrows only the list, so other fields stay usable).
pub(super) fn find(list: &[Transfer], tid: TransferId) -> Option<&Transfer> {
    list.iter().find(|t| t.tid == tid)
}

pub(super) fn find_mut(list: &mut [Transfer], tid: TransferId) -> Option<&mut Transfer> {
    list.iter_mut().find(|t| t.tid == tid)
}

/// Finished transfers kept on screen until dismissed, at most.
const KEEP_FINISHED: usize = 12;

impl Node {
    pub(super) fn this_computer(&self) -> &'static str {
        match Os::current() {
            Os::Macos => "this Mac",
            Os::Windows => "this PC",
            _ => "this computer",
        }
    }

    /// The transfer holding a device's one slot: preparing, offered, running
    /// or paused, in either direction.
    pub(super) fn active_with(&self, peer: &DeviceId) -> Option<&Transfer> {
        self.transfers.iter().find(|t| &t.peer == peer && t.active())
    }

    /// Bytes are (supposed to be) moving in some transfer.
    pub(super) fn transfer_running(&self) -> bool {
        self.transfers.iter().any(|t| t.phase == TransferPhase::Running)
    }

    /// Adds a transfer, clearing finished ones with the same device (a new
    /// transfer replaces their done screen) and keeping the list short.
    fn add_transfer(&mut self, t: Transfer) {
        let peer = t.peer.clone();
        self.transfers.retain(|x| x.active() || x.peer != peer);
        while self.transfers.iter().filter(|x| !x.active()).count() >= KEEP_FINISHED {
            let Some(i) = self.transfers.iter().position(|x| !x.active()) else { break };
            self.transfers.remove(i);
        }
        self.transfers.push(t);
    }

    pub(super) fn remove_transfer(&mut self, tid: TransferId) {
        self.transfers.retain(|t| t.tid != tid);
    }

    /// What the window says when a device's one transfer slot is taken.
    fn busy_text(&self, name: &str, t: &Transfer) -> String {
        match t.phase {
            TransferPhase::Paused => format!(
                "A paused transfer with {name} is waiting to continue. It goes first; send this when it’s done, or cancel the paused one."
            ),
            TransferPhase::Offered if t.dir == Direction::In => {
                format!("{name} is offering files to this computer. Answer that first, then send.")
            }
            _ => format!("{name} is busy with another transfer. Send this when it’s done (one at a time per device)."),
        }
    }

    /// Send to several devices: one transfer each. Says per device what
    /// didn't start.
    /// A device that's busy with a transfer gets them next (Mission 05 §1.5):
    /// they wait, and start when that transfer ends.
    pub(super) fn cmd_send_many(
        &mut self,
        peers: Vec<DeviceId>,
        paths: Vec<PathBuf>,
    ) -> Vec<(DeviceId, Result<(), String>)> {
        peers
            .into_iter()
            .map(|p| {
                let busy = self.active_with(&p).is_some() && self.trusts(&p);
                let r = if busy && !paths.is_empty() {
                    self.queue_next(p.clone(), paths.clone());
                    Ok(())
                } else {
                    self.cmd_send(p.clone(), paths.clone())
                };
                (p, r)
            })
            .collect()
    }

    /// Drop on the window / "Choose a folder": start a send to one device.
    pub(super) fn cmd_send(&mut self, peer: DeviceId, paths: Vec<PathBuf>) -> Result<(), String> {
        if paths.is_empty() {
            return Err("Nothing to send.".into());
        }
        let name = self.store.peer(&peer).map(|p| p.name.clone()).unwrap_or_else(|| "That device".into());
        if !self.trusts(&peer) {
            return Err(format!("Pair with {name} first. Unpaired devices can’t receive files."));
        }
        if self.store.peer(&peer).is_some_and(|p| p.disconnected) {
            return Err(format!("Disconnected from {name}. Press Connect to send to it again."));
        }
        if let Some(t) = self.active_with(&peer) {
            return Err(self.busy_text(&name, t));
        }
        let mut t = Transfer::new(new_tid(), Direction::Out, peer.clone(), name.clone());
        t.paths = paths.clone();
        if self.link_for(&peer).is_none() {
            return Err(format!("{name} isn’t connected right now."));
        }
        let (tid, skip_junk) = (t.tid, self.store.settings().skip_junk);
        self.add_transfer(t);
        self.dirty = true;
        let internal = self.internal_tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = plan::plan(tid, &paths, skip_junk).map_err(|e| e.to_string());
            let _ = internal.blocking_send(Internal::Planned(tid, result));
        });
        Ok(())
    }

    pub(super) fn on_planned(&mut self, tid: TransferId, result: Result<Planned, String>) {
        let Some(t) = find_mut(&mut self.transfers, tid).filter(|t| t.phase == TransferPhase::Preparing) else {
            return;
        };
        let planned = match result {
            Ok(p) => p,
            Err(e) => {
                t.phase = TransferPhase::Failed;
                t.failure = Some(format!("Couldn’t read what you dropped: {e}"));
                self.dirty = true;
                return;
            }
        };
        if planned.files.is_empty() && planned.dirs.is_empty() {
            t.phase = TransferPhase::Failed;
            t.failure = Some("Nothing to send: links and system files are skipped.".into());
            self.dirty = true;
            return;
        }
        // The receiver told us it never takes our files: say so before offering
        // (it would refuse anyway; the receiving core decides).
        let peer = t.peer.clone();
        if self.peers.get(&peer).and_then(|p| p.accepts) == Some(Incoming::Never) {
            let t = find_mut(&mut self.transfers, tid).expect("transfer");
            t.offer = Some(planned.offer);
            t.phase = TransferPhase::Blocked;
            t.block = Some(Block::NotAccepted);
            let name = t.peer_name.clone();
            self.say(
                "block",
                format!("{name} doesn’t accept files from this computer, so nothing was sent"),
                None,
                Some(Tone::Warn),
            );
            self.dirty = true;
            return;
        }
        let Some(link) = self.link_for(&peer) else {
            let t = find_mut(&mut self.transfers, tid).expect("transfer");
            t.phase = TransferPhase::Failed;
            t.failure = Some(format!("Lost the connection to {}. Nothing was sent.", t.peer_name));
            self.dirty = true;
            return;
        };
        let offer = planned.offer.clone();
        let skipped = planned.skipped.len() as u64;
        let t = find_mut(&mut self.transfers, tid).expect("transfer");
        t.link = Some(link.id());
        t.offer = Some(offer.clone());
        t.planned = Some(planned);
        t.phase = TransferPhase::Offered;
        let what = describe(&offer);
        let name = t.peer_name.clone();
        link.try_send(Msg::Offer(offer.clone()));
        // From the queue: now that the offer is out, say what's next after it.
        if t.from_next.is_some() {
            self.tell_next(&peer);
        }
        self.say("offer", format!("Offered {what} to {name}"), Some(size(offer.total_bytes)), None);
        // Every skip is listed in Details (brief §6.5), up to a sensible cap.
        let notes =
            find(&self.transfers, tid).and_then(|t| t.planned.as_ref()).map(|p| p.skipped.clone()).unwrap_or_default();
        for n in notes.iter().take(LISTED) {
            self.say(
                "skip",
                format!("Left out {}", n.path.join("/")),
                Some(skip_reason(n.reason).into()),
                Some(Tone::Dim),
            );
        }
        if skipped > LISTED as u64 {
            self.say("skip", format!("…and {} more left out", skipped - LISTED as u64), None, Some(Tone::Dim));
        }
        self.dirty = true;
    }

    /// Offer from a trusted device.
    pub(super) fn on_offer(&mut self, link: &crate::net::link::Link, offer: Offer) {
        let info = link.info();
        let (peer, name) = (
            info.hello.id.clone(),
            self.store.peer(&info.hello.id).map(|p| p.name.clone()).unwrap_or_else(|| info.hello.name.clone()),
        );
        let decline = |reason| {
            link.try_send(Msg::Decline { tid: offer.tid, reason });
        };
        // A new offer from the device a transfer is paused with: it gave up on
        // that one (cancelled while we were apart), so this one replaces it.
        let paused: Vec<TransferId> = self
            .transfers
            .iter()
            .filter(|t| t.phase == TransferPhase::Paused && t.peer == peer)
            .map(|t| t.tid)
            .collect();
        for tid in paused {
            self.cancel_paused(tid);
        }
        // One transfer at a time per device (any number across devices).
        if self.active_with(&peer).is_some() {
            decline(DeclineReason::Busy);
            self.say("busy", format!("{name} tried to send during another transfer with it"), None, None);
            return;
        }
        let what = describe(&offer);
        let incoming = self.store.peer(&peer).map(|p| p.incoming).unwrap_or_default();
        // "Files from <device>: Never": refused here, whatever the sender knew.
        if incoming == Incoming::Never {
            decline(DeclineReason::NotAccepting);
            self.say("refuse", format!("Refused {what} from {name}: files from {name} are set to Never"), None, None);
            return;
        }
        let mut t = Transfer::new(offer.tid, Direction::In, peer.clone(), name.clone());
        t.save_to = Some(self.receive_dir_for(Some(peer.clone())));
        t.link = Some(link.id());
        t.offer = Some(offer.clone());
        let needed = offer.total_bytes + SPACE_MARGIN;
        // Space is reserved across every incoming transfer: what the others
        // still need isn't free for this one.
        let reserved = self.reserved_incoming();
        let available = self.free_space().map(|free| free.saturating_sub(reserved));
        if let Some(available) = available.filter(|a| *a < needed) {
            decline(DeclineReason::NoSpace { needed, available });
            t.phase = TransferPhase::Failed;
            t.failure = Some(format!(
                "Not enough space for {what} from {name}: it needs {} and {} free. Free up space or choose another folder in Settings.",
                size(needed),
                size(available)
            ));
            self.say(
                "space",
                format!("Refused {what} from {name}: not enough free space"),
                Some(format!("needs {}, {} free", size(needed), size(available))),
                Some(Tone::Warn),
            );
            self.add_transfer(t);
            self.dirty = true;
            return;
        }
        t.phase = TransferPhase::Offered;
        let always = incoming == Incoming::Always;
        let tid = t.tid;
        self.add_transfer(t);
        self.say("offer", format!("{name} offers {what}"), Some(size(offer.total_bytes)), None);
        self.dirty = true;
        if always {
            self.say("accept", format!("Accepted: files from {name} are always accepted"), None, None);
            self.accept_offer(tid, true);
        } else {
            self.notice(
                NoticeKind::Offer,
                format!("{name} wants to send"),
                format!("{what}, {}", size(offer.total_bytes)),
            );
        }
    }

    /// Where files from `peer` are saved (Mission 05 §4): straight into the
    /// folder; in the SeamDrop folder, with "A folder for each device" its
    /// own folder inside it (Mission 04 §1.8); or a new numbered folder for
    /// each delivery, made (so its number is taken) only when `reserve`.
    pub(super) fn receive_dir_for(&mut self, peer: Option<DeviceId>) -> PathBuf {
        self.receive_dir_reserving(peer, false)
    }

    pub(super) fn receive_dir_reserving(&mut self, peer: Option<DeviceId>, reserve: bool) -> PathBuf {
        let base = self.store.settings().receive_dir.clone();
        match self.store.settings().receive_mode.unwrap_or_default() {
            crate::store::ReceiveMode::Loose => base,
            crate::store::ReceiveMode::AppFolder => {
                match peer.filter(|_| self.store.settings().folder_per_device).and_then(|p| self.device_folder(&p)) {
                    Some(folder) => base.join(folder),
                    None => base,
                }
            }
            crate::store::ReceiveMode::Numbered if !reserve => base,
            crate::store::ReceiveMode::Numbered => {
                let name = peer.and_then(|p| self.store.peer(&p).map(|t| t.name.clone())).unwrap_or_default();
                numbered_folder(&base, &name).unwrap_or(base)
            }
        }
    }

    /// "From MacBook Pro": named after what this computer calls the device,
    /// safe on every OS, unique among the devices here, chosen once and kept.
    fn device_folder(&mut self, peer: &DeviceId) -> Option<String> {
        let p = self.store.peer(peer)?.clone();
        if let Some(f) = p.folder {
            return Some(f);
        }
        let safe = crate::fsafe::sanitize(&format!("From {}", p.name), crate::fsafe::Target::Windows).name;
        let name = match safe.trim_start_matches('.') {
            "" => "From a device".to_owned(),
            s => s.to_owned(),
        };
        let taken: std::collections::HashSet<String> = self
            .store
            .peers()
            .iter()
            .filter(|o| &o.id != peer)
            .filter_map(|o| o.folder.as_deref().map(crate::fsafe::fold))
            .collect();
        let mut chosen = name.clone();
        let mut n = 1;
        while taken.contains(&crate::fsafe::fold(&chosen)) {
            n += 1;
            chosen = format!("{name} ({n})");
        }
        if let Err(e) = self.store.update_peer(peer, |t| t.folder = Some(chosen.clone())) {
            self.say("error", "Couldn’t save the settings", Some(e.to_string()), Some(Tone::Err));
        }
        Some(chosen)
    }

    /// What incoming transfers (offered, running, paused) still need.
    fn reserved_incoming(&self) -> u64 {
        self.transfers
            .iter()
            .filter(|t| t.dir == Direction::In && t.active())
            .map(|t| t.total().saturating_sub(t.moved_bytes()) + SPACE_MARGIN)
            .sum()
    }

    fn free_space(&self) -> Option<u64> {
        if let Some(v) = self.cfg.fault_free_space {
            return Some(v);
        }
        let mut dir = self.store.settings().receive_dir.clone();
        loop {
            if dir.exists() {
                return fs4::available_space(&dir).ok();
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    pub(super) fn cmd_accept(&mut self, tid: TransferId, always: bool) {
        let Some(t) =
            find(&self.transfers, tid).filter(|t| t.dir == Direction::In && t.phase == TransferPhase::Offered)
        else {
            return;
        };
        if always {
            let peer = t.peer.clone();
            self.cmd_set_incoming(&peer, Incoming::Always);
        }
        self.accept_offer(tid, false);
    }

    fn accept_offer(&mut self, tid: TransferId, auto: bool) {
        let receive_dir = self.receive_dir_reserving(find(&self.transfers, tid).map(|t| t.peer.clone()), true);
        let keep_dates = self.store.settings().keep_dates;
        let link = find(&self.transfers, tid).and_then(|t| t.link).and_then(|id| self.links.get(&id).cloned());
        let kind = link.as_ref().map(|l| self.link_kind(&l.info().remote, Some(l.info().local.ip()), None));
        let target = self.name_rules();
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        let Some(link) = link.filter(|l| !l.is_closed()) else {
            t.phase = TransferPhase::Failed;
            t.failure = Some(format!("Lost the connection to {}. Nothing was saved.", t.peer_name));
            self.dirty = true;
            return;
        };
        // Unfinished files wait in the one folder's hidden staging area,
        // wherever they're saved in the end (one place to tidy up).
        let staging = self.store.settings().receive_dir.join(INCOMING_DIR).join(format!("{:016x}", t.tid));
        if let Err(e) = std::fs::create_dir_all(&staging).and_then(|()| std::fs::create_dir_all(&receive_dir)) {
            link.try_send(Msg::Decline { tid: t.tid, reason: DeclineReason::Other });
            t.phase = TransferPhase::Failed;
            t.failure = Some(format!("{} couldn’t write to {}: {e}", crate::consts::APP_NAME, receive_dir.display()));
            self.dirty = true;
            return;
        }
        if let Some(area) = staging.parent() {
            portal_sys::fs::hide(area);
        }
        t.save_to = Some(receive_dir.clone());
        t.auto_accepted = auto;
        t.link_kind = kind;
        let state = InState {
            tid: t.tid,
            peer: t.peer.clone(),
            peer_name: t.peer_name.clone(),
            offer: t.offer.clone().expect("offer"),
            auto_accepted: auto,
            receive_dir,
            staging,
            target,
            namer: crate::fsafe::Namer::new(target),
            dir_times: Vec::new(),
            dirs: 0,
            done: 0,
            bytes_done: 0,
            partial: None,
            paused_at: None,
            stopped: None,
            keep_dates,
        };
        // Saved before anything arrives, so even a crash in the first second resumes.
        if let Err(e) = self.journal.save(&Saved::In(Box::new(state.clone()))) {
            self.say("pause", SAVE_FAILED, Some(e.to_string()), Some(Tone::Warn));
        }
        self.spawn_recv(tid, &link, state, None);
        link.try_send(Msg::Accept { tid });
        self.tech("stream", "receiving, 1 MiB chunks, BLAKE3 per file");
        self.dirty = true;
    }

    pub(super) fn cmd_decline(&mut self, tid: TransferId) {
        let Some(t) =
            find(&self.transfers, tid).filter(|t| t.dir == Direction::In && t.phase == TransferPhase::Offered)
        else {
            return;
        };
        if let Some(link) = t.link.and_then(|id| self.links.get(&id)) {
            link.try_send(Msg::Decline { tid: t.tid, reason: DeclineReason::User });
        }
        self.say(
            "decline",
            format!("Declined {} from {}", t.offer.as_ref().map(describe).unwrap_or_default(), t.peer_name),
            None,
            None,
        );
        self.remove_transfer(tid);
        self.dirty = true;
    }

    pub(super) fn cmd_cancel(&mut self, tid: TransferId) {
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        match t.phase {
            TransferPhase::Paused => return self.cancel_paused(tid),
            TransferPhase::Running => {
                t.cancelled_here = true;
                if let Some(c) = &t.cancel {
                    c.send_replace(true);
                }
            }
            TransferPhase::Offered if t.dir == Direction::In => return self.cmd_decline(tid),
            TransferPhase::Offered => {
                if let Some(link) = t.link.and_then(|id| self.links.get(&id)) {
                    link.try_send(Msg::Cancel { tid: t.tid, reason: CancelReason::User });
                }
                self.say("cancel", "Withdrew the offer", None, None);
                self.remove_transfer(tid);
            }
            _ => self.remove_transfer(tid),
        }
        self.dirty = true;
    }

    pub(super) fn cmd_dismiss(&mut self, tid: TransferId) {
        if find(&self.transfers, tid).is_some_and(|t| !t.active()) {
            self.remove_transfer(tid);
            self.dirty = true;
        }
    }

    /// accept / decline / cancel-before-start from the other side.
    pub(super) fn on_transfer_msg(&mut self, link: &crate::net::link::Link, msg: Msg) {
        let tid = match &msg {
            Msg::Accept { tid } | Msg::Decline { tid, .. } | Msg::Cancel { tid, .. } => *tid,
            _ => return,
        };
        // Only the link the offer went over may answer it.
        let Some(t) = find_mut(&mut self.transfers, tid).filter(|t| t.link == Some(link.id())) else { return };
        match msg {
            Msg::Accept { tid } if tid == t.tid && t.dir == Direction::Out && t.phase == TransferPhase::Offered => {
                let Some(planned) = t.planned.clone() else { return };
                let t = find(&self.transfers, tid).expect("transfer");
                self.save_out(t, &planned);
                self.spawn_send(tid, link, crate::transfer::send::Start::default(), 0);
                self.say(
                    "accept",
                    format!("{} accepted; sending", link.info().hello.name),
                    Some("1 MiB chunks".into()),
                    None,
                );
                self.dirty = true;
            }
            // Ours, from what was next, crossed theirs: back to the front,
            // quietly (theirs goes first).
            Msg::Decline { tid, reason: DeclineReason::Busy } if tid == t.tid && t.from_next.is_some() => {
                let n = t.from_next.take().expect("from next");
                self.remove_transfer(tid);
                self.requeue_next(n);
            }
            Msg::Decline { tid, reason } if tid == t.tid && t.dir == Direction::Out => {
                let name = t.peer_name.clone();
                match reason {
                    DeclineReason::User => {
                        t.phase = TransferPhase::Declined;
                        self.say("decline", format!("{name} declined, so nothing was sent"), None, None);
                    }
                    DeclineReason::NotAccepting => {
                        t.phase = TransferPhase::Blocked;
                        t.block = Some(Block::NotAccepted);
                        t.planned = None;
                        self.say(
                            "refuse",
                            format!("{name} doesn’t accept files from this computer"),
                            None,
                            Some(Tone::Warn),
                        );
                    }
                    other => {
                        t.phase = TransferPhase::Failed;
                        t.failure = Some(match other {
                            DeclineReason::Busy => {
                                format!("{name} is busy with another transfer. Try again when it’s done.")
                            }
                            DeclineReason::NoSpace { needed, available } => format!(
                                "{name} doesn’t have enough free space: it needs {} and has {}.",
                                size(needed),
                                size(available)
                            ),
                            DeclineReason::NotPaired => {
                                format!("{name} doesn’t trust this device any more. Pair again.")
                            }
                            _ => format!("{name} couldn’t take it."),
                        });
                        self.say(
                            "refuse",
                            format!("{name} couldn’t take it"),
                            Some(format!("{other:?}")),
                            Some(Tone::Warn),
                        );
                    }
                }
                self.dirty = true;
            }
            Msg::Cancel { tid, .. } if tid == t.tid && t.dir == Direction::In && t.phase == TransferPhase::Offered => {
                let name = t.peer_name.clone();
                self.say("cancel", format!("{name} withdrew the offer"), None, None);
                self.remove_transfer(tid);
                self.dirty = true;
            }
            _ => {}
        }
    }

    /// `cancel` / `complete` about a paused transfer (no run is listening).
    pub(super) fn on_paused_msg(&mut self, link: &crate::net::link::Link, msg: Msg) {
        let peer = link.info().hello.id.clone();
        let tid = match &msg {
            Msg::Cancel { tid, .. } | Msg::Complete { tid, .. } => *tid,
            _ => return,
        };
        let Some(t) = find(&self.transfers, tid).filter(|t| t.peer == peer && t.phase == TransferPhase::Paused) else {
            return;
        };
        match msg {
            Msg::Cancel { tid, reason } if tid == t.tid => self.paused_ended_there(tid, reason),
            // The receiver had finished; only the report went missing.
            Msg::Complete { tid, summary } if tid == t.tid && t.dir == Direction::Out => self.on_sent(tid, Ok(summary)),
            _ => {}
        }
    }

    pub(super) fn on_sent(&mut self, tid: TransferId, outcome: Result<Summary, Failure>) {
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        if let Some(l) = t.link.and_then(|id| self.links.get(&id)) {
            l.set_sink(None);
        }
        let notes = t.progress.take_notes();
        for n in &notes {
            self.say("resume", n.clone(), None, None);
        }
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        t.notes.extend(notes);
        if matches!(outcome, Err(Failure::Lost)) && !t.cancelled_here {
            return self.pause(tid);
        }
        let name = t.peer_name.clone();
        let peer_id = t.peer.clone();
        let peer_os = self.store.peer(&t.peer).map(|p| p.os).unwrap_or(Os::Other);
        t.elapsed = Some(t.started.elapsed().saturating_sub(t.paused_for));
        t.awake = None;
        let percent = t.percent();
        match outcome {
            Ok(summary) => {
                self.journal.remove(tid);
                let skipped = t.offer.as_ref().map(|o| o.skipped.len() as u64).unwrap_or(0);
                let mut notes = t.notes.clone();
                if let Some(kept) = t.kept.filter(|k| *k > 0) {
                    notes.insert(
                        0,
                        format!(
                            "Continued after the connection dropped: the {} already on {name} wasn’t sent again.",
                            size(kept)
                        ),
                    );
                }
                if summary.renamed > 0 {
                    notes.push(format!("{} on {}.", renamed_words(summary.renamed), os_word(peer_os)));
                }
                if skipped > 0 {
                    let words = t.offer.as_ref().map(|o| skip_words(&o.skipped)).unwrap_or_default();
                    notes.push(format!("{} left out: {words}.", plural(skipped, "item", "items")));
                }
                let kept = t.kept.unwrap_or(0);
                let renamed_list = renamed_items(&summary.renames);
                t.result = Some(ResultView {
                    saved_to: summary.saved_in.clone(),
                    reveal: None,
                    open: None,
                    files: summary.files,
                    bytes: summary.bytes,
                    renamed: summary.renamed,
                    skipped,
                    long_paths: summary.long_paths,
                    kept_bytes: kept,
                    skipped_items: t.offer.as_ref().map(|o| skipped_items(&o.skipped)).unwrap_or_default(),
                    renamed_items: renamed_list,
                    notes,
                    first_arrival: false,
                });
                t.phase = TransferPhase::Done;
                let secs = t.elapsed.map(|d| d.as_secs_f64()).unwrap_or(0.0).max(0.001);
                // The sender's Details names what changed on the other side too
                for rn in summary.renames.iter() {
                    self.say(
                        "rename",
                        format!("Renamed on {name}: {} to {}", rn.from.join("/"), rn.to.join("/")),
                        Some(rn.reason.clone()),
                        None,
                    );
                }
                self.say("verify", format!("Every file checked on {name} ✓"), Some("BLAKE3".into()), None);
                self.say(
                    "sent",
                    format!("Sent {} ({}) to {name}", plural(summary.files, "file", "files"), size(summary.bytes)),
                    Some(moved_words(summary.bytes, kept, secs)),
                    None,
                );
            }
            Err(Failure::CancelledHere) | Err(_) if t.cancelled_here => {
                self.journal.end(tid, &t.peer, Ending::Cancelled);
                self.say("cancel", format!("Cancelled; the partial files on {name} were removed"), None, None);
                let text = self.cancelled_text(Direction::Out, &name);
                let t = find_mut(&mut self.transfers, tid).expect("transfer");
                t.phase = TransferPhase::Cancelled;
                t.failure = Some(text);
            }
            Err(f) => {
                self.journal.remove(tid);
                let msg = self.failure_text(&f, Direction::Out, &name, percent);
                self.say("stop", msg.clone(), Some(format!("{} {}", f.code(), f.detail())), Some(Tone::Err));
                self.notice(NoticeKind::Failed, "Transfer stopped".into(), msg.clone());
                let t = find_mut(&mut self.transfers, tid).expect("transfer");
                t.phase = TransferPhase::Failed;
                t.failure = Some(msg);
            }
        }
        self.log_middle_speed(tid);
        self.dirty = true;
        // The device's slot is free: a transfer paused with it goes next.
        self.peer_back(&peer_id);
    }

    /// "middle 80 %: 112.4 MB/s" in Details, for transfers long enough to measure.
    fn log_middle_speed(&self, tid: TransferId) {
        let Some(t) = find(&self.transfers, tid).filter(|t| t.phase == TransferPhase::Done) else { return };
        let (Some(a), Some(b)) = t.marks else { return };
        let total = t.offer.as_ref().map(|o| o.total_bytes).unwrap_or(0);
        let secs = b.duration_since(a).as_secs_f64();
        if total >= 100 * 1024 * 1024 && secs > 0.0 {
            let rate = (total as f64 * 0.8) / secs / 1e6;
            self.say(
                "speed",
                format!("Average speed {rate:.1} MB/s"),
                Some(format!("middle 80 %, over {secs:.1} s")),
                Some(Tone::Dim),
            );
        }
    }

    pub(super) fn on_received(&mut self, tid: TransferId, outcome: Result<recv::Received, Failure>) {
        let rules_os = match self.name_rules() {
            crate::fsafe::Target::Windows => "Windows",
            crate::fsafe::Target::Mac => "macOS",
            crate::fsafe::Target::Other => "this computer",
        };
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        if let Some(l) = t.link.and_then(|id| self.links.get(&id)) {
            l.set_sink(None);
        }
        if matches!(outcome, Err(Failure::Lost)) && !t.cancelled_here {
            return self.pause(tid);
        }
        let name = t.peer_name.clone();
        let peer_id = t.peer.clone();
        t.elapsed = Some(t.started.elapsed().saturating_sub(t.paused_for));
        t.awake = None;
        let percent = t.percent();
        match outcome {
            Ok(r) => {
                self.journal.end(tid, &t.peer, Ending::Done { summary: r.summary.clone() });
                self.facts.write_denied = false;
                let s = &r.summary;
                let mut notes: Vec<String> = r
                    .numbered
                    .iter()
                    .take(3)
                    .map(|(wanted, got)| format!("“{wanted}” already existed, so it was saved as “{got}”."))
                    .collect();
                if r.numbered.len() > 3 {
                    notes.push(format!("{} more already existed and were numbered.", r.numbered.len() - 3));
                }
                if s.renamed > 0 {
                    notes.push(format!("{} on {rules_os}.", renamed_words(s.renamed)));
                }
                if s.skipped > 0 {
                    let words = t.offer.as_ref().map(|o| skip_words(&o.skipped)).unwrap_or_default();
                    notes.push(format!("{} left out on {name}: {words}.", plural(s.skipped, "item", "items")));
                }
                notes.extend(t.notes.iter().cloned());
                if let Some(kept) = t.kept.filter(|k| *k > 0) {
                    notes.insert(
                        0,
                        format!(
                            "Continued after the connection dropped: the {} already here was kept and checked.",
                            size(kept)
                        ),
                    );
                }
                if s.long_paths > 0 {
                    notes.push(format!(
                        "{} longer than 260 characters. Some Windows apps may not open them.",
                        plural(s.long_paths, "path is", "paths are")
                    ));
                }
                let receive_dir = r.saved_in.display().to_string();
                let renames = r.renames.clone();
                let first_arrival = !self.store.settings().received_before;
                let skipped_list = t.offer.as_ref().map(|o| o.skipped.clone()).unwrap_or_default();
                t.result = Some(ResultView {
                    saved_to: Some(receive_dir.clone()),
                    reveal: r.saved.first().map(|p| p.display().to_string()),
                    open: match (r.saved.as_slice(), s.files) {
                        ([one], 1) if one.is_file() && crate::open::safe_to_open(&one.to_string_lossy()) => {
                            Some(one.display().to_string())
                        }
                        _ => None,
                    },
                    files: s.files,
                    bytes: s.bytes,
                    renamed: s.renamed,
                    skipped: s.skipped,
                    long_paths: s.long_paths,
                    kept_bytes: t.kept.unwrap_or(0),
                    skipped_items: skipped_items(&skipped_list),
                    renamed_items: renamed_items(&s.renames),
                    notes,
                    first_arrival,
                });
                t.phase = TransferPhase::Done;
                if first_arrival && let Err(e) = self.store.update_settings(|s| s.received_before = true) {
                    tracing::warn!("saving settings: {e}");
                }
                let what = t.offer.as_ref().map(describe).unwrap_or_default();
                let (files, bytes) = (s.files, s.bytes);
                for p in r.long.iter().take(20) {
                    self.say(
                        "long",
                        format!("{} is longer than Windows allows by default", p.display()),
                        Some("over 260 characters".into()),
                        Some(Tone::Warn),
                    );
                }
                for rn in renames.iter().take(LISTED) {
                    self.say(
                        "rename",
                        format!("Renamed {} to {}", rn.from.join("/"), rn.to.join("/")),
                        Some(rn.reason.to_string()),
                        None,
                    );
                }
                // What the sender left out, named here too (brief §6.5)
                for n in skipped_list.iter().take(LISTED) {
                    self.say(
                        "skip",
                        format!("{name} left out {}", n.path.join("/")),
                        Some(skip_reason(n.reason).into()),
                        Some(Tone::Dim),
                    );
                }
                self.say(
                    "verify",
                    "Every file checked ✓",
                    Some(format!("BLAKE3, {}", plural(files, "file", "files"))),
                    None,
                );
                self.say(
                    "saved",
                    format!("Saved {what} in {}", short_dir(&r.saved_in)),
                    Some(format!("{}, {receive_dir}", size(bytes))),
                    None,
                );
                self.notice(NoticeKind::Received, format!("Received {what} from {name}"), size(bytes));
            }
            Err(_) if t.cancelled_here => {
                // The connection may have dropped just before Cancel took
                // effect, which keeps the partial files for a resume.
                let peer = t.peer.clone();
                self.drop_partial(tid);
                self.journal.end(tid, &peer, Ending::Cancelled);
                self.say("cancel", "Cancelled; the partial files were removed", None, None);
                let text = self.cancelled_text(Direction::In, &name);
                let t = find_mut(&mut self.transfers, tid).expect("transfer");
                t.phase = TransferPhase::Cancelled;
                t.failure = Some(text);
            }
            Err(f) => {
                self.journal.remove(tid);
                self.facts.write_denied = matches!(f, Failure::AccessDenied(_));
                let msg = self.failure_text(&f, Direction::In, &name, percent);
                self.say("stop", msg.clone(), Some(format!("{} {}", f.code(), f.detail())), Some(Tone::Err));
                self.notice(NoticeKind::Failed, "Transfer stopped".into(), msg.clone());
                let t = find_mut(&mut self.transfers, tid).expect("transfer");
                t.phase = TransferPhase::Failed;
                t.failure = Some(msg);
            }
        }
        self.log_middle_speed(tid);
        self.dirty = true;
        // The device's slot is free: a transfer paused with it goes next.
        self.peer_back(&peer_id);
    }

    /// What the screen says after Cancel was pressed here.
    pub(super) fn cancelled_text(&self, dir: Direction, name: &str) -> String {
        match dir {
            Direction::Out => {
                format!(
                    "You cancelled it. Your files are still on {}; nothing was saved on {name}.",
                    self.this_computer()
                )
            }
            Direction::In => format!("You cancelled it. What had arrived from {name} was removed."),
        }
    }

    /// "Send again" after a send that stopped or was cancelled: the same items.
    pub(super) fn cmd_send_again(&mut self, tid: TransferId) -> Result<(), String> {
        let Some(t) =
            find(&self.transfers, tid).filter(|t| t.dir == Direction::Out && !t.active() && !t.paths.is_empty())
        else {
            return Err("There’s nothing to send again.".into());
        };
        let (peer, paths) = (t.peer.clone(), t.paths.clone());
        self.remove_transfer(tid);
        self.cmd_send(peer, paths)
    }

    /// What the failed screen says. `percent`: how far it got.
    pub(super) fn failure_text(&self, f: &Failure, dir: Direction, name: &str, percent: u64) -> String {
        let me = self.this_computer();
        let nothing = if dir == Direction::Out {
            format!("Your files are still on {me}; nothing was saved on {name}.")
        } else {
            "Nothing was saved.".to_owned()
        };
        match f {
            Failure::CancelledHere => format!("Cancelled. {nothing}"),
            Failure::CancelledThere { code: None, .. } => format!("{name} cancelled. {nothing}"),
            Failure::CancelledThere { code: Some(code), msg } => match code.as_str() {
                "disk_full" => format!("{name} ran out of space. {nothing}"),
                "access_denied" => format!("{name} couldn’t save the files ({msg}). {nothing}"),
                "corrupt" => format!("A file arrived damaged on {name} and was discarded ({msg}). Try again."),
                "read_failed" => format!("{name} couldn’t read {msg}. {nothing}"),
                "cancelled" => format!("{name} cancelled. {nothing}"),
                "expired" => format!("It was paused for too long, so {name} removed what had arrived. Send it again."),
                "unknown" => format!("{name} no longer has this transfer. {nothing} Send it again."),
                _ => format!("{name} stopped the transfer ({code}). {nothing}"),
            },
            Failure::Lost => {
                let again = if dir == Direction::Out {
                    format!("Send it again once {name} shows as connected.")
                } else {
                    format!("Send it again from {name} once it shows as connected.")
                };
                format!("The connection to {name} dropped at {percent}%. {nothing} {again}")
            }
            Failure::DiskFull => {
                format!(
                    "{} ran out of space. {nothing} Free up space or choose another folder in Settings.",
                    capitalise(me)
                )
            }
            Failure::AccessDenied(what) => {
                format!("{APP_NAME} isn’t allowed to write to the receive folder ({what}). {nothing}")
            }
            Failure::Corrupt(file) => format!("“{file}” arrived damaged and was discarded. {nothing} Try again."),
            Failure::ReadFailed(msg) => format!("Couldn’t read {msg}. Nothing was sent."),
            Failure::Protocol(msg) | Failure::Io(msg) => format!("The transfer stopped: {msg}. {nothing}"),
        }
    }

    fn name_rules(&self) -> crate::fsafe::Target {
        self.cfg.name_rules.unwrap_or_else(crate::fsafe::Target::current)
    }

    pub(super) fn notice(&self, kind: NoticeKind, title: String, body: String) {
        let _ = self.events.send(Event::Notice(Notice { kind, title, body }));
    }

    /// Called every 100 ms while a transfer runs: progress events, and
    /// noticing when nothing has moved for a while.
    pub(super) fn emit_progress(&mut self) {
        let running: Vec<TransferId> =
            self.transfers.iter().filter(|t| t.phase == TransferPhase::Running).map(|t| t.tid).collect();
        for tid in running {
            self.emit_progress_of(tid);
        }
    }

    fn emit_progress_of(&mut self, tid: TransferId) {
        let notes = find(&self.transfers, tid).map(|t| t.progress.take_notes()).unwrap_or_default();
        for n in &notes {
            self.say("resume", n.clone(), None, None);
        }
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        t.notes.extend(notes);
        let bytes = t.moved_bytes();
        let now = Instant::now();
        if bytes != t.moved.1 {
            t.moved = (now, bytes);
        }
        let total = t.total();
        // At 100 % the receiver may still be verifying and saving: not a stall.
        let stalled = bytes < total && now.duration_since(t.moved.0) >= STALLED_AFTER;
        let mut said = None;
        if stalled != t.stalled {
            t.stalled = stalled;
            self.dirty = true;
            let (pct, name) = (t.percent(), t.peer_name.clone());
            said = Some(if stalled {
                format!("Nothing has moved with {name} for 2 seconds, at {pct} %; waiting")
            } else {
                format!("Moving again with {name}")
            });
        }
        if total > 0 && t.marks.0.is_none() && bytes * 10 >= total {
            t.marks.0 = Some(now);
        }
        if total > 0 && t.marks.1.is_none() && bytes * 10 >= total * 9 {
            t.marks.1 = Some(now);
        }
        let at = t.created.elapsed();
        let shown = t.estimate.as_mut().map(|e| e.sample(at, bytes));
        let file = t.progress.current();
        if let Some(text) = said {
            self.say("net", text, None, None);
        }
        let Some(shown) = shown else { return };
        let view = ProgressView {
            id: format!("{tid:016x}"),
            bytes,
            total,
            rate: shown.rate,
            speed: shown.speed,
            eta: shown.eta,
            file,
        };
        let _ = self.events.send(Event::Progress(view));
    }

    /// The peer now takes our files: a send it refused is offered again.
    pub(super) fn after_peer_accepts(&mut self, peer: &DeviceId, accepts: Option<Incoming>) {
        let retry = self
            .transfers
            .iter()
            .find(|t| t.dir == Direction::Out && t.block == Some(Block::NotAccepted) && &t.peer == peer)
            .map(|t| (t.tid, t.paths.clone()));
        if let (Some((tid, paths)), true) = (retry, accepts != Some(Incoming::Never)) {
            self.say(
                "resume",
                format!(
                    "{} accepts files from this computer now; sending",
                    self.store.peer(peer).map(|p| p.name.clone()).unwrap_or_default()
                ),
                None,
                None,
            );
            self.remove_transfer(tid);
            if let Err(e) = self.cmd_send(peer.clone(), paths) {
                self.say("send", e, None, Some(Tone::Warn));
            }
        }
    }

    /// Leftovers from a crash: partial files never stay around.
    /// Leftovers that can't be resumed (a crash, an old build) are removed;
    /// a paused transfer's staging folder is kept.
    pub(super) fn clean_staging(&self) {
        let receive = &self.store.settings().receive_dir;
        let legacy = receive.join(crate::consts::LEGACY_INCOMING_DIR);
        let mut removed = legacy.exists() && std::fs::remove_dir_all(&legacy).is_ok();
        let keep = self.kept_staging();
        let staging = receive.join(INCOMING_DIR);
        for e in std::fs::read_dir(&staging).into_iter().flatten().flatten() {
            if !keep.iter().any(|k| k.as_str() == e.file_name().to_string_lossy()) {
                let path = e.path();
                let gone = if path.is_dir() { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
                removed |= gone.is_ok();
            }
        }
        if keep.is_empty() {
            let _ = std::fs::remove_dir(&staging);
        }
        if removed {
            self.say("clean", "Removed unfinished files from an earlier run", None, Some(Tone::Dim));
        }
    }
}

/// "1 name changed so it works" / "7 names changed so they work"
fn renamed_words(n: u64) -> String {
    if n == 1 { "1 name changed so it works".into() } else { format!("{n} names changed so they work") }
}

/// What Details says when a transfer's progress can't be saved
pub(super) const SAVE_FAILED: &str = "Couldn’t save this transfer’s progress, so it can’t continue after a restart";

/// "Downloads/SeamDrop": the last two parts of a folder, for a story line
fn short_dir(p: &std::path::Path) -> String {
    let parts: Vec<_> = p.components().rev().take(2).map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// What a done screen names: "Docs/link.txt", "a link (shortcut): links aren’t sent"
fn skipped_items(notes: &[crate::proto::SkipNote]) -> Vec<ListedItem> {
    use crate::proto::SkipReason::*;
    let why = |r| match r {
        Symlink => "a link (shortcut): links aren’t sent",
        Junk => "a system file the other computer makes for itself",
        Unreadable => "couldn’t be read on the sending computer",
        Special => "not a regular file",
    };
    notes.iter().take(LISTED).map(|n| ListedItem { path: n.path.join("/"), why: why(n.reason).into() }).collect()
}

/// "Nasty/Résumé: Q3?.pdf", "saved as “Résumé_ Q3_.pdf”: ‘:’ and ‘?’ aren’t allowed in Windows names"
fn renamed_items(renames: &[crate::proto::RenameNote]) -> Vec<ListedItem> {
    renames
        .iter()
        .take(LISTED)
        .map(|r| ListedItem {
            path: r.from.join("/"),
            why: format!("saved as “{}”: {}", r.to.last().map(String::as_str).unwrap_or_default(), r.reason),
        })
        .collect()
}

/// "353 MB in 3.0 s, 118 MB/s; 280 MB was already there": only what
/// crossed the wire this time counts towards the speed.
fn moved_words(bytes: u64, kept: u64, secs: f64) -> String {
    let moved = bytes.saturating_sub(kept);
    let rate = format!("{}/s", size((moved as f64 / secs.max(0.001)) as u64));
    if kept > 0 {
        format!("{} in {secs:.1} s, {rate}; {} was already there", size(moved), size(kept))
    } else {
        format!("in {secs:.1} s, {rate}")
    }
}

fn skip_reason(r: crate::proto::SkipReason) -> &'static str {
    use crate::proto::SkipReason::*;
    match r {
        Symlink => "link (shortcut), skipped",
        Junk => "system file, skipped",
        Unreadable => "couldn’t be read, skipped",
        Special => "not a regular file, skipped",
    }
}

/// "2 links and 3 system files"
fn skip_words(notes: &[crate::proto::SkipNote]) -> String {
    use crate::proto::SkipReason::*;
    let count = |r| notes.iter().filter(|n| n.reason == r).count() as u64;
    let parts: Vec<String> = [
        (count(Symlink), "link", "links"),
        (count(Junk), "system file", "system files"),
        (count(Unreadable), "unreadable item", "unreadable items"),
        (count(Special), "special file", "special files"),
    ]
    .into_iter()
    .filter(|(n, _, _)| *n > 0)
    .map(|(n, one, many)| plural(n, one, many))
    .collect();
    match parts.as_slice() {
        [] => "nothing".into(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// "3 items", "Photos", "report.pdf"
pub(super) fn describe(offer: &Offer) -> String {
    match offer.items.as_slice() {
        [one] => one.name.clone(),
        many => format!("{} items", many.len()),
    }
}

/// "SeamDrop 012 – from Desk PC": the next number after the highest already
/// in `base` (so they sort in order of arrival), made now so two deliveries
/// can't take the same one. The device name is made safe on every system.
pub(super) fn numbered_folder(base: &std::path::Path, from: &str) -> Option<PathBuf> {
    let prefix = format!("{} ", crate::consts::APP_NAME);
    let highest = std::fs::read_dir(base)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.strip_prefix(&prefix)?.split(' ').next()?.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    std::fs::create_dir_all(base).ok()?;
    let who = crate::fsafe::sanitize(from.trim(), crate::fsafe::Target::Windows).name;
    for n in highest + 1..highest + 1000 {
        let name = if who.is_empty() { format!("{prefix}{n:03}") } else { format!("{prefix}{n:03} – from {who}") };
        let dir = base.join(name);
        match std::fs::create_dir(&dir) {
            Ok(()) => return Some(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        }
    }
    None
}
