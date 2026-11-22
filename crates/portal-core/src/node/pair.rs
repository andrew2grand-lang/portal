//! Pairing by typed code on the node (Mission 04 §2; the scheme is in
//! `crate::pairing`, the wire flow in `docs/PROTOCOL.md`).
//!
//! The asker is the computer where Pair was pressed: its person types the
//! code. The shower is asked first ("… wants to pair · Show code / Ignore")
//! and then shows a fresh code. Per try:
//!
//! ```text
//! asker  → pair_ask                      (the shower's person: Show code / Ignore)
//! shower → pair_ready                    (Ignore sends nothing at all)
//! asker  → pair_key  {its SPAKE2 share}  (after the code is typed)
//! shower → pair_key  {its share}
//! asker  → pair_check {its confirmation}
//! shower → pair_wrong {tries left}       (wrong code: type it again)
//!        | pair_check {its confirmation} (right: the asker checks it)
//! asker  → pair_done                     (both trust each other)
//! ```
//!
//! SPAKE2 roles follow TLS: A is the TLS client, B the TLS server.

use std::time::{Duration, Instant};

use p256::Scalar;

use super::{Node, Purpose};
use crate::events::{NoticeKind, PairStep, PairingView, Tone};
use crate::model::DeviceId;
use crate::net::link::{Link, LinkId};
use crate::pairing::{self, MAX_TRIES};
use crate::proto::{B32, ErrorCode, Msg, Share};
use crate::spake2::{Keys, Role, Start};
use crate::store::TrustedPeer;

/// How long "Paired with …" stays on screen.
const PAIRED_SHOWN: Duration = Duration::from_secs(8);

pub(super) struct PairSession {
    pub peer: DeviceId,
    pub peer_name: String,
    /// Its address, for "MacBook Pro (192.168.1.20) wants to pair"
    pub peer_addr: String,
    pub link: Option<LinkId>,
    /// This computer asked (its person types the code).
    pub mine: bool,
    pub step: PairStep,
    /// The shower's code, once Show code was pressed.
    code: Option<String>,
    /// The password from the code (the shower's, or what the asker typed).
    w: Option<Scalar>,
    /// This try's exchange (dropped after a wrong code).
    start: Option<Start>,
    keys: Option<Keys>,
    /// Shower: the asker's confirmation checked out; waiting for `pair_done`.
    confirmed: bool,
    /// Wrong codes so far in this pairing.
    wrong: u32,
    pub failed: Option<String>,
    pub started: Instant,
    paired_at: Option<Instant>,
}

impl PairSession {
    fn new(link: &Link, mine: bool, step: PairStep) -> Self {
        let info = link.info();
        Self {
            peer: info.hello.id.clone(),
            peer_name: info.hello.name.clone(),
            peer_addr: info.remote.ip().to_string(),
            link: Some(link.id()),
            mine,
            step,
            code: None,
            w: None,
            start: None,
            keys: None,
            confirmed: false,
            wrong: 0,
            failed: None,
            started: Instant::now(),
            paired_at: None,
        }
    }

    /// What the window shows; `timeout` is how long each step may take.
    pub fn view(&self, timeout: Duration) -> PairingView {
        PairingView {
            peer_id: self.peer.clone(),
            peer_name: self.peer_name.clone(),
            peer_addr: self.peer_addr.clone(),
            mine: self.mine,
            step: self.step,
            // Only the shower ever has the code
            code: if self.mine { None } else { self.code.clone() },
            tries_left: MAX_TRIES - self.wrong.min(MAX_TRIES),
            failed: self.failed.clone(),
            left_ms: timeout.saturating_sub(self.started.elapsed()).as_millis() as u64,
        }
    }

    fn active(&self) -> bool {
        self.failed.is_none() && self.step != PairStep::Paired
    }

    /// Forget this try's exchange (a wrong code, or the end).
    fn forget_try(&mut self) {
        self.start = None;
        self.keys = None;
        self.confirmed = false;
    }
}

/// What the person did about a pairing (UI → core).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairAction {
    /// Shower: show the code
    ShowCode,
    /// Shower: not now (says nothing to the asker)
    Ignore,
    /// Asker: the code as typed
    Code(String),
    /// Either: stop
    Cancel,
    /// Clear "Paired with …" or why it ended
    Dismiss,
    /// After too many wrong codes: allow pairing requests again
    AllowAgain,
}

impl Node {
    /// "Pair" pressed: use a link to the device, dialling one if needed.
    pub(super) fn cmd_pair(&mut self, peer: DeviceId) -> Result<(), String> {
        if self.pairing.as_ref().is_some_and(PairSession::active) {
            return Err("Finish the pairing that’s in progress first.".into());
        }
        let Some(p) = self.peers.get(&peer) else {
            return Err("That device isn’t nearby any more.".into());
        };
        let name = p.name.clone();
        if let Some(link) = self.link_for(&peer) {
            self.start_asking(&link);
            return Ok(());
        }
        let Some(path) = self.best_path(p) else {
            return Err(format!("{name} isn’t reachable right now."));
        };
        // Waiting for the dial: a session without a link yet
        self.pairing = Some(PairSession {
            peer: peer.clone(),
            peer_name: name,
            peer_addr: path.addr.ip().to_string(),
            link: None,
            mine: true,
            step: PairStep::Asking,
            code: None,
            w: None,
            start: None,
            keys: None,
            confirmed: false,
            wrong: 0,
            failed: None,
            started: Instant::now(),
            paired_at: None,
        });
        self.spawn_dial(vec![path.addr], None, Purpose::Pair(peer));
        self.dirty = true;
        Ok(())
    }

    /// The dial for a pairing finished.
    pub(super) fn pair_link_ready(&mut self, peer: &DeviceId, link: Option<Link>) {
        let Some(session) = self.pairing.as_mut().filter(|s| &s.peer == peer && s.link.is_none()) else { return };
        match link {
            Some(link) => self.start_asking(&link),
            None => {
                session.failed = Some(format!(
                    "Couldn’t reach {}. Check that {} is open there.",
                    session.peer_name,
                    crate::consts::APP_NAME
                ));
                self.dirty = true;
            }
        }
    }

    fn start_asking(&mut self, link: &Link) {
        let name = link.info().hello.name.clone();
        link.try_send(Msg::PairAsk);
        self.pairing = Some(PairSession::new(link, true, PairStep::Asking));
        self.say("pair", format!("Asking {name} to pair"), None, None);
        self.dirty = true;
    }

    pub(super) fn cmd_pairing(&mut self, action: PairAction) -> Result<(), String> {
        if action == PairAction::AllowAgain {
            self.fail_cap.allow_again();
            self.say("pair", "Pairing requests are allowed again", None, None);
            self.dirty = true;
            return Ok(());
        }
        let Some(s) = self.pairing.as_mut() else { return Ok(()) };
        let link = s.link.and_then(|id| self.links.get(&id).cloned());
        match action {
            PairAction::Dismiss => {
                if !s.active() {
                    self.pairing = None;
                    self.dirty = true;
                }
            }
            PairAction::ShowCode if !s.mine && s.step == PairStep::Asked && s.active() => {
                let code = pairing::new_code();
                s.w = Some(pairing::password(&code));
                s.code = Some(code);
                s.step = PairStep::Code;
                // The code's lifetime starts now: two minutes to type it
                s.started = Instant::now();
                if let Some(l) = &link {
                    l.try_send(Msg::PairReady);
                }
                self.dirty = true;
            }
            PairAction::Ignore | PairAction::Cancel if !s.mine && s.step == PairStep::Asked => {
                // Nothing goes back: the asker can't tell this from a prompt
                // nobody saw. No new prompt from it for a minute.
                let name = s.peer_name.clone();
                if let Some(l) = &link {
                    self.limiter.cool_down(&l.info().peer_fp.to_hex(), Instant::now());
                    self.limiter.cool_down(&l.info().remote.ip().to_string(), Instant::now());
                }
                self.say("pair", format!("Ignored a pairing request from {name}"), None, Some(Tone::Dim));
                self.pairing = None;
                self.dirty = true;
            }
            PairAction::Cancel if s.active() => {
                let name = s.peer_name.clone();
                if let Some(l) = &link {
                    l.try_send(Msg::PairStop);
                    if !s.mine {
                        self.limiter.cool_down(&l.info().peer_fp.to_hex(), Instant::now());
                        self.limiter.cool_down(&l.info().remote.ip().to_string(), Instant::now());
                    }
                }
                self.say("pair", format!("Cancelled pairing with {name}"), None, None);
                self.pairing = None;
                self.dirty = true;
            }
            PairAction::Code(typed) if s.mine && matches!(s.step, PairStep::Code | PairStep::Wrong) && s.active() => {
                let Some(code) = pairing::normalize_code(&typed) else {
                    return Err(format!("Type the 6 digits shown on {}.", s.peer_name));
                };
                let Some(l) = &link else { return Ok(()) };
                let role = if l.info().dialer { Role::A } else { Role::B };
                let w = pairing::password(&code);
                let start = Start::new(role, w);
                l.try_send(Msg::PairKey { share: Share(start.share()) });
                s.w = Some(w);
                s.start = Some(start);
                s.keys = None;
                s.step = PairStep::Checking;
                self.dirty = true;
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn on_pair_msg(&mut self, link: &Link, msg: Msg) {
        match msg {
            Msg::PairAsk => self.on_pair_ask(link),
            Msg::Error { code: ErrorCode::RateLimited, .. } => {
                let info = link.info();
                if let Some(s) = self.pairing.as_mut().filter(|s| s.active() && s.mine && s.peer == info.hello.id) {
                    s.failed = Some(format!(
                        "{} isn’t taking pairing requests right now. Try again in a minute, or ask whoever uses it.",
                        s.peer_name
                    ));
                    self.dirty = true;
                }
            }
            other => self.on_pair_step(link, other),
        }
    }

    fn on_pair_ask(&mut self, link: &Link) {
        let info = link.info().clone();
        if let Some(s) = self.pairing.as_mut().filter(|s| s.active()) {
            if s.peer == info.hello.id && s.mine && s.step == PairStep::Asking {
                // Both asked each other at once: the TLS client stays the
                // asker, the server shows the code.
                if !info.dialer {
                    return;
                }
                s.mine = false;
                s.step = PairStep::Asked;
                s.link = Some(link.id());
                s.started = Instant::now();
                self.pair_prompt(&info.hello.name, &info.remote.ip().to_string());
                return;
            }
            if s.peer != info.hello.id {
                link.try_send(Msg::Error { code: ErrorCode::RateLimited, msg: "busy pairing".into() });
            }
            return;
        }
        if self.fail_cap.locked() {
            self.say(
                "pair",
                format!(
                    "Turned down a pairing request from {}: pairing is locked after too many wrong codes",
                    info.hello.name
                ),
                Some(info.remote.ip().to_string()),
                Some(Tone::Warn),
            );
            link.try_send(Msg::Error { code: ErrorCode::RateLimited, msg: "pairing locked".into() });
            return;
        }
        // Prompts per key and per address (made-up device ids cost nothing).
        // Over the limit: silence, like Ignore.
        let now = Instant::now();
        let by_key = self.limiter.allow(&info.peer_fp.to_hex(), now);
        let by_ip = self.limiter.allow(&info.remote.ip().to_string(), now);
        if !(by_key && by_ip) {
            self.say(
                "pair",
                format!("Ignored {}: too many pairing requests", info.hello.name),
                Some(info.remote.ip().to_string()),
                Some(Tone::Dim),
            );
            return;
        }
        self.pairing = Some(PairSession::new(link, false, PairStep::Asked));
        self.pair_prompt(&info.hello.name, &info.remote.ip().to_string());
    }

    fn pair_prompt(&mut self, name: &str, addr: &str) {
        self.say("pair", format!("{name} wants to pair"), Some(addr.to_owned()), None);
        self.notice(
            NoticeKind::Pairing,
            format!("{name} wants to pair"),
            format!("From {addr}. Open {} to show a code or ignore it.", crate::consts::APP_NAME),
        );
        self.dirty = true;
    }

    /// The exchange itself, on the session's own link only.
    fn on_pair_step(&mut self, link: &Link, msg: Msg) {
        let info = link.info().clone();
        let my_fp = self.identity.fingerprint;
        let Some(s) = self.pairing.as_mut().filter(|s| s.active() && s.link == Some(link.id())) else { return };
        let (client, server) = if info.dialer { (my_fp, info.peer_fp) } else { (info.peer_fp, my_fp) };
        let ctx = pairing::context(&info.exporter, client, server);
        let name = s.peer_name.clone();
        match (s.mine, msg) {
            (_, Msg::PairStop) => {
                s.failed = Some(match s.step {
                    PairStep::Asked => format!("{name} stopped asking to pair."),
                    _ => format!("{name} cancelled pairing. Nothing was paired."),
                });
                s.forget_try();
                self.say("pair", format!("{name} cancelled pairing"), None, None);
                self.dirty = true;
            }
            // Asker
            (true, Msg::PairReady) if s.step == PairStep::Asking => {
                s.step = PairStep::Code;
                s.started = Instant::now();
                self.dirty = true;
            }
            (true, Msg::PairKey { share }) if s.step == PairStep::Checking && s.keys.is_none() => {
                let Some(start) = &s.start else { return };
                match start.finish(&ctx, &share.0) {
                    Ok(keys) => {
                        link.try_send(Msg::PairCheck { mac: B32(keys.confirmation()) });
                        s.keys = Some(keys);
                    }
                    Err(e) => self.end_pairing(link, "Couldn’t pair: the other computer’s answer wasn’t valid.", e),
                }
            }
            (true, Msg::PairWrong { tries_left }) if s.step == PairStep::Checking => {
                s.wrong = MAX_TRIES - tries_left.min(MAX_TRIES);
                s.forget_try();
                if tries_left == 0 {
                    s.failed = Some(format!(
                        "That wasn’t the code shown on {name}, {MAX_TRIES} times. Nothing was paired. Pair again for a new code."
                    ));
                } else {
                    s.step = PairStep::Wrong;
                }
                self.say("pair", format!("Wrong code for {name}"), Some(format!("{tries_left} tries left")), None);
                self.dirty = true;
            }
            (true, Msg::PairCheck { mac }) if s.step == PairStep::Checking && s.keys.is_some() => {
                let ok = s.keys.as_ref().is_some_and(|k| k.verify(&mac.0));
                if ok {
                    link.try_send(Msg::PairDone);
                    self.finish_pairing(link);
                } else {
                    self.end_pairing(
                        link,
                        "Couldn’t check the other computer. Nothing was paired.",
                        "its confirmation didn’t match",
                    );
                }
            }
            // Shower
            (false, Msg::PairKey { share }) if matches!(s.step, PairStep::Code | PairStep::Wrong) => {
                let Some(w) = s.w else { return };
                let start = Start::new(if info.dialer { Role::A } else { Role::B }, w);
                match start.finish(&ctx, &share.0) {
                    Ok(keys) => {
                        link.try_send(Msg::PairKey { share: Share(start.share()) });
                        s.start = Some(start);
                        s.keys = Some(keys);
                        s.step = PairStep::Checking;
                        self.dirty = true;
                    }
                    Err(e) => self.end_pairing(link, "Couldn’t pair: the other computer’s answer wasn’t valid.", e),
                }
            }
            (false, Msg::PairCheck { mac }) if s.step == PairStep::Checking && s.keys.is_some() && !s.confirmed => {
                let Some(keys) = s.keys.as_ref() else { return };
                if keys.verify(&mac.0) {
                    link.try_send(Msg::PairCheck { mac: B32(keys.confirmation()) });
                    s.confirmed = true;
                    return;
                }
                // A wrong code: counted here and across every requester
                s.wrong += 1;
                s.forget_try();
                let was_locked = self.fail_cap.locked();
                self.fail_cap.failed();
                let who = format!("{name} ({})", info.remote.ip());
                self.last_wrong = Some((who.clone(), crate::util::now_ms()));
                let newly_locked = self.fail_cap.locked() && !was_locked;
                let tries_left = if self.fail_cap.locked() { 0 } else { MAX_TRIES.saturating_sub(s.wrong) };
                link.try_send(Msg::PairWrong { tries_left });
                if tries_left > 0 {
                    s.step = PairStep::Wrong;
                    self.say(
                        "pair",
                        format!("A wrong code was typed on {name}"),
                        Some(format!("{tries_left} tries left")),
                        None,
                    );
                } else {
                    let wrong = s.wrong;
                    link.try_send(Msg::PairStop);
                    s.failed = Some(if self.fail_cap.locked() {
                        "The code was typed wrong too many times. Nothing was paired, and pairing is locked until you allow it again in Settings."
                            .to_owned()
                    } else {
                        format!("The code was typed wrong {MAX_TRIES} times on {name}. Nothing was paired.")
                    });
                    self.say(
                        "pair",
                        format!("Wrong code {wrong} times on {name}; nothing was paired"),
                        None,
                        Some(Tone::Warn),
                    );
                }
                if newly_locked {
                    // The one sign someone may be guessing: on screen, in
                    // Details and as a notification
                    self.say(
                        "pair",
                        "Pairing is locked: too many wrong codes",
                        Some(format!("the last from {who}; locked until allowed again in Settings")),
                        Some(Tone::Err),
                    );
                    self.notice(
                        NoticeKind::Pairing,
                        "Pairing is locked".into(),
                        format!("Too many wrong codes, the last from {who}. Nothing was paired."),
                    );
                }
                self.dirty = true;
            }
            (false, Msg::PairDone) if s.step == PairStep::Checking && s.confirmed => self.finish_pairing(link),
            // Anything else on this link is out of order: stop
            (_, other) => self.end_pairing(
                link,
                "The pairing messages arrived out of order. Nothing was paired.",
                format!("{other:?}"),
            ),
        }
    }

    /// Stops a pairing that went wrong in the exchange, telling the other side.
    fn end_pairing(&mut self, link: &Link, why: &str, detail: impl std::fmt::Display) {
        link.try_send(Msg::PairStop);
        if let Some(s) = self.pairing.as_mut() {
            s.failed = Some(why.into());
            s.forget_try();
        }
        self.say("pair", why.to_owned(), Some(detail.to_string()), Some(Tone::Err));
        self.dirty = true;
    }

    fn finish_pairing(&mut self, link: &Link) {
        let info = link.info().clone();
        let addr = super::with_port(info.remote, info.hello.port).to_string();
        // Paired again (it removed this computer, say): keep what was set here
        let before = self.store.peer(&info.hello.id).cloned();
        let peer = TrustedPeer {
            id: info.hello.id.clone(),
            name: before.as_ref().filter(|p| p.renamed).map_or_else(|| info.hello.name.clone(), |p| p.name.clone()),
            os: info.hello.os,
            fingerprint: info.peer_fp,
            addrs: vec![addr],
            trusted: true,
            incoming: crate::model::Incoming::Ask,
            paired_at: crate::util::now_ms() / 1000,
            renamed: before.as_ref().is_some_and(|p| p.renamed),
            disconnected: false,
            clip_sync: false,
            clip_sync_at: 0,
            clip_to: before.as_ref().map(|p| p.clip_to).unwrap_or_default(),
            // Asked fresh at pairing: "Share the Shelf with it?" (off until both say yes)
            shelf_share: false,
            shelf_share_at: 0,
            shelf_answered: false,
            folder: before.and_then(|p| p.folder),
        };
        if let Err(e) = self.store.upsert_peer(peer) {
            self.say("error", "Couldn’t save the pairing", Some(e.to_string()), Some(Tone::Err));
            if let Some(s) = self.pairing.as_mut() {
                s.failed = Some("Couldn’t save the pairing. Nothing was paired.".into());
            }
            self.dirty = true;
            return;
        }
        if let Some(s) = self.pairing.as_mut() {
            s.step = PairStep::Paired;
            s.paired_at = Some(Instant::now());
            s.forget_try();
            s.code = None;
            s.w = None;
        }
        let name = info.hello.name.clone();
        self.say("pair", format!("Paired with {name}"), Some(format!("key {}", &info.peer_fp.to_hex()[..12])), None);
        self.notice(
            NoticeKind::Paired,
            format!("Paired with {name}"),
            format!("Files and the clipboard can go between this computer and {name} now."),
        );
        // Anything else claiming this id with another key is an impostor.
        self.close_impostors(&info.hello.id);
        // A fresh pairing starts at Ask: say so, replacing anything it heard before
        self.send_status(link);
        self.dirty = true;
    }

    /// Timeouts, lost links, and "Paired with …" going away.
    pub(super) fn pairing_tick(&mut self) {
        let Some(s) = self.pairing.as_mut() else { return };
        if s.step == PairStep::Paired {
            if s.paired_at.is_some_and(|t| t.elapsed() > PAIRED_SHOWN) {
                self.pairing = None;
                self.dirty = true;
            }
            return;
        }
        if !s.active() {
            return;
        }
        let lost = s.link.is_some_and(|id| !self.links.contains_key(&id));
        if lost {
            s.failed = Some(format!("Lost the connection to {}. Nothing was paired.", s.peer_name));
            s.forget_try();
            self.dirty = true;
        } else if s.started.elapsed() > self.cfg.pair_timeout {
            // The asker can't tell an ignored request from one nobody saw
            s.failed = Some(match (s.mine, s.step) {
                (true, PairStep::Asking) => {
                    format!("No answer from {} within 2 minutes. Nothing was paired.", s.peer_name)
                }
                (false, PairStep::Asked) => {
                    self.pairing = None;
                    self.dirty = true;
                    return;
                }
                // The code was shown and nobody typed it in time
                (false, _) => format!(
                    "The code expired after 2 minutes. Nothing was paired. To try again, press Pair on {}.",
                    s.peer_name
                ),
                (true, _) => {
                    "The code expired after 2 minutes. Nothing was paired. Press Pair again for a new one.".into()
                }
            });
            s.forget_try();
            if let Some(l) = s.link.and_then(|id| self.links.get(&id)) {
                l.try_send(Msg::PairStop);
            }
            self.dirty = true;
        }
    }

    pub(super) fn pairing_locked(&self) -> bool {
        self.fail_cap.locked()
    }
}
