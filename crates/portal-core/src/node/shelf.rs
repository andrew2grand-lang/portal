//! The Shelf on the node (Mission 05 §3.2): consent per pair of devices,
//! records over the encrypted link without relaying, and the commands the
//! window uses. The data model and its rules are in `crate::shelf`.
//!
//! Also device pictures (§2.3): ours to every trusted link, theirs checked
//! before use.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::Node;
use crate::events::{NoticeKind, ShelfPeerView, ShelfView, Tone};
use crate::model::DeviceId;
use crate::net::link::Link;
use crate::proto::Msg;
use crate::shelf::{self, Refused, Shelf, ShelfFiles, ShelfRecord};

/// Undo is offered this long after a removal or Clear Shelf.
pub const UNDO_FOR: Duration = Duration::from_secs(8);

#[derive(Default)]
pub(super) struct ShelfState {
    pub shelf: Shelf,
    /// Each paired device's own answer (it agreed to share with us).
    pub theirs: HashMap<DeviceId, bool>,
    /// What the last removal or Clear took off, for Undo.
    pub undo: Option<(Instant, Vec<ShelfRecord>)>,
    /// A "full" notice was already shown for this device.
    pub full_noted: HashMap<DeviceId, Instant>,
    /// Pictures from paired devices (checked).
    pub pictures: HashMap<DeviceId, crate::picture::Picture>,
}

fn now_ms() -> u64 {
    super::next::now_ms()
}

/// The first line, up to 80 characters.
fn preview(text: &str) -> String {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    let mut s: String = line.chars().take(80).collect();
    if line.chars().count() > 80 {
        s.push('…');
    }
    s
}

impl Node {
    fn shelf_files(&self) -> ShelfFiles {
        ShelfFiles::in_dir(&self.cfg.data_dir)
    }

    /// At start: the Shelf from disk.
    pub(super) fn load_shelf(&mut self) {
        match self.shelf_files().load() {
            Ok(mut s) => {
                s.prune(now_ms());
                // Items from devices no longer paired leave.
                let keep: Vec<DeviceId> =
                    self.store.peers().iter().filter(|p| p.trusted).map(|p| p.id.clone()).collect();
                let me = self.identity.device_id.clone();
                s.records.retain(|_, r| r.origin == me || keep.contains(&r.origin));
                self.shelf.shelf = s;
            }
            Err(e) => self.say("shelf", "Couldn’t open the Shelf", Some(e.to_string()), Some(Tone::Err)),
        }
    }

    fn save_shelf(&mut self) {
        self.shelf.shelf.prune(now_ms());
        if let Err(e) = self.shelf_files().save(&self.shelf.shelf) {
            self.say("shelf", "Couldn’t save the Shelf", Some(e.to_string()), Some(Tone::Err));
        }
        self.dirty = true;
    }

    /// Shared with `id` right now: both agreed.
    pub(super) fn shelf_shared_with(&self, id: &DeviceId) -> bool {
        self.store.peer(id).is_some_and(|p| p.trusted && p.shelf_share) && self.shelf.theirs.get(id) == Some(&true)
    }

    pub(super) fn shelf_peer_view(&self, id: &DeviceId) -> Option<ShelfPeerView> {
        let p = self.store.peer(id).filter(|p| p.trusted)?;
        Some(ShelfPeerView {
            mine: p.shelf_share,
            theirs: self.shelf.theirs.get(id) == Some(&true),
            answered: p.shelf_answered,
        })
    }

    pub(super) fn shelf_views(&self) -> Vec<ShelfView> {
        let me = &self.identity.device_id;
        let mut v: Vec<&ShelfRecord> = self.shelf.shelf.live().collect();
        v.sort_by_key(|r| std::cmp::Reverse((r.added, r.id.clone())));
        v.into_iter()
            .map(|r| ShelfView {
                id: r.id.clone(),
                name: r.name.clone(),
                preview: (!r.private).then(|| preview(&r.text)),
                private: r.private,
                from_id: r.origin.clone(),
                from_name: if &r.origin == me {
                    self.store.settings().device_name.clone()
                } else {
                    self.store.peer(&r.origin).map(|p| p.name.clone()).unwrap_or_default()
                },
                mine: &r.origin == me,
                at_ms: r.added,
            })
            .collect()
    }

    // ---- consent -------------------------------------------------------------

    /// This computer agrees (or stops agreeing) to share the Shelf with `id`.
    pub(super) fn set_shelf_share(&mut self, id: &DeviceId, on: bool) {
        let Some(p) = self.store.peer(id).filter(|p| p.trusted).cloned() else { return };
        let at = now_ms().max(p.shelf_share_at + 1);
        if let Err(e) = self.store.set_shelf_share(id, on, at) {
            self.say("error", "Couldn’t save the settings", Some(e.to_string()), Some(Tone::Err));
            return;
        }
        let name = p.name.clone();
        if !on {
            // Its items leave here; ours leave there when it hears.
            if self.shelf.shelf.forget_origin(id) > 0 {
                self.save_shelf();
            }
            self.say("shelf", format!("The Shelf isn’t shared with {name} any more"), None, None);
        } else if self.shelf.theirs.get(id) == Some(&true) {
            self.say("shelf", format!("The Shelf is shared with {name}: both agreed"), None, None);
        } else {
            self.say("shelf", format!("Sharing the Shelf with {name} once {name} agrees too"), None, None);
        }
        if let Some(link) = self.link_for(id) {
            link.try_send(Msg::ShelfShare { on, at });
            if on && self.shelf.theirs.get(id) == Some(&true) {
                self.send_shelf_all(&link);
            }
        }
        self.dirty = true;
    }

    /// On a fresh trusted link: our answer, our picture.
    pub(super) fn shelf_on_link(&mut self, link: &Link) {
        let id = link.info().hello.id.clone();
        if let Some(p) = self.store.peer(&id).filter(|p| p.trusted) {
            link.try_send(Msg::ShelfShare { on: p.shelf_share, at: p.shelf_share_at });
        }
        link.try_send(Msg::Picture { picture: self.store.settings().picture.clone() });
    }

    /// `shelf_share` from a trusted device.
    pub(super) fn on_shelf_share(&mut self, link: &Link, on: bool) {
        let id = link.info().hello.id.clone();
        let Some(name) = self.store.peer(&id).filter(|p| p.trusted).map(|p| p.name.clone()) else { return };
        let was = self.shelf.theirs.insert(id.clone(), on) == Some(true);
        self.dirty = true;
        if !on {
            if was || self.shelf.shelf.records.values().any(|r| r.origin == id) {
                // Unshared there: its items leave here too.
                if self.shelf.shelf.forget_origin(&id) > 0 {
                    self.save_shelf();
                }
                if was {
                    self.say("shelf", format!("{name} stopped sharing the Shelf; its items left this one"), None, None);
                }
            }
            return;
        }
        if self.store.peer(&id).is_some_and(|p| p.shelf_share) {
            if !was {
                self.say("shelf", format!("The Shelf is shared with {name}"), None, None);
            }
            self.send_shelf_all(link);
        }
    }

    /// Everything `link`'s device should have from us: our own items and our
    /// edits of its items (never a third device's).
    fn send_shelf_all(&self, link: &Link) {
        let to = link.info().hello.id.clone();
        let items = self.shelf.shelf.for_peer(&self.identity.device_id, &to);
        link.try_send(Msg::Shelf { items });
    }

    /// One changed record: to its origin's sharing devices if it's ours, or
    /// back to its origin (which passes it on) if it's another device's.
    fn send_shelf_change(&self, r: &ShelfRecord) {
        let me = &self.identity.device_id;
        let to: Vec<DeviceId> = if &r.origin == me {
            self.store.peers().iter().filter(|p| self.shelf_shared_with(&p.id)).map(|p| p.id.clone()).collect()
        } else if self.shelf_shared_with(&r.origin) {
            vec![r.origin.clone()]
        } else {
            Vec::new()
        };
        for id in to {
            if let Some(link) = self.link_for(&id) {
                link.try_send(Msg::Shelf { items: vec![r.clone()] });
            }
        }
    }

    /// `shelf` from a trusted device: checked, merged, and our own items'
    /// changes passed on to the others that share with us.
    pub(super) fn on_shelf(&mut self, link: &Link, items: Vec<ShelfRecord>) {
        let from = link.info().hello.id.clone();
        let Some(name) = self.store.peer(&from).filter(|p| p.trusted).map(|p| p.name.clone()) else { return };
        if !self.shelf_shared_with(&from) {
            self.say(
                "shelf",
                format!("Ignored Shelf items from {name}: the Shelf isn’t shared with it"),
                None,
                Some(Tone::Dim),
            );
            return;
        }
        let me = self.identity.device_id.clone();
        let (mut changed, mut ours, mut refused, mut full) = (false, Vec::new(), 0u32, false);
        for r in items.into_iter().take(shelf::MAX_ITEMS + shelf::MAX_REMOVED) {
            let mine = r.origin == me;
            match self.shelf.shelf.accept(&me, &from, r.clone(), now_ms()) {
                Ok(true) => {
                    changed = true;
                    if mine {
                        ours.push(r);
                    }
                }
                Ok(false) => {}
                Err(Refused::Full) => full = true,
                Err(e) => {
                    refused += 1;
                    tracing::debug!("refused a Shelf record from {name}: {e:?}");
                }
            }
        }
        if refused > 0 {
            self.say(
                "shelf",
                format!("Refused {refused} Shelf item(s) from {name}"),
                Some("didn’t pass the checks".into()),
                Some(Tone::Warn),
            );
        }
        if full && self.shelf.full_noted.get(&from).is_none_or(|t| t.elapsed() > Duration::from_secs(3600)) {
            self.shelf.full_noted.insert(from.clone(), Instant::now());
            self.say("shelf", format!("The Shelf is full, so items from {name} weren’t kept"), None, Some(Tone::Warn));
            self.notice(
                NoticeKind::Clipboard,
                "Shelf full".into(),
                format!("Items from {name} weren’t kept. Remove some to make room."),
            );
        }
        if changed {
            self.save_shelf();
        }
        // An edit of our item from one device: the others that share with us hear it.
        for r in ours {
            self.send_shelf_change(&r);
        }
    }

    // ---- the window's commands ----------------------------------------------

    /// Add to Shelf (typed or pasted), or Put on Shelf.
    pub(super) fn shelf_add(&mut self, name: String, text: String, private: bool) -> Result<String, String> {
        let name = name.trim().chars().take(shelf::MAX_NAME).collect::<String>().replace(['\n', '\r'], " ");
        if text.trim().is_empty() {
            return Err("Nothing to keep: the text is empty.".into());
        }
        if text.len() > shelf::MAX_TEXT {
            return Err("Too long for the Shelf (64 KB at most). Send it as a file instead.".into());
        }
        if self.shelf.shelf.live_count() >= shelf::MAX_ITEMS {
            return Err(format!("The Shelf holds {} items. Remove one first.", shelf::MAX_ITEMS));
        }
        let me = self.identity.device_id.clone();
        let stamp = self.shelf.shelf.stamp(now_ms());
        let r = ShelfRecord {
            id: shelf::new_id(),
            origin: me.clone(),
            name,
            text,
            private,
            stamp,
            by: me,
            added: stamp,
            removed: false,
        };
        let id = r.id.clone();
        self.shelf.shelf.apply(r.clone());
        self.save_shelf();
        self.send_shelf_change(&r);
        self.say(
            "shelf",
            if private { "Kept a private item on the Shelf" } else { "Kept an item on the Shelf" },
            None,
            None,
        );
        Ok(id)
    }

    /// Renames (or edits) an item, wherever it was added.
    pub(super) fn shelf_edit(&mut self, id: &str, name: Option<String>, text: Option<String>) -> Result<(), String> {
        let Some(mut r) = self.shelf.shelf.records.get(id).filter(|r| !r.removed).cloned() else {
            return Err("That item isn’t on the Shelf any more.".into());
        };
        if let Some(n) = name {
            r.name = n.trim().chars().take(shelf::MAX_NAME).collect::<String>().replace(['\n', '\r'], " ");
        }
        if let Some(t) = text {
            if t.len() > shelf::MAX_TEXT || t.trim().is_empty() {
                return Err("The text must be between 1 byte and 64 KB.".into());
            }
            r.text = t;
        }
        r.stamp = self.shelf.shelf.stamp(now_ms());
        r.by = self.identity.device_id.clone();
        self.shelf.shelf.apply(r.clone());
        self.save_shelf();
        self.send_shelf_change(&r);
        Ok(())
    }

    /// ✕ (one id) or Clear Shelf (all): removal markers everywhere, and Undo.
    pub(super) fn shelf_remove(&mut self, ids: Option<Vec<String>>) {
        let targets: Vec<ShelfRecord> = match ids {
            Some(ids) => {
                ids.iter().filter_map(|id| self.shelf.shelf.records.get(id)).filter(|r| !r.removed).cloned().collect()
            }
            None => self.shelf.shelf.live().cloned().collect(),
        };
        if targets.is_empty() {
            return;
        }
        let me = self.identity.device_id.clone();
        let mut gone = Vec::new();
        for r in &targets {
            let marker = ShelfRecord {
                id: r.id.clone(),
                origin: r.origin.clone(),
                name: String::new(),
                text: String::new(),
                private: false,
                stamp: self.shelf.shelf.stamp(now_ms()),
                by: me.clone(),
                added: r.added,
                removed: true,
            };
            self.shelf.shelf.apply(marker.clone());
            gone.push(marker);
        }
        self.shelf.undo = Some((Instant::now(), targets.clone()));
        self.save_shelf();
        for m in &gone {
            self.send_shelf_change(m);
        }
        self.say(
            "shelf",
            format!("Removed {} from the Shelf", super::xfer::plural(gone.len() as u64, "item", "items")),
            None,
            None,
        );
    }

    /// Undo, a few seconds after a removal: the items come back as new
    /// changes (new ids, added here).
    pub(super) fn shelf_undo(&mut self) -> Result<(), String> {
        let Some((at, items)) = self.shelf.undo.take() else { return Err("Nothing to undo.".into()) };
        if at.elapsed() > UNDO_FOR {
            return Err("Too late to undo.".into());
        }
        for r in items {
            let _ = self.shelf_add(r.name, r.text, r.private);
        }
        Ok(())
    }

    /// The text of an item, for Copy and Reveal (never logged).
    pub(super) fn shelf_text(&self, id: &str) -> Option<(String, bool)> {
        self.shelf.shelf.records.get(id).filter(|r| !r.removed).map(|r| (r.text.clone(), r.private))
    }

    /// Copy: on the clipboard, with the private marks for a private item.
    pub(super) fn shelf_copy(&mut self, id: &str) -> Result<(), String> {
        let Some((text, private)) = self.shelf_text(id) else {
            return Err("That item isn’t on the Shelf any more.".into());
        };
        let c = crate::clip::Contents { text: Some(text), ..Default::default() };
        let written = if private { self.cfg.clipboard.write_private(&c) } else { self.cfg.clipboard.write(&c) };
        match written {
            Ok(count) => {
                // Our own write: automatic sync doesn't send it on.
                self.clip_ignore(count);
                Ok(())
            }
            Err(e) => Err(format!("Couldn’t put it on the clipboard: {e}")),
        }
    }

    /// A device unpaired: its items leave, and its picture.
    pub(super) fn forget_shelf_from(&mut self, id: &DeviceId) {
        self.shelf.theirs.remove(id);
        self.shelf.pictures.remove(id);
        if self.shelf.shelf.forget_origin(id) > 0 {
            self.save_shelf();
        }
    }

    // ---- pictures (§2.3) -------------------------------------------------------

    pub(super) fn on_picture(&mut self, link: &Link, picture: Option<crate::picture::Picture>) {
        let id = link.info().hello.id.clone();
        if !self.trusts(&id) {
            return;
        }
        match picture {
            Some(p) => match crate::picture::check(&p) {
                Ok(()) => {
                    self.shelf.pictures.insert(id, p);
                }
                Err(e) => {
                    self.shelf.pictures.remove(&id);
                    self.say(
                        "picture",
                        "A device picture didn’t pass the checks; showing the default",
                        Some(format!("{e:?}")),
                        Some(Tone::Dim),
                    );
                }
            },
            None => {
                self.shelf.pictures.remove(&id);
            }
        }
        self.dirty = true;
    }

    /// Ours changed: tell every trusted link.
    pub(super) fn send_picture_all(&self) {
        let picture = self.store.settings().picture.clone();
        for link in self.links.values() {
            if self.trusts(&link.info().hello.id) {
                link.try_send(Msg::Picture { picture: picture.clone() });
            }
        }
    }
}
