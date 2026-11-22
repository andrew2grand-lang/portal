//! The Shelf (Mission 05 §3.2): text you deliberately keep, shared with the
//! devices you agree to share it with, kept on disk, encrypted.
//!
//! - **Records.** Each item has an id and the device it was added on (its
//!   origin). A change (add, edit, removal) is a new version of the record
//!   with a stamp of `max(now, last + 1)` milliseconds (decision 156: newer
//!   than anything seen, whatever the clocks say) and the device that made
//!   it, which breaks ties. The newest version wins everywhere.
//! - **Removals leave markers** (the record, emptied) for `KEEP_REMOVED`, so a
//!   device that was away for months can't bring an item back. At most
//!   `MAX_REMOVED` markers are kept (the oldest go first).
//! - **No relaying.** An item goes from its origin to the devices that share
//!   the Shelf with that origin directly, and no further: a device only ever
//!   sends records whose origin is itself or the device it's talking to (an
//!   edit of that device's item, going back to it). The origin passes its own
//!   items on.
//! - **Limits.** `MAX_ITEMS` live items and `MAX_TEXT` bytes each; ids,
//!   names, sizes and stamps from other devices are checked before anything
//!   is kept.
//! - **At rest** the whole Shelf is one file sealed with ChaCha20-Poly1305
//!   under a random key kept beside it, protected by the platform's secret
//!   store where there is one (`portal_sys::secret`: DPAPI on Windows; on
//!   macOS a 0600 file, decision 271 says what that does and doesn't stop).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ring::aead::{self, Aad, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

use crate::model::DeviceId;

/// Live items on one Shelf.
pub const MAX_ITEMS: usize = 200;
/// Bytes of text per item (UTF-8).
pub const MAX_TEXT: usize = 64 * 1024;
/// Characters in an item's name.
pub const MAX_NAME: usize = 80;
/// Removal markers are kept this long (a device away longer can't bring an
/// item back after that, but nobody is away for 400 days and still paired)…
pub const KEEP_REMOVED_MS: u64 = 400 * 24 * 60 * 60 * 1000;
/// …and at most this many (about 100 bytes each).
pub const MAX_REMOVED: usize = 4000;
/// A stamp further ahead than this (a broken clock) is refused.
const AHEAD_MS: u64 = 24 * 60 * 60 * 1000;

/// One version of one Shelf item, as kept and as sent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShelfRecord {
    /// 32 hex digits, random.
    pub id: String,
    /// The device it was added on.
    pub origin: DeviceId,
    /// "Vercel key"; empty for none.
    #[serde(default)]
    pub name: String,
    /// The text itself ("" once removed).
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub private: bool,
    /// When this version was made (ms, decision 156) and by whom.
    pub stamp: u64,
    pub by: DeviceId,
    /// When the item was first added (ms), for "from MacBook · 14:02".
    pub added: u64,
    /// A removal marker.
    #[serde(default)]
    pub removed: bool,
}

impl ShelfRecord {
    /// Newer than `other` (stamp, then device id to break a tie).
    pub fn newer_than(&self, other: &ShelfRecord) -> bool {
        (self.stamp, &self.by) > (other.stamp, &other.by)
    }
}

/// Why something from another device wasn't kept.
#[derive(Debug, PartialEq, Eq)]
pub enum Refused {
    /// Its origin is neither the sender nor this device: no relaying.
    Relayed,
    Invalid(&'static str),
    /// This Shelf is full.
    Full,
}

pub fn new_id() -> String {
    let mut b = [0u8; 16];
    SystemRandom::new().fill(&mut b).expect("system randomness");
    crate::util::to_hex(&b)
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Checks a record from another device before anything is kept.
pub fn check(r: &ShelfRecord, now_ms: u64) -> Result<(), Refused> {
    if !valid_id(&r.id) {
        return Err(Refused::Invalid("id"));
    }
    if r.text.len() > MAX_TEXT {
        return Err(Refused::Invalid("text too long"));
    }
    if r.name.chars().count() > MAX_NAME || r.name.contains(['\n', '\r']) {
        return Err(Refused::Invalid("name"));
    }
    if r.stamp > now_ms + AHEAD_MS || r.added > now_ms + AHEAD_MS {
        return Err(Refused::Invalid("stamp in the future"));
    }
    if r.removed && (!r.text.is_empty() || !r.name.is_empty()) {
        return Err(Refused::Invalid("a removal with text"));
    }
    Ok(())
}

/// Everything on this computer's Shelf, by id.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Shelf {
    pub records: HashMap<String, ShelfRecord>,
    /// The newest stamp seen from anyone (decision 156).
    pub last: u64,
}

impl Shelf {
    /// A stamp newer than anything seen.
    pub fn stamp(&mut self, now_ms: u64) -> u64 {
        self.last = now_ms.max(self.last + 1);
        self.last
    }

    pub fn live(&self) -> impl Iterator<Item = &ShelfRecord> {
        self.records.values().filter(|r| !r.removed)
    }

    pub fn live_count(&self) -> usize {
        self.live().count()
    }

    /// Takes `r` if it's newer than what's here. True if it changed anything.
    pub fn apply(&mut self, r: ShelfRecord) -> bool {
        self.last = self.last.max(r.stamp);
        match self.records.get(&r.id) {
            Some(have) if !r.newer_than(have) => false,
            _ => {
                self.records.insert(r.id.clone(), r);
                true
            }
        }
    }

    /// A record from `from`, checked: no relaying, valid, and room for it.
    pub fn accept(&mut self, me: &DeviceId, from: &DeviceId, r: ShelfRecord, now_ms: u64) -> Result<bool, Refused> {
        if &r.origin != from && &r.origin != me {
            return Err(Refused::Relayed);
        }
        check(&r, now_ms)?;
        let new_live = !r.removed && self.records.get(&r.id).is_none_or(|h| h.removed);
        if new_live && self.live_count() >= MAX_ITEMS {
            return Err(Refused::Full);
        }
        Ok(self.apply(r))
    }

    /// What to send `to`: records whose origin is this device or `to`.
    pub fn for_peer(&self, me: &DeviceId, to: &DeviceId) -> Vec<ShelfRecord> {
        let mut v: Vec<ShelfRecord> =
            self.records.values().filter(|r| &r.origin == me || &r.origin == to).cloned().collect();
        v.sort_by_key(|r| (r.added, r.id.clone()));
        v
    }

    /// Forgets everything that came from `origin` (unshared or unpaired).
    pub fn forget_origin(&mut self, origin: &DeviceId) -> usize {
        let before = self.records.len();
        self.records.retain(|_, r| &r.origin != origin);
        before - self.records.len()
    }

    /// Old removal markers go: older than `KEEP_REMOVED_MS`, and past `MAX_REMOVED`.
    pub fn prune(&mut self, now_ms: u64) {
        self.records.retain(|_, r| !r.removed || now_ms.saturating_sub(r.stamp) < KEEP_REMOVED_MS);
        let mut removed: Vec<(u64, String)> =
            self.records.values().filter(|r| r.removed).map(|r| (r.stamp, r.id.clone())).collect();
        if removed.len() > MAX_REMOVED {
            removed.sort();
            for (_, id) in removed.iter().take(removed.len() - MAX_REMOVED) {
                self.records.remove(id);
            }
        }
    }
}

// ---- at rest ----------------------------------------------------------------

/// The Shelf's files in the data folder.
pub struct ShelfFiles {
    pub data: PathBuf,
    pub key: PathBuf,
}

impl ShelfFiles {
    pub fn in_dir(dir: &Path) -> Self {
        Self { data: dir.join("shelf.sealed"), key: dir.join("shelf.key") }
    }

    /// The key, made on first use and protected by the platform's secret
    /// store (`portal_sys::secret`).
    fn key(&self) -> std::io::Result<LessSafeKey> {
        let raw = match std::fs::read(&self.key) {
            Ok(sealed) => portal_sys::secret::unprotect(&sealed)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut k = vec![0u8; 32];
                SystemRandom::new().fill(&mut k).map_err(|_| std::io::Error::other("no randomness"))?;
                portal_sys::fs::write_private_file(&self.key, &portal_sys::secret::protect(&k)?)?;
                k
            }
            Err(e) => return Err(e),
        };
        let key = UnboundKey::new(&CHACHA20_POLY1305, &raw).map_err(|_| std::io::Error::other("bad Shelf key"))?;
        Ok(LessSafeKey::new(key))
    }

    /// Reads the Shelf (empty if there's none yet).
    pub fn load(&self) -> std::io::Result<Shelf> {
        let sealed = match std::fs::read(&self.data) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Shelf::default()),
            Err(e) => return Err(e),
        };
        if sealed.len() < aead::NONCE_LEN {
            return Err(std::io::Error::other("the Shelf file is cut short"));
        }
        let (nonce, body) = sealed.split_at(aead::NONCE_LEN);
        let mut body = body.to_vec();
        let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| std::io::Error::other("bad nonce"))?;
        let plain = self
            .key()?
            .open_in_place(nonce, Aad::from(b"seamdrop-shelf-1"), &mut body)
            .map_err(|_| std::io::Error::other("the Shelf file couldn't be opened with this computer's key"))?;
        serde_json::from_slice(plain).map_err(std::io::Error::other)
    }

    /// Writes the Shelf, sealed, replacing the file atomically.
    pub fn save(&self, shelf: &Shelf) -> std::io::Result<()> {
        let mut body = serde_json::to_vec(shelf).map_err(std::io::Error::other)?;
        let mut nonce = [0u8; aead::NONCE_LEN];
        SystemRandom::new().fill(&mut nonce).map_err(|_| std::io::Error::other("no randomness"))?;
        self.key()?
            .seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::from(b"seamdrop-shelf-1"), &mut body)
            .map_err(|_| std::io::Error::other("couldn't seal the Shelf"))?;
        let mut out = nonce.to_vec();
        out.extend_from_slice(&body);
        portal_sys::fs::write_private_file(&self.data, &out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(c: char) -> DeviceId {
        DeviceId::parse(&c.to_string().repeat(32)).unwrap()
    }

    fn rec(origin: &DeviceId, id: &str, text: &str, stamp: u64) -> ShelfRecord {
        ShelfRecord {
            id: id.into(),
            origin: origin.clone(),
            name: String::new(),
            text: text.into(),
            private: false,
            stamp,
            by: origin.clone(),
            added: stamp,
            removed: false,
        }
    }

    const ID: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn the_newest_version_wins_and_ties_go_to_the_higher_device() {
        let (a, b) = (dev('a'), dev('b'));
        let mut s = Shelf::default();
        assert!(s.apply(rec(&a, ID, "one", 10)));
        assert!(!s.apply(rec(&a, ID, "older", 9)));
        let mut tie = rec(&a, ID, "tie from b", 10);
        tie.by = b.clone();
        assert!(s.apply(tie), "same stamp: the higher device id wins");
        assert_eq!(s.records[ID].text, "tie from b");
    }

    #[test]
    fn stamps_are_newer_than_anything_seen_whatever_the_clock() {
        let mut s = Shelf::default();
        s.apply(rec(&dev('a'), ID, "from a clock ahead", 5_000));
        assert_eq!(s.stamp(1_000), 5_001);
        assert_eq!(s.stamp(9_000), 9_000);
    }

    #[test]
    fn nothing_is_relayed_and_everything_is_checked() {
        let (me, b, c) = (dev('a'), dev('b'), dev('c'));
        let mut s = Shelf::default();
        assert_eq!(s.accept(&me, &b, rec(&c, ID, "c's item via b", 1), 10), Err(Refused::Relayed));
        assert_eq!(s.accept(&me, &b, rec(&b, "nope", "x", 1), 10), Err(Refused::Invalid("id")));
        let big = "x".repeat(MAX_TEXT + 1);
        assert_eq!(s.accept(&me, &b, rec(&b, ID, &big, 1), 10), Err(Refused::Invalid("text too long")));
        assert_eq!(
            s.accept(&me, &b, rec(&b, ID, "x", 10 + AHEAD_MS + 1), 10),
            Err(Refused::Invalid("stamp in the future"))
        );
        assert_eq!(s.accept(&me, &b, rec(&b, ID, "fine", 1), 10), Ok(true));
        // An edit of my own item, coming back from b, is fine too.
        let mut mine = rec(&me, "ffffffffffffffffffffffffffffffff", "mine", 2);
        mine.by = b;
        assert_eq!(s.accept(&me, &dev('b'), mine, 10), Ok(true));
    }

    #[test]
    fn a_full_shelf_refuses_more() {
        let (me, b) = (dev('a'), dev('b'));
        let mut s = Shelf::default();
        for i in 0..MAX_ITEMS {
            s.apply(rec(&me, &format!("{i:032x}"), "x", i as u64 + 1));
        }
        let r = rec(&b, "ffffffffffffffffffffffffffffffff", "one more", 9_999);
        assert_eq!(s.accept(&me, &b, r, 10_000), Err(Refused::Full));
    }

    #[test]
    fn removal_markers_age_out_and_are_capped() {
        let a = dev('a');
        let mut s = Shelf::default();
        let mut old = rec(&a, ID, "", 1);
        old.removed = true;
        s.apply(old);
        s.prune(KEEP_REMOVED_MS + 10);
        assert!(s.records.is_empty());
        for i in 0..MAX_REMOVED + 5 {
            let mut r = rec(&a, &format!("{i:032x}"), "", 1_000 + i as u64);
            r.removed = true;
            s.apply(r);
        }
        s.prune(2_000_000);
        assert_eq!(s.records.len(), MAX_REMOVED);
        assert!(!s.records.contains_key(&format!("{:032x}", 0)), "the oldest went first");
    }

    #[test]
    fn it_is_sealed_at_rest_and_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let files = ShelfFiles::in_dir(dir.path());
        let mut s = Shelf::default();
        let mut r = rec(&dev('a'), ID, "sk_live_SECRET_123", 5);
        r.private = true;
        r.name = "Stripe key".into();
        s.apply(r);
        files.save(&s).unwrap();
        let raw = std::fs::read(&files.data).unwrap();
        let hay = String::from_utf8_lossy(&raw);
        assert!(!hay.contains("SECRET") && !hay.contains("Stripe"), "no plaintext at rest");
        let back = files.load().unwrap();
        assert_eq!(back.records[ID].text, "sk_live_SECRET_123");
        // Another key can't open it.
        std::fs::remove_file(&files.key).unwrap();
        assert!(files.load().is_err());
    }
}
