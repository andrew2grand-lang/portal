//! What goes next to a busy device (Mission 05 §1.5).
//!
//! Files sent to a device that's busy with a transfer (one at a time per
//! device, decision 177) wait here instead of being refused: "Next for Desk
//! PC: 2 files · starts when the current transfer finishes". When that
//! device's slot comes free the earliest one is offered as usual (Ask every
//! time / Always accept still apply).
//!
//! Both computers can have something waiting for each other. Each tells the
//! other when its earliest item for it was queued (`queue_head`); when the
//! slot frees, the one queued earliest goes first, the lower device id on a
//! tie, and the other waits for the next free slot. An offer that crosses
//! anyway and is refused as busy goes back to the front.
//!
//! Kept in `<app data>/next.json`, so it survives a restart. Like a
//! transfer (§1.4), after a quit or a crash it waits for Continue; after an
//! update restart it goes on by itself.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Node;
use crate::events::{NextView, TransferPhase};
use crate::model::DeviceId;
use crate::proto::Msg;
use crate::transfer::journal::Stopped;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NextUp {
    pub id: u64,
    pub peer: DeviceId,
    pub paths: Vec<PathBuf>,
    /// When it was queued (unix ms): the earliest goes first.
    pub at: u64,
    /// Kept through a quit or a crash: waits for Continue.
    #[serde(default)]
    pub held: bool,
}

#[derive(Default, Serialize, Deserialize)]
struct Saved {
    items: Vec<NextUp>,
    /// Why the app last stopped (None: it didn't stop cleanly).
    stopped: Option<Stopped>,
}

#[derive(Default)]
pub(super) struct Queue {
    pub items: Vec<NextUp>,
    /// What each device said it has waiting for us (its earliest, unix ms).
    pub theirs: HashMap<DeviceId, Option<u64>>,
    /// What we last told each device.
    told: HashMap<DeviceId, Option<u64>>,
}

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Node {
    fn next_path(&self) -> PathBuf {
        self.cfg.data_dir.join("next.json")
    }

    /// At start: what was waiting, held if the app was quit or crashed.
    pub(super) fn load_next(&mut self) {
        let Ok(bytes) = std::fs::read(self.next_path()) else { return };
        let Ok(saved) = serde_json::from_slice::<Saved>(&bytes) else { return };
        let asks = saved.stopped.is_none_or(Stopped::asks);
        self.next.items = saved
            .items
            .into_iter()
            .filter(|n| self.trusts(&n.peer))
            .map(|mut n| {
                n.held |= asks;
                n
            })
            .collect();
        self.next_id = self.next.items.iter().map(|n| n.id).max().unwrap_or(0);
    }

    /// Writes the queue; `stopped` only when the app stops cleanly.
    pub(super) fn save_next(&self, stopped: Option<Stopped>) {
        let saved = Saved { items: self.next.items.clone(), stopped };
        if let Err(e) = crate::store::write_json(&self.next_path(), &saved) {
            tracing::warn!("couldn't save what's next: {e}");
        }
    }

    /// Files for a busy device: they go next.
    pub(super) fn queue_next(&mut self, peer: DeviceId, paths: Vec<PathBuf>) {
        self.next_id += 1;
        let name = self.store.peer(&peer).map(|p| p.name.clone()).unwrap_or_default();
        self.next.items.push(NextUp { id: self.next_id, peer: peer.clone(), paths, at: now_ms(), held: false });
        self.say("next", format!("Next for {name}: it starts when the current transfer finishes"), None, None);
        self.next_changed(&peer);
    }

    /// ✕ on one waiting item.
    pub(super) fn cmd_unqueue_next(&mut self, id: u64) {
        let Some(i) = self.next.items.iter().position(|n| n.id == id) else { return };
        let peer = self.next.items.remove(i).peer;
        self.next_changed(&peer);
    }

    /// Continue on items kept through a quit (all for that device).
    pub(super) fn cmd_continue_next(&mut self, peer: DeviceId) {
        for n in self.next.items.iter_mut().filter(|n| n.peer == peer) {
            n.held = false;
        }
        self.next_changed(&peer);
    }

    fn next_changed(&mut self, peer: &DeviceId) {
        self.save_next(None);
        self.tell_next(peer);
        self.dirty = true;
        self.start_next();
    }

    /// Our earliest for `peer` that isn't held.
    fn our_head(&self, peer: &DeviceId) -> Option<&NextUp> {
        self.next.items.iter().filter(|n| &n.peer == peer && !n.held).min_by_key(|n| (n.at, n.id))
    }

    /// Tells `peer` when our earliest for it was queued, if that changed
    /// (and always on a fresh link: `force`).
    pub(super) fn tell_next(&mut self, peer: &DeviceId) {
        self.tell_next_inner(peer, false);
    }

    pub(super) fn tell_next_on_link(&mut self, peer: &DeviceId) {
        self.tell_next_inner(peer, true);
    }

    fn tell_next_inner(&mut self, peer: &DeviceId, force: bool) {
        // One of ours from the queue is still being prepared: say nothing
        // until its offer is on the wire, or the other side would see a free
        // slot with nothing ahead and start its own (both would offer).
        if self
            .transfers
            .iter()
            .any(|t| &t.peer == peer && t.from_next.is_some() && t.phase == TransferPhase::Preparing)
        {
            return;
        }
        let head = self.our_head(peer).map(|n| n.at);
        if !force && self.next.told.get(peer) == Some(&head) {
            return;
        }
        if let Some(link) = self.link_for(peer)
            && link.try_send(Msg::QueueHead { at: head })
        {
            self.next.told.insert(peer.clone(), head);
        }
    }

    /// `queue_head` from a trusted device.
    pub(super) fn on_queue_head(&mut self, peer: DeviceId, at: Option<u64>) {
        self.next.theirs.insert(peer, at);
        self.start_next();
    }

    /// Starts what's next wherever a slot is free and it's our turn.
    pub(super) fn start_next(&mut self) {
        let me = self.identity.device_id.clone();
        let peers: Vec<DeviceId> = {
            let mut p: Vec<DeviceId> = self.next.items.iter().map(|n| n.peer.clone()).collect();
            p.sort();
            p.dedup();
            p
        };
        for peer in peers {
            let Some(ours) = self.our_head(&peer).cloned() else { continue };
            if self.active_with(&peer).is_some() || self.link_for(&peer).is_none() {
                continue;
            }
            // Theirs was queued earlier (or as early, and their id is lower): wait.
            if let Some(Some(theirs)) = self.next.theirs.get(&peer)
                && (*theirs, &peer) < (ours.at, &me)
            {
                continue;
            }
            self.next.items.retain(|n| n.id != ours.id);
            self.save_next(None);
            let name = self.store.peer(&peer).map(|p| p.name.clone()).unwrap_or_default();
            match self.cmd_send(peer.clone(), ours.paths.clone()) {
                Ok(()) => {
                    self.say("next", format!("Starting what was next for {name}"), None, None);
                    if let Some(t) = self.transfers.iter_mut().rev().find(|t| t.peer == peer && t.active()) {
                        t.from_next = Some(ours);
                    }
                }
                Err(e) => self.say("next", format!("What was next for {name} couldn’t start"), Some(e), None),
            }
            // Told once its offer is out (`on_planned`, or the next tick).
            self.tell_next(&peer);
            self.dirty = true;
        }
    }

    /// Our offer from the queue crossed theirs and was refused as busy: it
    /// goes back to the front.
    pub(super) fn requeue_next(&mut self, n: NextUp) {
        let peer = n.peer.clone();
        self.next.items.push(n);
        self.save_next(None);
        self.tell_next(&peer);
        self.dirty = true;
    }

    /// Every tick: tell each connected device if our head for it changed.
    pub(super) fn tell_next_all(&mut self) {
        let peers: Vec<DeviceId> = self.links.values().map(|l| l.info().hello.id.clone()).collect();
        for p in peers {
            if self.trusts(&p) {
                self.tell_next(&p);
            }
        }
    }

    /// Forgets everything waiting for a device (unpaired).
    pub(super) fn forget_next_for(&mut self, peer: &DeviceId) {
        let before = self.next.items.len();
        self.next.items.retain(|n| &n.peer != peer);
        self.next.theirs.remove(peer);
        self.next.told.remove(peer);
        if self.next.items.len() != before {
            self.save_next(None);
            self.dirty = true;
        }
    }

    pub(super) fn next_views(&self) -> Vec<NextView> {
        let mut items: Vec<&NextUp> = self.next.items.iter().collect();
        items.sort_by_key(|n| (n.at, n.id));
        items
            .into_iter()
            .map(|n| {
                let busy = self.active_with(&n.peer);
                NextView {
                    id: n.id,
                    peer_id: n.peer.clone(),
                    peer_name: self.store.peer(&n.peer).map(|p| p.name.clone()).unwrap_or_default(),
                    names: n
                        .paths
                        .iter()
                        .map(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default())
                        .collect(),
                    folders: n.paths.iter().filter(|p| p.is_dir()).count() as u64,
                    held: n.held,
                    waiting_for_paused: busy.is_some_and(|t| t.phase == TransferPhase::Paused),
                }
            })
            .collect()
    }
}
