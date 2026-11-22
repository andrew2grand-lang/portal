//! Trusted devices: keeping a link to each one, and settings that touch trust.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use super::{Node, Purpose};
use crate::events::{ReachStep, Tone};
use crate::model::{DeviceId, Incoming, Role};
use crate::net::discovery::Via;
use crate::net::link::Link;
use crate::proto::{ErrorCode, Msg};

/// The device with the larger id waits a little, so both sides rarely dial at once.
const FOLLOWER_DELAY: Duration = Duration::from_millis(1500);
/// Don't keep re-dialling the same better path if it doesn't answer.
const UPGRADE_EVERY: Duration = Duration::from_secs(10);
/// A sighting this recent counts as "found just now".
const FRESH: Duration = Duration::from_secs(10);
/// How often to check back with a device that disconnected from us.
const REMOTE_DISCONNECTED_WAIT: Duration = Duration::from_secs(60);

/// Partial settings change from the UI.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub device_name: Option<String>,
    pub receive_dir: Option<std::path::PathBuf>,
    pub skip_junk: Option<bool>,
    pub auto_update: Option<bool>,
    /// "Get test builds"
    pub test_builds: Option<bool>,
    /// On: agree to sync with every paired device; off: "Turn off clipboard
    /// sync" (with every device). Per device: `Core::set_clip_sync`.
    pub clipboard_sync: Option<bool>,
    /// The Send clipboard shortcut ("" puts the default back).
    pub clipboard_shortcut: Option<String>,
    /// Where files and the clipboard go: the devices ticked (remembered).
    pub send_to: Option<Vec<DeviceId>>,
    /// System, Light or Dark
    pub appearance: Option<crate::model::Appearance>,
    /// "A folder for each device"
    pub folder_per_device: Option<bool>,
    /// Mission 05 §4: how received files are put away, and their dates.
    pub receive_mode: Option<crate::store::ReceiveMode>,
    pub keep_dates: Option<bool>,
    /// The one-time note about received files was read.
    pub clear_receive_note: bool,
}

impl Node {
    /// Dials every trusted device that has no link yet (pinned to its key).
    pub(super) fn maintain_links(&mut self) {
        let now = Instant::now();
        let trusted: Vec<_> = self.store.peers().iter().filter(|p| p.trusted && !p.disconnected).cloned().collect();
        for t in trusted {
            if self.link_for(&t.id).is_some() || self.dialing.contains(&t.id) || !self.reach.due(&t.id, now) {
                continue;
            }
            let follower = self.identity.device_id > t.id;
            if follower && self.started.elapsed() < FOLLOWER_DELAY {
                continue;
            }
            // Fresh sightings first (best path first), then last-known addresses.
            let mut candidates: Vec<SocketAddr> = Vec::new();
            let mut found = false;
            if let Some(p) = self.peers.get(&t.id) {
                let mut paths: Vec<_> = p.paths.iter().collect();
                paths.sort_by_key(|x| self.path_rank(x));
                candidates.extend(paths.iter().map(|x| x.addr));
                found = p.paths.iter().any(|x| x.seen.elapsed() < FRESH && matches!(x.via, Via::Mdns | Via::Broadcast));
            }
            // "Found just now" only for a fresh discovery; an address from an
            // earlier connection is where it was last seen.
            let step = if found { ReachStep::TryingFound } else { ReachStep::TryingKnown };
            for a in &t.addrs {
                if let Ok(addr) = a.parse::<SocketAddr>()
                    && !candidates.contains(&addr)
                {
                    candidates.push(addr);
                }
            }
            if candidates.is_empty() {
                self.reach.looking(&t.id, now);
                continue;
            }
            candidates.truncate(6);
            self.dialing.insert(t.id.clone());
            self.reach.dialing(&t.id, now, step);
            self.spawn_dial(candidates, Some(t.fingerprint), Purpose::Reconnect(t.id.clone()));
        }
    }

    /// Between transfers, re-dial a linked peer over a strictly better path
    /// (e.g. a cable just plugged in). The duplicate-link rule then keeps the
    /// better link and closes the other.
    pub(super) fn upgrade_links(&mut self) {
        let now = Instant::now();
        let busy_links: Vec<_> = self.transfers.iter().filter(|t| t.active()).filter_map(|t| t.link).collect();
        let candidates: Vec<_> = self
            .store
            .peers()
            .iter()
            .filter(|t| t.trusted)
            .filter_map(|t| {
                let link = self.link_for(&t.id)?;
                if busy_links.contains(&link.id()) || self.dialing.contains(&t.id) {
                    return None;
                }
                let current = self.link_rank(&link).0;
                let peer = self.peers.get(&t.id)?;
                let best = self.best_path(peer)?;
                let best_kind = self.link_kind(&best.addr, None, best.if_index);
                super::paths::is_upgrade(current, best_kind)
                    .then(|| (t.id.clone(), t.fingerprint, best.addr, best_kind))
            })
            .collect();
        for (id, fp, addr, kind) in candidates {
            if self.upgrade_tried.get(&id).is_some_and(|t| now.duration_since(*t) < UPGRADE_EVERY) {
                continue;
            }
            self.upgrade_tried.insert(id.clone(), now);
            self.tech("link", format!("faster path {addr} ({kind:?}), connecting"));
            self.dialing.insert(id.clone());
            self.spawn_dial(vec![addr], Some(fp), Purpose::Reconnect(id));
        }
    }

    pub(super) fn reconnect_done(&mut self, id: &DeviceId, ok: bool) {
        self.dialing.remove(id);
        if ok {
            self.reach.connected(id);
        }
    }

    /// The other side told us whether it still trusts us.
    pub(super) fn on_welcome(&mut self, id: &DeviceId, name: &str, paired: bool) {
        let trusted_here = self.store.peer(id).is_some_and(|p| p.trusted);
        if !paired && let Some(p) = self.peers.get_mut(id) {
            p.accepts = None; // what it said about our files ended with the pairing
        }
        if trusted_here && !paired {
            let _ = self.store.update_peer(id, |p| p.trusted = false);
            self.say("trust", format!("{name} removed this computer. Pair again to send"), None, Some(Tone::Warn));
            self.forget_clips_from(id, name);
            self.forget_next_for(id);
            self.forget_shelf_from(id);
            self.dirty = true;
        }
    }

    pub(super) fn cmd_remove_peer(&mut self, id: &DeviceId) {
        let name = self.store.peer(id).map(|p| p.name.clone()).unwrap_or_default();
        self.drop_paused_with(id, None, "removed from paired devices");
        if let Ok(true) = self.store.remove_peer(id) {
            self.reach.forget(id);
            if let Some(p) = self.peers.get_mut(id) {
                p.accepts = None;
            }
            // Revoke at once: tell it, and treat its link as untrusted from now on.
            if let Some(link) = self.link_for(id) {
                link.try_send(Msg::Welcome { paired: false });
            }
            self.say("trust", format!("Removed {name}"), Some("no longer trusted".into()), None);
            self.forget_clips_from(id, &name);
            self.forget_next_for(id);
            self.forget_shelf_from(id);
            self.dirty = true;
        }
    }

    /// Disconnect (stay paired, stop connecting either way) or Connect again.
    pub(super) fn cmd_set_connected(&mut self, id: &DeviceId, connect: bool) {
        let Some(peer) = self.store.peer(id).filter(|p| p.trusted).cloned() else { return };
        if peer.disconnected != connect {
            return; // already so
        }
        if let Err(e) = self.store.set_disconnected(id, !connect) {
            self.say("error", "Couldn’t save the settings", Some(e.to_string()), Some(Tone::Err));
            return;
        }
        self.sync_turned_away();
        if connect {
            self.reach.start(id, Instant::now());
            self.restart_sweeps();
            if let Some(d) = &self.discovery {
                d.announce_now();
            }
            self.say("link", format!("Connecting to {} again", peer.name), None, None);
        } else {
            self.reach.forget(id);
            let active: Vec<_> = self.transfers.iter().filter(|t| t.active() && &t.peer == id).map(|t| t.tid).collect();
            for tid in active {
                self.cmd_cancel(tid);
            }
            let links: Vec<_> = self.links.values().filter(|l| &l.info().hello.id == id).cloned().collect();
            for link in links {
                link.try_send(Msg::Error {
                    code: ErrorCode::Disconnected,
                    msg: "disconnected on the other side".into(),
                });
                link.close();
            }
            self.say("link", format!("Disconnected from {}; still paired", peer.name), None, None);
        }
        self.dirty = true;
    }

    /// The keys the accept loop turns away: paired devices disconnected here.
    pub(super) fn sync_turned_away(&self) {
        let keys = self.store.peers().iter().filter(|p| p.trusted && p.disconnected).map(|p| p.fingerprint).collect();
        *self.turned_away.write().unwrap_or_else(|e| e.into_inner()) = keys;
    }

    /// The other side pressed Disconnect: say so, and stop knocking for a while.
    pub(super) fn on_remote_disconnected(&mut self, id: &DeviceId, name: &str) {
        self.reach.hold(id, Instant::now(), REMOTE_DISCONNECTED_WAIT);
        if self.facts.remote_disconnected.as_deref() != Some(name) {
            self.facts.remote_disconnected = Some(name.to_owned());
            self.say("link", format!("{name} disconnected from this computer; still paired"), None, Some(Tone::Warn));
            self.refresh_doctor();
        }
    }

    pub(super) fn cmd_rename_peer(&mut self, id: &DeviceId, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Ok(true) = self.store.rename_peer(id, &name.chars().take(63).collect::<String>()) {
            self.dirty = true;
        }
    }

    pub(super) fn cmd_set_incoming(&mut self, id: &DeviceId, value: Incoming) {
        if value == Incoming::Never {
            self.drop_paused_with(id, Some(crate::events::Direction::In), "files from it are now set to Never");
        }
        if let Ok(true) = self.store.set_incoming(id, value) {
            let name = self.store.peer(id).map(|p| p.name.clone()).unwrap_or_default();
            let word = match value {
                Incoming::Ask => "ask every time",
                Incoming::Always => "always accept",
                Incoming::Never => "never accept",
            };
            self.say("trust", format!("Files from {name}: {word}"), None, Some(Tone::Dim));
            if let Some(link) = self.link_for(id) {
                self.send_status(&link);
            }
            self.dirty = true;
        }
    }

    /// Tells a paired device our name and what we do with its files.
    pub(super) fn send_status(&self, link: &Link) {
        if !self.link_trusted(link) {
            return;
        }
        let accepts = self.store.peer(&link.info().hello.id).map(|p| p.incoming).unwrap_or_default();
        link.try_send(Msg::Status {
            role: if accepts == Incoming::Never { Role::Send } else { Role::Both },
            name: self.store.settings().device_name.clone(),
            accepts: Some(accepts),
        });
    }

    pub(super) fn cmd_settings(&mut self, patch: SettingsPatch) {
        let new_name =
            patch.device_name.map(|n| n.trim().chars().take(63).collect::<String>()).filter(|n| !n.is_empty());
        let result = self.store.update_settings(|s| {
            if let Some(n) = &new_name {
                s.device_name = n.clone();
            }
            if let Some(d) = &patch.receive_dir {
                s.receive_dir = d.clone();
            }
            if let Some(j) = patch.skip_junk {
                s.skip_junk = j;
            }
            if let Some(a) = patch.auto_update {
                s.auto_update = a;
            }
            if let Some(t) = patch.test_builds {
                s.test_builds = t;
            }
            if let Some(k) = &patch.clipboard_shortcut {
                s.clipboard_shortcut = Some(k.clone()).filter(|k| !k.is_empty());
            }
            if let Some(to) = &patch.send_to {
                s.send_to = to.clone();
            }
            if let Some(a) = patch.appearance {
                s.appearance = a;
            }
            if let Some(f) = patch.folder_per_device {
                s.folder_per_device = f;
            }
            if let Some(m) = patch.receive_mode {
                // Each choice has its own default folder until one is chosen.
                let was = s.receive_mode.unwrap_or_default();
                if patch.receive_dir.is_none() && m != was {
                    let (old_default, new_default) = (default_dir(was), default_dir(m));
                    if s.receive_dir == old_default {
                        s.receive_dir = new_default;
                    }
                }
                s.receive_mode = Some(m);
            }
            if let Some(k) = patch.keep_dates {
                s.keep_dates = k;
            }
            if patch.clear_receive_note {
                s.receive_note = None;
            }
        });
        if let Err(e) = result {
            self.say("error", "Couldn’t save the settings", Some(e.to_string()), Some(Tone::Err));
            return;
        }
        match patch.clipboard_sync {
            Some(true) => {
                let ids: Vec<DeviceId> =
                    self.store.peers().iter().filter(|p| p.trusted).map(|p| p.id.clone()).collect();
                for id in ids {
                    self.set_clip_sync(&id, true, crate::util::now_ms());
                }
            }
            Some(false) => self.clip_sync_off_all(),
            None => {}
        }
        if let Some(name) = new_name {
            self.hello.write().unwrap_or_else(|e| e.into_inner()).name = name.clone();
            if let Some(d) = &self.discovery {
                d.rename(&name);
            }
            for link in self.links.values() {
                self.send_status(link);
            }
            self.say("name", format!("This computer is now called {name}"), None, None);
        }
        self.dirty = true;
    }
}

/// The folder a way of receiving starts with: Downloads, or Downloads/SeamDrop.
fn default_dir(m: crate::store::ReceiveMode) -> std::path::PathBuf {
    match m {
        crate::store::ReceiveMode::AppFolder => crate::store::default_receive_dir(),
        _ => crate::store::downloads_dir(),
    }
}
