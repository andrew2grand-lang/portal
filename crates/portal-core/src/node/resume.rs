//! Pausing and resuming transfers (Mission 03 §2).
//!
//! A drop pauses a running transfer on both sides (the receiver keeps its
//! staging folder and a saved state; the sender keeps its file list). When a
//! trusted link to the same device comes back, the sender asks and the
//! receiver answers with what it already has; the sender continues from
//! there. No new prompt: the original accept still counts. Paused transfers
//! survive restarts (every one is loaded from the journal at start) and
//! expire after `Config::resume_for`. With one device, paused transfers
//! continue one after another (one transfer at a time per device).

use tokio::sync::{mpsc, watch};

use super::xfer::{Transfer, describe, find, find_mut, size};
use super::{Internal, Node};
use crate::consts::{APP_NAME, INCOMING_DIR};
use crate::events::{Direction, NoticeKind, Tone, TransferPhase};
use crate::model::DeviceId;
use crate::net::link::Link;
use crate::proto::{CancelReason, Msg, TransferId};
use crate::transfer::journal::{Ending, InState, OutState, Saved, Stopped, now_secs};
use crate::transfer::plan::Planned;
use crate::transfer::send::Start;
use crate::transfer::{Failure, recv, send};

/// A question about a paused transfer with no answer is asked again after this long.
const ASK_AGAIN: std::time::Duration = std::time::Duration::from_secs(5);

/// What the receiver found when it checked its partial data.
pub(super) struct Restored {
    pub state: InState,
    pub hasher: Option<blake3::Hasher>,
    pub note: Option<String>,
}

impl Node {
    // ---- starting a run ------------------------------------------------------

    /// Receiver: streams into `state` (fresh or restored) over `link`.
    pub(super) fn spawn_recv(
        &mut self,
        tid: TransferId,
        link: &Link,
        mut state: InState,
        hasher: Option<blake3::Hasher>,
    ) {
        // Running again: the checkpoints it saves aren't paused.
        state.paused_at = None;
        state.stopped = None;
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        let (sink_tx, sink_rx) = mpsc::channel(64);
        link.set_sink(Some(sink_tx));
        let (cancel_tx, cancel_rx) = watch::channel(false);
        t.cancel = Some(cancel_tx);
        t.link = Some(link.id());
        t.progress.set(state.bytes());
        t.progress.started.store(false, std::sync::atomic::Ordering::Relaxed);
        t.run();
        let ctx = recv::RecvCtx {
            link: link.clone(),
            inbound: sink_rx,
            cancel: cancel_rx,
            progress: t.progress.clone(),
            journal: self.journal.path(state.tid),
            state,
            hasher,
            write_limit: self.cfg.fault_write_limit,
            stall: self.cfg.stall,
        };
        let (tid, internal) = (t.tid, self.internal_tx.clone());
        tokio::spawn(async move {
            let outcome = recv::run(ctx).await;
            let _ = internal.send(Internal::Received(tid, outcome)).await;
        });
        self.dirty = true;
    }

    /// Sender: streams the planned files from `start` over `link`.
    pub(super) fn spawn_send(&mut self, tid: TransferId, link: &Link, start: Start, bytes: u64) {
        let here = self.this_computer();
        let faults =
            send::Faults { cut_at: self.cut.clone(), rate: self.cfg.fault_rate, hash_rate: self.cfg.fault_hash_rate };
        let stall = self.cfg.stall;
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        let Some(planned) = t.planned.clone() else { return };
        self.journal.set_running(t.tid);
        let (sink_tx, sink_rx) = mpsc::channel(64);
        link.set_sink(Some(sink_tx));
        let (cancel_tx, cancel_rx) = watch::channel(false);
        t.cancel = Some(cancel_tx);
        t.link = Some(link.id());
        t.progress.set(bytes);
        t.progress.set_acked(bytes);
        t.run();
        let ctx = send::SendCtx {
            link: link.clone(),
            planned,
            start,
            inbound: sink_rx,
            cancel: cancel_rx,
            progress: t.progress.clone(),
            here,
            faults,
            stall,
        };
        let (tid, internal) = (t.tid, self.internal_tx.clone());
        tokio::spawn(async move {
            let outcome = send::run(ctx).await;
            let _ = internal.send(Internal::Sent(tid, outcome)).await;
        });
        let kind = self.link_kind(&link.info().remote, Some(link.info().local.ip()), None);
        if let Some(t) = find_mut(&mut self.transfers, tid) {
            t.link_kind = Some(kind);
        }
        self.dirty = true;
    }

    /// The sender's saved state, written when the receiver accepts.
    pub(super) fn save_out(&self, t: &Transfer, planned: &Planned) {
        let state = OutState {
            tid: t.tid,
            peer: t.peer.clone(),
            peer_name: t.peer_name.clone(),
            paths: t.paths.clone(),
            offer: planned.offer.clone(),
            dirs: planned.dirs.clone(),
            files: planned.files.clone(),
            skipped: planned.skipped.clone(),
            bytes: 0,
            paused_at: None,
            stopped: None,
        };
        if let Err(e) = self.journal.save(&Saved::Out(state)) {
            self.say("pause", super::xfer::SAVE_FAILED, Some(e.to_string()), Some(Tone::Warn));
        }
    }

    // ---- pausing -------------------------------------------------------------

    /// The connection went: keep everything and wait for the device to come back.
    pub(super) fn pause(&mut self, tid: TransferId) {
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        let at = now_secs();
        t.phase = TransferPhase::Paused;
        t.paused_at = Some(at);
        t.resuming = false;
        t.awake = None;
        t.stalled = false;
        t.cancel = None;
        t.link = None;
        let (name, pct, bytes, total) = (t.peer_name.clone(), t.percent(), t.moved_bytes(), t.total());
        let (peer, dir) = (t.peer.clone(), t.dir);
        // Quitting or restarting to update says so; anything else is the network.
        self.journal.set_paused(tid, at, bytes, self.stopping.unwrap_or(Stopped::Network));
        self.say(
            "pause",
            format!("Lost the connection to {name} at {pct} %; it continues when {name} is back"),
            Some(format!("{} of {}", size(bytes), size(total))),
            Some(Tone::Warn),
        );
        self.notice(
            NoticeKind::Paused,
            "Transfer paused".into(),
            format!("Lost the connection to {name}. It continues by itself when {name} is back."),
        );
        self.dirty = true;
        // The other side already moved to a new link: continue at once.
        let pending = find_mut(&mut self.transfers, tid).and_then(|t| t.pending.take());
        if let Some((link_id, start, have)) = pending
            && dir == Direction::Out
            && let Some(link) = self.links.get(&link_id).cloned().filter(|l| !l.is_closed())
        {
            self.continue_sending(tid, &link, start, have);
        } else if self.link_for(&peer).is_some() {
            self.peer_back(&peer);
        }
    }

    /// A trusted link to `peer` is up, or its slot came free: pick up a
    /// transfer paused with it (the oldest first; one at a time per device).
    pub(super) fn peer_back(&mut self, peer: &DeviceId) {
        let Some(link) = self.link_for(peer) else { return };
        self.announce_holds(peer, &link);
        let busy = self.transfers.iter().any(|t| {
            &t.peer == peer
                && (matches!(t.phase, TransferPhase::Running | TransferPhase::Offered | TransferPhase::Preparing)
                    || t.resuming)
        });
        if busy {
            return;
        }
        // Held ones (here or there) wait for a person; the next one can go.
        let Some(t) = self
            .transfers
            .iter_mut()
            .find(|t| &t.peer == peer && t.phase == TransferPhase::Paused && !t.stopped_here && !t.waiting_there)
        else {
            return;
        };
        let tid = t.tid;
        let first = t.asked.is_none();
        t.asked = Some(std::time::Instant::now());
        match t.dir {
            Direction::Out => {
                link.try_send(Msg::ResumeAsk { tid: t.tid });
                let name = t.peer_name.clone();
                if first {
                    self.say("resume", format!("{name} is back; asking what already arrived"), None, None);
                }
            }
            Direction::In if !t.resuming => self.start_restore(tid),
            Direction::In => {}
        }
    }

    /// Tells `peer` about transfers held here (quit or crashed mid-way):
    /// "waiting for the person here", with what the receiver kept.
    fn announce_holds(&mut self, peer: &DeviceId, link: &Link) {
        for t in self.transfers.iter_mut().filter(|t| &t.peer == peer && t.phase == TransferPhase::Paused) {
            if !t.stopped_here || t.asked.is_some_and(|a| a.elapsed() < ASK_AGAIN) {
                continue;
            }
            t.asked = Some(std::time::Instant::now());
            link.try_send(Msg::Hold { tid: t.tid, on: true });
            if t.dir == Direction::In {
                link.try_send(Msg::Kept { tid: t.tid, bytes: t.progress.bytes() });
            }
        }
    }

    /// `hold` from a trusted link: the other side is waiting for its person
    /// (`on`), or they pressed Continue there.
    pub(super) fn on_hold(&mut self, link: &Link, tid: TransferId, on: bool) {
        let peer = link.info().hello.id.clone();
        let Some(t) = find_mut(&mut self.transfers, tid).filter(|t| t.peer == peer && t.active()) else {
            // Ended here while that computer was away (cancelled, say): it
            // isn't asking, so tell it now.
            if on {
                // "Done" here means the receiver finished, so it's news only
                // to a sender; a held receiver can't have finished.
                self.tell_ending(link, tid, true);
            }
            return;
        };
        let (dir, phase, name) = (t.dir, t.phase, t.peer_name.clone());
        if on {
            if t.waiting_there {
                return;
            }
            t.waiting_there = true;
            self.dirty = true;
            self.say(
                "resume",
                format!("{name} stopped partway there; waiting for someone there to continue"),
                None,
                None,
            );
            // A receiver that already started again on this link stops
            // quietly (no cancel): dropping its queue pauses it.
            if dir == Direction::In && phase == TransferPhase::Running {
                link.set_sink(None);
            }
            if dir == Direction::In {
                let bytes = find(&self.transfers, tid).map_or(0, |t| t.progress.bytes());
                link.try_send(Msg::Kept { tid, bytes });
            }
        } else if t.waiting_there {
            t.waiting_there = false;
            self.dirty = true;
            self.say("resume", format!("{name} continues the transfer"), None, None);
            self.peer_back(&peer);
        }
    }

    /// `kept` from the receiver, for a paused transfer here (a held sender
    /// after a crash learns its percentage this way).
    pub(super) fn on_kept(&mut self, link: &Link, tid: TransferId, bytes: u64) {
        let peer = link.info().hello.id.clone();
        let Some(t) = find_mut(&mut self.transfers, tid)
            .filter(|t| t.peer == peer && t.phase == TransferPhase::Paused && t.dir == Direction::Out)
        else {
            return;
        };
        t.progress.set(bytes);
        t.progress.set_acked(bytes);
        t.percent_known = true;
        self.dirty = true;
    }

    /// Continue pressed on a transfer held here.
    pub(super) fn continue_held(&mut self, tid: TransferId) {
        let Some(t) = find_mut(&mut self.transfers, tid).filter(|t| t.stopped_here) else { return };
        t.stopped_here = false;
        t.asked = None;
        let (peer, name) = (t.peer.clone(), t.peer_name.clone());
        self.say("resume", format!("Continuing the transfer with {name}"), None, None);
        self.dirty = true;
        if let Some(link) = self.link_for(&peer) {
            link.try_send(Msg::Hold { tid, on: false });
        }
        self.peer_back(&peer);
    }

    /// Every second: expiry, and asking again when a question about a
    /// paused transfer went unanswered (it can be lost with a link that
    /// closes as two computers connect to each other at once).
    pub(super) fn resume_tick(&mut self) {
        self.check_expiry();
        let due: Vec<DeviceId> = self
            .transfers
            .iter()
            .filter(|t| t.phase == TransferPhase::Paused && !t.resuming)
            .filter(|t| t.asked.is_none_or(|a| a.elapsed() >= ASK_AGAIN))
            .map(|t| t.peer.clone())
            .collect();
        for peer in due {
            self.peer_back(&peer);
        }
    }

    /// Receiver: re-checks the partial data off the actor, then continues.
    fn start_restore(&mut self, tid: TransferId) {
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        t.resuming = true;
        self.dirty = true;
        let (tid, journal, internal) = (t.tid, self.journal.clone(), self.internal_tx.clone());
        tokio::task::spawn_blocking(move || {
            let restored = match journal.load(tid) {
                Some(Saved::In(state)) => {
                    let mut state = *state;
                    let (hasher, note) = recv::restore(&mut state);
                    Ok(Restored { state, hasher, note })
                }
                _ => Err("the saved state is missing".to_owned()),
            };
            let _ = internal.blocking_send(Internal::Restored(tid, Box::new(restored)));
        });
    }

    pub(super) fn on_restored(&mut self, tid: TransferId, restored: Result<Restored, String>) {
        let Some(t) = find_mut(&mut self.transfers, tid).filter(|t| t.phase == TransferPhase::Paused) else {
            return;
        };
        t.resuming = false;
        let (peer, name) = (t.peer.clone(), t.peer_name.clone());
        let r = match restored {
            Ok(r) => r,
            Err(e) => {
                self.say(
                    "resume",
                    format!("Couldn’t continue with {name}: what had arrived couldn’t be read back"),
                    Some(e.to_string()),
                    Some(Tone::Err),
                );
                return self.end_paused(tid, Ending::Cancelled, format!("The part that had arrived from {name} couldn’t be read back, so it was removed. Send it again from {name}."));
            }
        };
        if let Some(note) = &r.note {
            self.say("resume", note.clone(), None, None);
            if let Some(t) = find_mut(&mut self.transfers, tid) {
                t.notes.push(note.clone());
            }
        }
        let Some(link) = self.link_for(&peer) else {
            self.dirty = true;
            return; // gone again: stays paused
        };
        if find(&self.transfers, tid).is_some_and(|t| t.waiting_there || t.stopped_here) {
            self.dirty = true;
            return; // held by a person, here or there
        }
        let state = r.state;
        let total = state.offer.total_bytes;
        let prefix = state.partial.as_ref().filter(|p| p.written > 0).map(|p| p.hash);
        let msg = Msg::ResumeFrom {
            tid,
            dirs: state.dirs,
            file: state.done,
            offset: state.offset(),
            bytes: state.bytes(),
            prefix,
        };
        self.say(
            "resume",
            format!("Resuming at {} (nothing sent twice)", size(state.bytes())),
            Some(format!(
                "{} of {} here, asking {name} to continue from file {} of {}",
                size(state.bytes()),
                size(total),
                state.done + 1,
                state.offer.file_count.max(1)
            )),
            None,
        );
        let kept = state.bytes();
        self.spawn_recv(tid, &link, state, r.hasher);
        link.try_send(msg.clone());
        if let Some(t) = find_mut(&mut self.transfers, tid) {
            t.resume_from = Some(msg);
            t.kept = Some(kept);
        }
    }

    /// Sender: the receiver says what it has; continue from there.
    fn continue_sending(&mut self, tid: TransferId, link: &Link, start: Start, have: u64) {
        let Some(t) = find(&self.transfers, tid) else { return };
        let total = t.total();
        let files = t.offer.as_ref().map_or(0, |o| o.file_count);
        let pct = (have * 100).checked_div(total).unwrap_or(0).min(100);
        self.say(
            "resume",
            format!("Resumed at {} (nothing sent twice)", size(have)),
            Some(format!(
                "{} has {} of {}, continuing from {pct} % (file {} of {})",
                t.peer_name,
                size(have),
                size(total),
                (start.file + 1).min(files.max(1)),
                files.max(1)
            )),
            None,
        );
        self.spawn_send(tid, link, start, have);
        if let Some(t) = find_mut(&mut self.transfers, tid) {
            t.kept = Some(have);
        }
    }

    /// `resume_ask` / `resume_from` from a trusted link.
    pub(super) fn on_resume_msg(&mut self, link: &Link, msg: Msg) {
        let peer = link.info().hello.id.clone();
        let (tid, from) = match &msg {
            Msg::ResumeAsk { tid } => (*tid, None),
            Msg::ResumeFrom { tid, dirs, file, offset, bytes, prefix } => {
                (*tid, Some((Start { dirs: *dirs, file: *file, offset: *offset, prefix: *prefix }, *bytes)))
            }
            _ => return,
        };
        // Only a transfer that's still under way answers; one that ended here
        // (cancelled, failed, done) answers from its note in the journal.
        let ours =
            find(&self.transfers, tid).filter(|t| t.peer == peer && t.active()).map(|t| (t.dir, t.phase, t.link));
        match (ours, from) {
            // Sender, held by its person: not yet.
            (Some((Direction::Out, TransferPhase::Paused, _)), Some(_))
                if find(&self.transfers, tid).is_some_and(|t| t.stopped_here) =>
            {
                link.try_send(Msg::Hold { tid, on: true });
            }
            // Sender: the receiver is ready.
            (Some((Direction::Out, TransferPhase::Paused, _)), Some((start, have))) => {
                self.continue_sending(tid, link, start, have)
            }
            // Sender, still winding down a run on a link the receiver has left.
            (Some((Direction::Out, TransferPhase::Running, Some(old))), Some((start, have))) if old != link.id() => {
                if let Some(t) = find_mut(&mut self.transfers, tid) {
                    t.pending = Some((link.id(), start, have));
                }
                if let Some(old) = self.links.get(&old) {
                    old.close();
                }
            }
            // Receiver: the sender is back and asking.
            (Some((Direction::In, TransferPhase::Paused, _)), None) => self.peer_back(&peer),
            (Some((Direction::In, TransferPhase::Running, Some(old))), None) if old != link.id() => {
                // Its old link is dead: pausing restarts from here.
                if let Some(old) = self.links.get(&old) {
                    old.close();
                }
            }
            // Asked again on the same link. Nothing heard since: our answer
            // went missing, so repeat it. Otherwise it's out of date and the
            // two sides are out of step: a fresh connection resumes from
            // what's really on disk.
            (Some((Direction::In, TransferPhase::Running, Some(_))), None) => {
                let Some(t) = find(&self.transfers, tid) else { return };
                if t.progress.started.load(std::sync::atomic::Ordering::Relaxed) {
                    let name = t.peer_name.clone();
                    self.say(
                        "resume",
                        format!("{name} asked again mid-transfer; reconnecting to pick up from here"),
                        None,
                        Some(Tone::Dim),
                    );
                    link.close();
                } else if let Some(again) = t.resume_from.clone() {
                    link.try_send(again);
                }
            }
            (Some(_), _) => {}
            // Not (or no longer) here: say how it ended.
            (None, _) => self.tell_ending(link, tid, from.is_none()),
        }
    }

    /// A transfer the other side still has that isn't under way here: say
    /// how it ended (`done` only to a sender asking: a receiver can't use it).
    fn tell_ending(&self, link: &Link, tid: TransferId, to_sender: bool) {
        let reply = match self.journal.ending(tid) {
            Some(Ending::Done { summary }) if to_sender => Msg::Complete { tid, summary },
            Some(Ending::Cancelled) => Msg::Cancel { tid, reason: CancelReason::User },
            Some(Ending::Expired) => {
                Msg::Cancel { tid, reason: CancelReason::Error { code: "expired".into(), msg: String::new() } }
            }
            _ => Msg::Cancel { tid, reason: CancelReason::Error { code: "unknown".into(), msg: String::new() } },
        };
        link.try_send(reply);
    }

    // ---- ending a paused transfer --------------------------------------------

    /// Ends the paused transfer here: partial files and saved state go; the
    /// window shows `message`.
    pub(super) fn end_paused(&mut self, tid: TransferId, ending: Ending, message: String) {
        let Some(dir) = find(&self.transfers, tid).map(|t| t.dir) else { return };
        if dir == Direction::In {
            self.drop_partial(tid);
        }
        let Some(t) = find_mut(&mut self.transfers, tid) else { return };
        self.journal.end(t.tid, &t.peer, ending);
        t.phase = TransferPhase::Failed;
        t.failure = Some(message);
        t.resuming = false;
        t.paused_at = None;
        self.dirty = true;
    }

    /// Cancel pressed while paused: it ends here, and the other side hears
    /// when it's back (or now, if it's connected).
    pub(super) fn cancel_paused(&mut self, tid: TransferId) {
        let Some(t) = find(&self.transfers, tid) else { return };
        let (peer, dir) = (t.peer.clone(), t.dir);
        if let Some(link) = self.link_for(&peer) {
            link.try_send(Msg::Cancel { tid, reason: CancelReason::User });
        }
        if dir == Direction::In {
            self.drop_partial(tid);
        }
        self.journal.end(tid, &peer, Ending::Cancelled);
        let what = if dir == Direction::In { "partial files removed" } else { "nothing more will be sent" };
        self.say("cancel", format!("Cancelled the paused transfer; {what}"), None, None);
        let name = find(&self.transfers, tid).map(|t| t.peer_name.clone()).unwrap_or_default();
        let text = self.cancelled_text(dir, &name);
        if let Some(t) = find_mut(&mut self.transfers, tid) {
            t.phase = TransferPhase::Cancelled;
            t.failure = Some(text);
            t.paused_at = None;
        }
        self.dirty = true;
        // Another transfer paused with the same device can go now.
        self.peer_back(&peer);
    }

    /// The other side said, while this one was paused, that it's over.
    pub(super) fn paused_ended_there(&mut self, tid: TransferId, reason: CancelReason) {
        let Some(t) = find(&self.transfers, tid) else { return };
        let f = match reason {
            CancelReason::User => Failure::CancelledThere { code: None, msg: String::new() },
            CancelReason::Error { code, msg } => Failure::CancelledThere { code: Some(code), msg },
        };
        let text = self.failure_text(&f, t.dir, &t.peer_name.clone(), t.percent());
        self.say("cancel", format!("{} ended the paused transfer", t.peer_name), Some(f.detail()), Some(Tone::Warn));
        self.end_paused(tid, Ending::Cancelled, text);
    }

    /// Every second: a transfer paused for longer than `resume_for` ends.
    pub(super) fn check_expiry(&mut self) {
        let limit = self.cfg.resume_for.as_secs().max(1);
        let expired: Vec<TransferId> = self
            .transfers
            .iter()
            .filter(|t| t.phase == TransferPhase::Paused)
            .filter(|t| t.paused_at.is_some_and(|at| now_secs().saturating_sub(at) >= limit))
            .map(|t| t.tid)
            .collect();
        for tid in expired {
            self.expire(tid);
        }
    }

    fn expire(&mut self, tid: TransferId) {
        let Some(t) = find(&self.transfers, tid) else { return };
        let (name, what, dir) = (t.peer_name.clone(), t.offer.as_ref().map(describe).unwrap_or_default(), t.dir);
        let how_long = human(self.cfg.resume_for);
        let message = match dir {
            Direction::Out => format!(
                "Paused for more than {how_long}, so {APP_NAME} stopped sending {what}. Nothing was saved on {name}. Send it again."
            ),
            Direction::In => format!(
                "Paused for more than {how_long}, so {APP_NAME} removed the part of {what} that had arrived. Send it again from {name}."
            ),
        };
        self.say(
            "expire",
            format!("Paused for more than {how_long}, so the part that had arrived was removed"),
            None,
            Some(Tone::Warn),
        );
        self.end_paused(tid, Ending::Expired, message);
    }

    // ---- at start ----------------------------------------------------------

    /// Picks up every transfer that was running or paused when the app last
    /// stopped (oldest first). None is dropped: with one device they
    /// continue one after another.
    pub(super) fn load_paused(&mut self) {
        let keep_notes = self.cfg.resume_for.as_secs() + 3600;
        let mut live: Vec<(Saved, u64)> = Vec::new();
        for (saved, modified) in self.journal.load_all() {
            match &saved {
                Saved::Ended(e) if now_secs().saturating_sub(e.at) > keep_notes => self.journal.remove(e.tid),
                Saved::Ended(_) => {}
                _ => live.push((saved, modified)),
            }
        }
        live.sort_by_key(|(_, modified)| *modified);
        for (saved, modified) in live {
            self.load_one(saved, modified);
        }
        self.check_expiry();
        self.dirty = true;
    }

    fn load_one(&mut self, saved: Saved, modified: u64) {
        let mut t = match saved {
            Saved::Out(s) => {
                let mut t = Transfer::new(s.tid, Direction::Out, s.peer.clone(), s.peer_name.clone());
                let why = Stopped::of(s.stopped, s.paused_at);
                t.stopped_here = why.asks();
                // After a crash the bytes saved are from when it started:
                // the receiver says what it kept once it's connected.
                t.percent_known = why != Stopped::Crash;
                t.paths = s.paths.clone();
                t.progress.set(s.bytes);
                t.progress.set_acked(s.bytes);
                // Running when the app stopped: its saved state is from when
                // it started, so the pause counts from now.
                t.paused_at = Some(s.paused_at.unwrap_or_else(now_secs));
                t.offer = Some(s.offer.clone());
                t.planned = Some(Planned { offer: s.offer, dirs: s.dirs, files: s.files, skipped: s.skipped });
                t
            }
            Saved::In(s) => {
                let mut t = Transfer::new(s.tid, Direction::In, s.peer.clone(), s.peer_name.clone());
                t.stopped_here = Stopped::of(s.stopped, s.paused_at).asks();
                t.progress.set(s.bytes());
                // Running when the app stopped: saved every second, so the
                // last save is when it stopped.
                t.paused_at = Some(s.paused_at.unwrap_or(modified));
                t.auto_accepted = s.auto_accepted;
                t.save_to = Some(s.receive_dir.clone());
                t.offer = Some(s.offer);
                t
            }
            Saved::Ended(_) => return,
        };
        // Trusted still? A device removed since can't resume anything.
        if !self.trusts(&t.peer) {
            if let Some(Saved::In(s)) = self.journal.load(t.tid) {
                recv::cleanup(&s.staging);
            }
            self.journal.remove(t.tid);
            return;
        }
        if let Some(p) = self.store.peer(&t.peer) {
            t.peer_name = p.name.clone();
        }
        t.phase = TransferPhase::Paused;
        let (name, pct, held) = (t.peer_name.clone(), t.percent(), t.stopped_here);
        self.transfers.push(t);
        let text = if held {
            format!("A transfer with {name} stopped partway when {APP_NAME} last closed; it waits for Continue")
        } else {
            format!("A transfer with {name} is paused at {pct} %; it continues when {name} is back")
        };
        self.say("pause", text, None, None);
    }

    /// Removes an incoming transfer's partial files (where its saved state
    /// says they are, else where they'd be now).
    pub(super) fn drop_partial(&self, tid: TransferId) {
        let staging = match self.journal.load(tid) {
            Some(Saved::In(state)) => state.staging,
            _ => self.store.settings().receive_dir.join(INCOMING_DIR).join(format!("{tid:016x}")),
        };
        recv::cleanup(&staging);
    }

    /// The staging folders paused incoming transfers still need.
    pub(super) fn kept_staging(&self) -> Vec<String> {
        self.transfers
            .iter()
            .filter(|t| t.dir == Direction::In && t.phase == TransferPhase::Paused)
            .map(|t| format!("{:016x}", t.tid))
            .collect()
    }

    /// A device can no longer send to us (removed, or set to Never): a
    /// transfer paused with it can't resume.
    pub(super) fn drop_paused_with(&mut self, peer: &DeviceId, dir: Option<Direction>, why: &str) {
        let paused: Vec<(TransferId, String)> = self
            .transfers
            .iter()
            .filter(|t| &t.peer == peer && t.phase == TransferPhase::Paused && dir.is_none_or(|d| d == t.dir))
            .map(|t| (t.tid, t.peer_name.clone()))
            .collect();
        for (tid, name) in paused {
            self.say("cancel", format!("The paused transfer with {name} ended: {why}"), None, Some(Tone::Warn));
            self.cancel_paused(tid);
        }
    }
}

/// "24 hours", "2 seconds"
fn human(d: std::time::Duration) -> String {
    let s = d.as_secs();
    let (n, unit) = if s >= 3600 && s.is_multiple_of(3600) {
        (s / 3600, "hour")
    } else if s >= 60 && s.is_multiple_of(60) {
        (s / 60, "minute")
    } else {
        (s, "second")
    };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}
