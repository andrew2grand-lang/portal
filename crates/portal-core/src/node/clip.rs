//! The shared clipboard on the node (Mission 03 §4, Mission 04 §3–4).
//!
//! - **Send clipboard** reads this computer's clipboard and sends it to the
//!   chosen paired devices (the remembered choice when none is given). Each
//!   puts it on its Shelf and, unless it set that device to "Shelf only",
//!   on its clipboard (marked as arrived, so it's never sent back). Too big
//!   for a clipboard: sent as a file.
//! - **Automatic sync** is agreed per pair of devices: it runs between two
//!   devices only when both said yes. Off on either side turns it off for
//!   that pair on both. Pausing is local: nothing is sent or taken from
//!   here for a while, and nobody's consent changes.
//! - Content an app marked private is never read, let alone sent. Only paired,
//!   trusted links carry it, over the same encrypted connection. The log says
//!   only the kind and size.
//! - The last 10 items received (the Shelf) are kept in memory only.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::{Internal, Node};
use crate::clip::{self, Assembly, ClipSent, Contents, Read, TooBig};
use crate::consts::APP_NAME;
use crate::events::{ClipPeerView, ClipView, NoticeKind, Tone};
use crate::model::DeviceId;
use crate::net::link::{Link, LinkId};
use crate::proto::Msg;
use crate::util::now_ms;

/// How often automatic sync looks at the clipboard.
pub(super) const WATCH_EVERY: Duration = Duration::from_millis(500);
/// A copy another app holds (Windows lets one app at a time open the
/// clipboard) is tried again this many times, a watch tick apart (about 4 s)
/// before automatic sync gives up on it, and says so (Mission 05 §1.9).
const BUSY_TRIES: u32 = 8;
/// Contents kept for Copy again, all items together; older ones let go first.
const KEEP_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Default)]
pub(super) struct ClipState {
    history: VecDeque<Entry>,
    incoming: HashMap<LinkId, Assembly>,
    /// The change count last looked at, and the one our own write made.
    last_seen: u64,
    ignore: Option<u64>,
    reading: bool,
    next_id: u64,
    /// A private copy was already noted (once per copy, not twice a second).
    private_noted: Option<u64>,
    /// The change the watcher found the clipboard busy for, and how often.
    busy: (u64, u32),
    /// The latest copy that wasn't synced, said once in the window (§3.1).
    pub(super) notice: Option<crate::events::ClipNotice>,
    notices: u64,
    /// What each paired device said about syncing with this one.
    pub(super) theirs: HashMap<DeviceId, Theirs>,
    /// Paused here (local only: consent stays as it is).
    pub(super) pause: Option<Pause>,
}

/// One paired device's side of automatic sync with this computer.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Theirs {
    /// It agreed to sync with this computer.
    pub on: bool,
    /// When it last changed that (its clock).
    pub at: u64,
    /// It asks this computer to agree, unanswered.
    pub asks: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Pause {
    /// Until this time (and unix ms, for the UI).
    Until(Instant, u64),
    /// Until SeamDrop restarts.
    Restart,
}

#[derive(Debug)]
struct Entry {
    id: u64,
    from: String,
    from_id: Option<DeviceId>,
    at_ms: u64,
    contents: Option<Contents>,
    kind: &'static str,
    summary: String,
    preview: String,
    /// Put on the clipboard here too (not "Shelf only").
    pasted: bool,
    /// Sent as private on purpose: no preview, marked private when copied.
    private: bool,
    /// An image's small preview.
    thumb: Option<String>,
}

/// Who asked for a read.
pub(super) enum Reader {
    /// Send clipboard, to these devices (None: the remembered choice). The
    /// reply is a sentence either way.
    Button(Option<Vec<DeviceId>>, tokio::sync::oneshot::Sender<Result<ClipSent, String>>),
    /// Automatic sync saw a change (its count).
    Watch(u64),
}

/// Paired devices a send goes to, and the chosen ones that aren't connected.
struct Targets {
    to: Vec<(DeviceId, String, Link)>,
    away: Vec<String>,
}

pub(super) use crate::util::and_list as names;

impl Node {
    pub(super) fn clip_views(&self) -> Vec<ClipView> {
        self.clip
            .history
            .iter()
            .map(|e| ClipView {
                id: e.id,
                from: e.from.clone(),
                from_id: e.from_id.clone(),
                at_ms: e.at_ms,
                kind: e.kind.into(),
                summary: e.summary.clone(),
                preview: e.preview.clone(),
                copyable: e.contents.is_some(),
                pasted: e.pasted,
                private: e.private,
                thumb: e.thumb.clone(),
            })
            .collect()
    }

    /// The remembered choice: the devices ticked last time that are still
    /// paired, or every paired device until a choice is made.
    pub(super) fn send_to(&self) -> Vec<DeviceId> {
        let paired: Vec<DeviceId> =
            self.store.peers().iter().filter(|p| p.trusted && !p.disconnected).map(|p| p.id.clone()).collect();
        let chosen: Vec<DeviceId> =
            self.store.settings().send_to.iter().filter(|id| paired.contains(id)).cloned().collect();
        if chosen.is_empty() { paired } else { chosen }
    }

    /// Where a send goes: the devices asked for (or the remembered choice),
    /// connected ones only.
    fn clip_targets(&self, wanted: Option<Vec<DeviceId>>) -> Result<Targets, String> {
        let wanted = wanted.filter(|w| !w.is_empty()).unwrap_or_else(|| self.send_to());
        if wanted.is_empty() {
            return Err("Pair with your other computer first.".into());
        }
        let mut targets = Targets { to: Vec::new(), away: Vec::new() };
        for id in wanted {
            let Some(p) = self.store.peer(&id).filter(|p| p.trusted) else { continue };
            match self.link_for(&id) {
                Some(link) => targets.to.push((id, p.name.clone(), link)),
                None => targets.away.push(p.name.clone()),
            }
        }
        if targets.to.is_empty() {
            return Err(match targets.away.as_slice() {
                [] => "Pair with your other computer first.".into(),
                [one] => format!("{one} isn’t connected right now."),
                many => format!("None of them is connected right now: {}.", names(many)),
            });
        }
        Ok(targets)
    }

    /// Send clipboard (the window, the tray, the shortcut).
    pub(super) fn cmd_send_clipboard(
        &mut self,
        to: Option<Vec<DeviceId>>,
        reply: tokio::sync::oneshot::Sender<Result<ClipSent, String>>,
    ) {
        if let Err(e) = self.clip_targets(to.clone()) {
            let _ = reply.send(Err(e));
            return;
        }
        self.read_clipboard(Reader::Button(to, reply));
    }

    fn read_clipboard(&mut self, who: Reader) {
        let (io, tx) = (self.cfg.clipboard.clone(), self.internal_tx.clone());
        tokio::task::spawn_blocking(move || {
            let read = io.read();
            let _ = tx.blocking_send(Internal::ClipRead(who, read));
        });
    }

    pub(super) fn on_clip_read(&mut self, who: Reader, read: Read) {
        let (auto, wanted) = match &who {
            Reader::Watch(_) => (true, None),
            Reader::Button(to, _) => (false, to.clone()),
        };
        if auto {
            self.clip.reading = false;
        }
        if let Reader::Watch(count) = who {
            if read == Read::Busy {
                let tries = if self.clip.busy.0 == count { self.clip.busy.1 + 1 } else { 1 };
                self.clip.busy = (count, tries);
                if tries < BUSY_TRIES {
                    return; // still unseen: read again on the next tick
                }
                self.clip.last_seen = count;
                self.say(
                    "clipboard",
                    "Couldn’t read that copy: another app kept the clipboard busy. Try Send clipboard",
                    Some(format!("busy for {tries} tries")),
                    Some(Tone::Warn),
                );
                self.notice(
                    NoticeKind::Clipboard,
                    "Not synced".into(),
                    "Couldn’t read that copy. Try Send clipboard.".into(),
                );
                return;
            }
            self.clip.last_seen = count;
        }
        let answer = |who: Reader, r: Result<String, String>| {
            if let Reader::Button(_, reply) = who {
                let _ = reply.send(r.map(|said| ClipSent { said, files: Vec::new() }));
            }
        };
        let contents = match read {
            Read::Private { marker, source } => {
                let count = if let Reader::Watch(c) = who { Some(c) } else { None };
                if !auto || self.clip.private_noted != count {
                    self.clip.private_noted = count;
                    let tone = if auto { Some(Tone::Dim) } else { None };
                    let detail = match &source {
                        Some(s) => format!("{marker}, from {s}"),
                        None => marker.clone(),
                    };
                    self.say("clipboard", clip::private_words(source.as_deref()), Some(detail), tone);
                    if let Some(c) = count {
                        self.not_synced(
                            crate::events::ClipNoticeKind::Private,
                            c,
                            "Not synced",
                            "The app you copied from marked it private. Put it on the Shelf to keep it.",
                        );
                    }
                }
                return answer(who, Err(clip::PRIVATE_SAID.into()));
            }
            // Offered as a transfer; automatic sync leaves files alone
            Read::Files(files) => {
                if let Reader::Button(_, reply) = who {
                    let said = match files.len() {
                        0 => "Files are on the clipboard, but they couldn’t be read. Drop them on the window instead."
                            .into(),
                        1 => "1 file on the clipboard".into(),
                        n => format!("{n} files on the clipboard"),
                    };
                    let _ = reply.send(if files.is_empty() { Err(said) } else { Ok(ClipSent { said, files }) });
                }
                return;
            }
            Read::Busy => {
                return answer(who, Err("Another app is using the clipboard right now. Try again in a moment.".into()));
            }
            Read::Empty => return answer(who, Err("The clipboard is empty. Copy something first.".into())),
            // Automatic sync never sends back what just arrived.
            Read::Contents { ours: true, .. } if auto => return,
            Read::Contents { contents, .. } if contents.is_empty() => {
                return answer(who, Err("The clipboard is empty. Copy something first.".into()));
            }
            Read::Contents { contents, .. } => contents,
        };
        let targets = if auto {
            let to: Vec<_> = self
                .store
                .peers()
                .iter()
                .filter(|p| self.clip_sync_with(&p.id))
                .filter_map(|p| self.link_for(&p.id).map(|l| (p.id.clone(), p.name.clone(), l)))
                .collect();
            if to.is_empty() {
                return;
            }
            Targets { to, away: Vec::new() }
        } else {
            match self.clip_targets(wanted) {
                Ok(t) => t,
                Err(e) => return answer(who, Err(e)),
            }
        };
        if auto && clip::check(&contents).is_err() {
            self.say(
                "clipboard",
                "A copy was too big to sync by itself; Send clipboard sends it as a file",
                Some(clip::summary(&contents)),
                Some(Tone::Dim),
            );
            if let Reader::Watch(c) = who {
                self.not_synced(
                    crate::events::ClipNoticeKind::TooBig,
                    c,
                    "Not synced",
                    "Too big for the clipboard. Send it as a file.",
                );
            }
            return;
        }
        let said = self.send_contents(&contents, &targets, auto, false, "Clipboard");
        answer(who, said)
    }

    /// The Shelf's text box (Mission 04 §4.1): typed or pasted text, sent like
    /// a copy to these devices (None: the remembered choice). Never read from
    /// the clipboard, so a private mark there doesn't apply.
    pub(super) fn cmd_send_text(
        &mut self,
        to: Option<Vec<DeviceId>>,
        text: String,
        private: bool,
    ) -> Result<String, String> {
        if text.trim().is_empty() {
            return Err("Type something to send first.".into());
        }
        let targets = self.clip_targets(to)?;
        let contents = Contents { text: Some(text), ..Default::default() };
        self.send_contents(&contents, &targets, false, private, if private { "Private text" } else { "Text" })
    }

    /// Sends contents to the targets (as a file when over the clipboard
    /// limits); the sentence to show. `what` starts the Details line.
    fn send_contents(
        &mut self,
        contents: &Contents,
        targets: &Targets,
        auto: bool,
        private: bool,
        what: &str,
    ) -> Result<String, String> {
        if private && clip::check(contents).is_err() {
            // A file would arrive unmarked: never for private text
            return Err("That’s too long to send as private text. Send it as a file instead.".into());
        }
        if let Err(big) = clip::check(contents) {
            return self.send_clip_as_file(targets, contents, big);
        }
        let summary = if private { format!("private {}", clip::summary(contents)) } else { clip::summary(contents) };
        for (_, _, link) in &targets.to {
            self.clip.next_id += 1;
            let mut msgs = clip::parts(self.clip.next_id, contents, auto);
            for m in &mut msgs {
                if let Msg::Clip { private: p, .. } = m {
                    *p = private;
                }
            }
            let link = link.clone();
            tokio::spawn(async move {
                for m in msgs {
                    if link.send(m).await.is_err() {
                        return;
                    }
                }
            });
        }
        let to_names: Vec<String> = targets.to.iter().map(|t| t.1.clone()).collect();
        let how = if auto { "synced to" } else { "sent to" };
        self.say("clipboard", format!("{what} {how} {}", names(&to_names)), Some(summary.clone()), None);
        let paste = match targets.to.as_slice() {
            [(id, _, _)] => match self.store.peer(id).map(|p| p.os) {
                Some(crate::model::Os::Windows) => "Paste it there with Ctrl+V.",
                Some(crate::model::Os::Macos) => "Paste it there with ⌘V.",
                _ => "Paste it there as usual.",
            },
            _ => "Paste it there as usual.",
        };
        let away = match targets.away.as_slice() {
            [] => String::new(),
            [one] => format!(" {one} isn’t connected, so it didn’t go there."),
            many => format!(" {} aren’t connected, so it didn’t go there.", names(many)),
        };
        let kept = if private { " It’s marked private there, so clipboard history leaves it." } else { "" };
        Ok(format!("Sent to {} ({summary}). {paste}{kept}{away}", names(&to_names)))
    }

    /// Over the clipboard limits: the same contents as a file transfer.
    fn send_clip_as_file(&mut self, targets: &Targets, c: &Contents, big: TooBig) -> Result<String, String> {
        let (file, bytes, what, size) = match big {
            TooBig::Image(n) => ("Clipboard image.png", c.png.clone().unwrap_or_default(), "image", n),
            TooBig::Text(n) => ("Clipboard text.txt", c.text.clone().unwrap_or_default().into_bytes(), "text", n),
        };
        let dir: PathBuf = self.cfg.data_dir.join("clipboard-out");
        let path = dir.join(file);
        std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&path, &bytes))
            .map_err(|e| format!("Couldn’t prepare the file to send: {e}"))?;
        let peers: Vec<DeviceId> = targets.to.iter().map(|t| t.0.clone()).collect();
        let results = self.cmd_send_many(peers, vec![path]);
        let started: Vec<String> = results
            .iter()
            .filter(|(_, r)| r.is_ok())
            .filter_map(|(id, _)| self.store.peer(id).map(|p| p.name.clone()))
            .collect();
        if started.is_empty() {
            return Err(results.into_iter().find_map(|(_, r)| r.err()).unwrap_or_default());
        }
        self.say(
            "clipboard",
            format!("The {what} was too big for a clipboard, so it’s going as a file"),
            Some(clip::size(size)),
            None,
        );
        Ok(format!(
            "The {what} is {}, too big for a clipboard, so it’s being sent to {} as a file instead ({file}).",
            clip::size(size),
            names(&started)
        ))
    }

    /// A clipboard part from a trusted link.
    pub(super) fn on_clip_part(&mut self, link: &Link, msg: Msg) {
        let Msg::Clip { id, n, of, auto, data, private } = msg else { return };
        let lid = link.id();
        if n == 0 {
            match Assembly::new(id, of, auto) {
                Ok(a) => {
                    self.clip.incoming.insert(lid, a);
                }
                Err(e) => {
                    return self.say(
                        "clipboard",
                        "Refused an incoming clipboard",
                        Some(e.to_string()),
                        Some(Tone::Warn),
                    );
                }
            }
        }
        let Some(a) = self.clip.incoming.get_mut(&lid) else { return };
        match a.push(id, n, &data) {
            Ok(None) => {}
            Ok(Some(contents)) => {
                self.clip.incoming.remove(&lid);
                let peer = link.info().hello.id.clone();
                let from =
                    self.store.peer(&peer).map(|p| p.name.clone()).unwrap_or_else(|| link.info().hello.name.clone());
                if auto && !self.clip_sync_with(&peer) {
                    let why =
                        if self.clip.pause.is_some() { "sync is paused here" } else { "sync with it is off here" };
                    self.say(
                        "clipboard",
                        format!("Didn’t take an automatic copy from {from}: {why}"),
                        Some(clip::summary(&contents)),
                        Some(Tone::Dim),
                    );
                    return;
                }
                // Incoming always goes to the clipboard (Mission 05 §3.1: the
                // old "Shelf only" choice is gone; the Shelf is only what you
                // put there).
                let (io, tx) = (self.cfg.clipboard.clone(), self.internal_tx.clone());
                tokio::task::spawn_blocking(move || {
                    let written = if private { io.write_private(&contents) } else { io.write(&contents) };
                    let written = written.map_err(|e| e.to_string());
                    let _ = tx.blocking_send(Internal::ClipWritten {
                        from,
                        from_id: peer,
                        contents,
                        auto,
                        private,
                        written,
                    });
                });
            }
            Err(e) => {
                self.clip.incoming.remove(&lid);
                self.say("clipboard", "Refused an incoming clipboard", Some(e.to_string()), Some(Tone::Warn));
            }
        }
    }

    pub(super) fn on_clip_written(
        &mut self,
        from: String,
        from_id: DeviceId,
        contents: Contents,
        auto: bool,
        private: bool,
        written: Result<u64, String>,
    ) {
        let count = match written {
            Ok(c) => c,
            Err(e) => {
                self.say(
                    "clipboard",
                    format!("Couldn’t put what {from} sent on the clipboard. Send it again"),
                    Some(e.to_string()),
                    Some(Tone::Err),
                );
                return;
            }
        };
        // Our own write: automatic sync must not send it back.
        self.clip.ignore = Some(count);
        self.clip.last_seen = count;
        self.shelve(from, Some(from_id), contents, auto, true, private);
    }

    /// Puts an arrived item on the Shelf (and says so). Private text shows no
    /// preview, here or in a notification.
    fn shelve(
        &mut self,
        from: String,
        from_id: Option<DeviceId>,
        contents: Contents,
        auto: bool,
        pasted: bool,
        private: bool,
    ) {
        let (summary, preview) = if private {
            (format!("private {}", clip::summary(&contents)), "Private text".to_owned())
        } else {
            (clip::summary(&contents), clip::preview(&contents))
        };
        let where_ = if pasted { "ready to paste" } else { "on the Shelf" };
        let what = if private { "Private text" } else { "Clipboard" };
        self.say("clipboard", format!("{what} from {from}, {where_}"), Some(summary.clone()), None);
        if !auto {
            let body =
                if pasted { format!("{preview} — ready to paste.") } else { format!("{preview} — on the Shelf.") };
            self.notice(NoticeKind::Clipboard, format!("Clipboard from {from}"), body);
        }
        self.clip.next_id += 1;
        let thumb = if private { None } else { contents.png.as_deref().and_then(clip::thumbnail) };
        let entry = Entry {
            id: self.clip.next_id,
            from,
            from_id,
            at_ms: now_ms(),
            kind: clip::kind(&contents),
            summary,
            preview,
            contents: Some(contents),
            pasted,
            private,
            thumb,
        };
        self.clip.history.push_front(entry);
        self.clip.history.truncate(clip::HISTORY);
        // Big items are let go (oldest first) beyond the memory budget.
        let mut kept = 0usize;
        for e in self.clip.history.iter_mut() {
            let size = e.contents.as_ref().map_or(0, |c| clip::text_bytes(c) + c.png.as_ref().map_or(0, Vec::len));
            kept += size;
            if kept > KEEP_BYTES {
                e.contents = None;
            }
        }
        self.dirty = true;
    }

    /// Put on Shelf from a private-copy notice (Mission 05 §3.1, amending
    /// decision 146): the marked copy is read only now, on the click, and
    /// only if the clipboard still holds the copy the notice was about.
    pub(super) fn shelf_put_marked(&mut self, count: u64) -> Result<(), String> {
        const GONE: &str = "That copy is gone; copy it again.";
        if self.cfg.clipboard.change_count() != count {
            return Err(GONE.into());
        }
        let Some((text, at)) = self.cfg.clipboard.read_marked_text() else { return Err(GONE.into()) };
        if at != count || text.trim().is_empty() {
            return Err(GONE.into());
        }
        self.shelf_add(String::new(), text, true)?;
        self.clip.notice = None;
        self.dirty = true;
        Ok(())
    }

    /// Put on Shelf from Recently received (text only).
    pub(super) fn shelf_put_received(&mut self, id: u64) -> Result<(), String> {
        let Some((text, private)) = self
            .clip
            .history
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.contents.as_ref().and_then(|c| c.text.clone()).map(|t| (t, e.private)))
        else {
            return Err("Only text goes on the Shelf, and that item isn’t kept any more.".into());
        };
        self.shelf_add(String::new(), text, private).map(|_| ())
    }

    /// Save as file, for an image in Recently received: into the received
    /// files folder, so nobody digs through screenshot folders.
    pub(super) fn save_clip_image(&mut self, id: u64, name: &str) -> Result<String, String> {
        let Some(png) = self.clip.history.iter().find(|e| e.id == id).and_then(|e| e.contents.as_ref()?.png.clone())
        else {
            return Err("That image isn’t kept any more.".into());
        };
        let dir = self.store.settings().receive_dir.clone();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn’t open {}: {e}", dir.display()))?;
        // The window names it in local time ("Clipboard image 2026-11-21
        // 17.20"); made safe here, a .png, never a path.
        let target = crate::fsafe::Target::current();
        let base = name.rsplit(['/', '\\']).next().unwrap_or("").trim().trim_end_matches(".png");
        let base = if base.is_empty() { "Clipboard image" } else { base };
        let file = crate::fsafe::sanitize(&format!("{base}.png"), target).name;
        let mut path = dir.join(&file);
        let mut n = 2;
        while path.exists() {
            path = dir.join(crate::fsafe::numbered_fitting(&file, n, false, target));
            n += 1;
        }
        std::fs::write(&path, png).map_err(|e| format!("Couldn’t save it: {e}"))?;
        self.say("clipboard", "Saved a clipboard image as a file", Some(path.display().to_string()), None);
        Ok(path.display().to_string())
    }

    /// Copy again (from the history).
    pub(super) fn cmd_copy_clip(&mut self, id: u64, reply: tokio::sync::oneshot::Sender<Result<(), String>>) {
        let Some((contents, private)) =
            self.clip.history.iter().find(|e| e.id == id).and_then(|e| e.contents.clone().map(|c| (c, e.private)))
        else {
            let _ = reply.send(Err("That item isn’t kept any more.".into()));
            return;
        };
        self.copy_own(contents, private, reply);
    }

    /// Writes the clipboard on this computer's own behalf (Copy on the Shelf,
    /// Copy technical log): automatic sync must not send it on.
    pub(super) fn copy_own(
        &mut self,
        contents: Contents,
        private: bool,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    ) {
        let io = self.cfg.clipboard.clone();
        let tx = self.internal_tx.clone();
        tokio::task::spawn_blocking(move || {
            let written = if private { io.write_private(&contents) } else { io.write(&contents) };
            let written = written.map_err(|e| e.to_string());
            let _ = tx.blocking_send(Internal::ClipCopied(written.as_ref().ok().copied()));
            let _ = reply.send(written.map(|_| ()));
        });
    }

    pub(super) fn on_clip_copied(&mut self, count: Option<u64>) {
        if let Some(c) = count {
            self.clip.ignore = Some(c);
            self.clip.last_seen = c;
        }
    }

    pub(super) fn cmd_remove_clip(&mut self, id: u64) {
        let before = self.clip.history.len();
        self.clip.history.retain(|e| e.id != id);
        if self.clip.history.len() < before {
            self.say("clipboard", "Took an item off the Shelf", None, Some(Tone::Dim));
            self.dirty = true;
        }
    }

    /// No longer paired: what the device sent leaves the Shelf too.
    pub(super) fn forget_clips_from(&mut self, id: &DeviceId, name: &str) {
        let before = self.clip.history.len();
        self.clip.history.retain(|e| e.from_id.as_ref() != Some(id));
        let gone = before - self.clip.history.len();
        if gone > 0 {
            let items = if gone == 1 { "1 item".to_owned() } else { format!("{gone} items") };
            self.say("clipboard", format!("Cleared {items} from {name} off the Shelf"), None, Some(Tone::Dim));
            self.dirty = true;
        }
    }

    pub(super) fn cmd_clear_clips(&mut self) {
        self.clip.history.clear();
        self.say("clipboard", "Cleared the Shelf", None, Some(Tone::Dim));
        self.dirty = true;
    }

    // ---- automatic sync ------------------------------------------------------

    /// This computer agreed to sync with `id`.
    fn clip_mine(&self, id: &DeviceId) -> bool {
        self.store.peer(id).is_some_and(|p| p.trusted && p.clip_sync)
    }

    /// Copies go between this computer and `id` by themselves: both agreed,
    /// and it isn't paused here.
    pub(super) fn clip_sync_with(&self, id: &DeviceId) -> bool {
        self.clip_mine(id) && self.clip.theirs.get(id).is_some_and(|t| t.on) && !self.clip_paused()
    }

    fn clip_paused(&self) -> bool {
        match self.clip.pause {
            Some(Pause::Until(until, _)) => Instant::now() < until,
            Some(Pause::Restart) => true,
            None => false,
        }
    }

    /// Watching while sync runs with someone who's connected.
    /// A copy that wasn't synced: said once, in the window (and as a
    /// notification when it isn't in front), with what to do about it.
    fn not_synced(&mut self, kind: crate::events::ClipNoticeKind, count: u64, title: &str, body: &str) {
        self.clip.notices += 1;
        let id = self.clip.notices;
        self.clip.notice = Some(crate::events::ClipNotice { id, kind, count, at_ms: now_ms() });
        self.notice(NoticeKind::Clipboard, title.into(), body.into());
        self.dirty = true;
    }

    /// A clipboard write of our own (Copy from the Shelf): automatic sync
    /// doesn't send it on.
    pub(super) fn clip_ignore(&mut self, count: u64) {
        self.clip.ignore = Some(count);
        self.clip.last_seen = count;
    }

    pub(super) fn clip_watching(&self) -> bool {
        self.store.peers().iter().any(|p| self.clip_sync_with(&p.id) && self.link_for(&p.id).is_some())
    }

    /// Every second: a pause that's over ends.
    pub(super) fn clip_tick(&mut self) {
        if let Some(Pause::Until(until, _)) = self.clip.pause
            && Instant::now() >= until
        {
            self.clip.pause = None;
            self.clip.last_seen = self.cfg.clipboard.change_count();
            self.say("clipboard", "Clipboard sync is running again after the pause", None, None);
            self.dirty = true;
        }
    }

    pub(super) fn clip_watch(&mut self) {
        if self.clip.reading {
            return;
        }
        let count = self.cfg.clipboard.change_count();
        if count == self.clip.last_seen {
            return;
        }
        if self.clip.ignore == Some(count) {
            self.clip.last_seen = count;
            return;
        }
        // Not seen until it's read (or given up on): a busy clipboard is
        // tried again on the next ticks (Mission 05 §1.9).
        self.clip.reading = true;
        self.read_clipboard(Reader::Watch(count));
    }

    /// This computer agrees (or stops agreeing) to sync with `id`, and tells it.
    pub(super) fn set_clip_sync(&mut self, id: &DeviceId, on: bool, now: u64) {
        let Some(p) = self.store.peer(id).filter(|p| p.trusted).cloned() else { return };
        // Newer than any change seen from either computer, whatever their
        // clocks say, so the other side takes it as the latest.
        let theirs_at = self.clip.theirs.get(id).map_or(0, |t| t.at);
        let at = now.max(p.clip_sync_at + 1).max(theirs_at + 1);
        if let Err(e) = self.store.set_clip_sync(id, on, at) {
            self.say("error", "Couldn’t save the settings", Some(e.to_string()), Some(Tone::Err));
            return;
        }
        // Answered: on is a yes, off is a no.
        if let Some(t) = self.clip.theirs.get_mut(id) {
            t.asks = false;
        }
        if on {
            // From now on: what's already on the clipboard isn't sent.
            self.clip.last_seen = self.cfg.clipboard.change_count();
        }
        let theirs = self.clip.theirs.get(id).is_some_and(|t| t.on);
        let state = match (on, theirs) {
            (true, true) => format!("Clipboard sync with {} is on: both agreed", p.name),
            (true, false) => format!("Clipboard sync with {0} is on here; waiting for {0} to agree", p.name),
            (false, _) => format!("Clipboard sync with {} is off", p.name),
        };
        self.say("clipboard", state, None, None);
        if let Some(link) = self.link_for(id) {
            link.try_send(Msg::ClipSync { on, at });
        }
        self.dirty = true;
    }

    /// "Turn off clipboard sync": off with every device, which also answers
    /// "no" to any that asked (or are on, waiting for this one).
    pub(super) fn clip_sync_off_all(&mut self) {
        let ids: Vec<DeviceId> = self
            .store
            .peers()
            .iter()
            .filter(|p| p.trusted)
            .filter(|p| p.clip_sync || self.clip.theirs.get(&p.id).is_some_and(|t| t.on || t.asks))
            .map(|p| p.id.clone())
            .collect();
        for id in ids {
            self.set_clip_sync(&id, false, now_ms());
        }
        for t in self.clip.theirs.values_mut() {
            t.asks = false;
        }
        self.dirty = true;
    }

    /// Pause here: `None` until SeamDrop restarts.
    pub(super) fn pause_clip_sync(&mut self, wait: Option<Duration>) {
        self.clip.pause = Some(match wait {
            Some(d) => Pause::Until(Instant::now() + d, now_ms() + d.as_millis() as u64),
            None => Pause::Restart,
        });
        let until = match wait {
            Some(d) => format!("for {} minutes", d.as_secs().div_ceil(60)),
            None => format!("until {APP_NAME} restarts"),
        };
        self.say(
            "clipboard",
            format!("Clipboard sync paused {until}"),
            Some("only on this computer; nobody’s choice changes".into()),
            None,
        );
        self.dirty = true;
    }

    pub(super) fn resume_clip_sync(&mut self) {
        if self.clip.pause.take().is_some() {
            self.clip.last_seen = self.cfg.clipboard.change_count();
            self.say("clipboard", "Clipboard sync is running again", None, None);
            self.dirty = true;
        }
    }

    /// `clip_sync` from a trusted link: that device's side of the pair. Off
    /// turns it off here too for this pair (when it's newer than our own
    /// change); on never turns it on here: it asks.
    pub(super) fn on_clip_sync(&mut self, link: &Link, on: bool, at: u64) {
        let id = link.info().hello.id.clone();
        let Some(p) = self.store.peer(&id).cloned() else { return };
        let name = p.name.clone();
        let t = self.clip.theirs.entry(id.clone()).or_default();
        let was = t.on;
        t.on = on;
        t.at = t.at.max(at);
        self.dirty = true;
        if !on {
            t.asks = false;
            if p.clip_sync && at > p.clip_sync_at {
                if let Err(e) = self.store.set_clip_sync(&id, false, at) {
                    self.say("error", "Couldn’t save the settings", Some(e.to_string()), Some(Tone::Err));
                }
                self.say("clipboard", format!("Clipboard sync with {name} is off: turned off on {name}"), None, None);
                self.notice(
                    NoticeKind::ClipboardSync,
                    "Clipboard sync is off".into(),
                    format!("Turned off on {name}. Copies stay on each computer."),
                );
            } else if was {
                self.say("clipboard", format!("Clipboard sync is off on {name}"), None, Some(Tone::Dim));
            }
            return;
        }
        if p.clip_sync {
            if !was {
                // Both have said yes now: start from what's copied next.
                self.clip.last_seen = self.cfg.clipboard.change_count();
                self.say("clipboard", format!("Clipboard sync with {name} is on: both agreed"), None, None);
            }
            return;
        }
        if !t.asks {
            t.asks = true;
            self.say(
                "clipboard",
                format!("{name} asks to sync the clipboard; it stays off unless you agree"),
                None,
                None,
            );
            self.notice(
                NoticeKind::ClipboardSync,
                format!("{name} asks to sync the clipboard"),
                format!("Copies would go between {name} and this computer by themselves. It stays off unless you turn it on."),
            );
        }
    }

    /// A trusted link came up: say where sync with it stands here.
    pub(super) fn send_clip_sync(&self, link: &Link) {
        let id = &link.info().hello.id;
        let (on, at) = self.store.peer(id).map_or((false, 0), |p| (p.clip_sync, p.clip_sync_at));
        link.try_send(Msg::ClipSync { on, at });
    }

    /// A paired device's clipboard state, for its row in the window.
    pub(super) fn clip_peer_view(&self, id: &DeviceId) -> Option<ClipPeerView> {
        let p = self.store.peer(id).filter(|p| p.trusted)?;
        let t = self.clip.theirs.get(id).copied().unwrap_or_default();
        Some(ClipPeerView { mine: p.clip_sync, theirs: t.on, asks: t.asks && !p.clip_sync, to: p.clip_to })
    }

    /// When the pause ends (unix ms), or 0 for "until SeamDrop restarts".
    pub(super) fn clip_pause_view(&self) -> Option<u64> {
        match self.clip.pause {
            Some(Pause::Until(until, ms)) if Instant::now() < until => Some(ms),
            Some(Pause::Restart) => Some(0),
            _ => None,
        }
    }

    pub(super) fn forget_clip_link(&mut self, id: LinkId) {
        self.clip.incoming.remove(&id);
    }
}
