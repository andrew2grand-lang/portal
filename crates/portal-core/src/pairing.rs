//! Pairing by typed code (Mission 04 §2): one computer shows a fresh random
//! 6-digit code, the person types it on the other, and the code goes in only
//! through SPAKE2 (`crate::spake2`, RFC 9382). Nothing derived from the code
//! ever travels, so it can't be tried offline; each try costs an exchange,
//! and tries are capped.
//!
//! Roles are fixed by TLS: A is the TLS client, B the TLS server, whichever
//! computer asked. Both certificate fingerprints and the TLS exporter are in
//! the identities (so in the transcript) and in the confirmation keys' data,
//! so a man in the middle (two TLS sessions) fails on both sides.
//! The full message flow is in `docs/PROTOCOL.md`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use p256::Scalar;

use crate::model::Fingerprint;
use crate::spake2::{Context, scalar_from_wide};

/// Pairing requests allowed per device key or address within `WINDOW`.
pub const MAX_ATTEMPTS: usize = 3;
pub const WINDOW: Duration = Duration::from_secs(600);
/// After Ignore or a cancel the same device can't raise a prompt again for this long.
pub const COOL_DOWN: Duration = Duration::from_secs(60);
/// From the request to "Paired with …": then it's abandoned.
pub const TIMEOUT: Duration = Duration::from_secs(120);
/// Wrong codes allowed in one pairing (typos), then it ends.
pub const MAX_TRIES: u32 = 3;
/// Wrong codes from anyone, then pairing is locked here until the person
/// allows it again.
pub const LOCK_AFTER: u32 = 10;

const LABEL: &[u8] = b"SeamDrop pairing v1";

/// A fresh random code: six digits, each value equally likely.
pub fn new_code() -> String {
    loop {
        let n = u32::from_be_bytes(crate::util::random_bytes::<4>());
        // Rejection sampling: below the largest multiple of a million
        if n < 4_294_000_000 {
            return format!("{:06}", n % 1_000_000);
        }
    }
}

/// What was typed, as six digits ("048 213" → "048213"), or None.
pub fn normalize_code(typed: &str) -> Option<String> {
    let kept: String = typed.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    (kept.len() == 6 && kept.chars().all(|c| c.is_ascii_digit())).then_some(kept)
}

/// SPAKE2's w for a code: HKDF-SHA256 (salt "SeamDrop pairing v1", the six
/// ASCII digits, info "w") to 48 bytes, reduced mod p. No memory-hard hash:
/// a code is used for one exchange and never travels in any form, so there's
/// nothing to attack offline.
pub fn password(code: &str) -> Scalar {
    let mut wide = [0u8; 48];
    ring::hkdf::Salt::new(ring::hkdf::HKDF_SHA256, LABEL)
        .extract(code.as_bytes())
        .expand(&[b"w"], Len(48))
        .and_then(|okm| okm.fill(&mut wide))
        .expect("48 bytes is a valid HKDF-SHA256 output length");
    scalar_from_wide(&wide)
}

/// The identities and data for one exchange, from the TLS session (never
/// from messages): A = label ‖ "A" ‖ client fingerprint ‖ exporter,
/// B = label ‖ "B" ‖ server fingerprint ‖ exporter, AAD = label ‖ exporter ‖
/// client fingerprint ‖ server fingerprint.
pub fn context(exporter: &[u8; 32], client: Fingerprint, server: Fingerprint) -> Context {
    let id = |role: &[u8], fp: &Fingerprint| [LABEL, b" ", role, fp.as_bytes(), exporter].concat();
    Context {
        a: id(b"A", &client),
        b: id(b"B", &server),
        aad: [LABEL, exporter, client.as_bytes(), server.as_bytes()].concat(),
    }
}

/// Wrong codes from anyone (Mission 04 §2: a limit per device or address
/// alone isn't enough; new keys cost nothing and IPv6 addresses change).
#[derive(Debug, Default)]
pub struct FailCap {
    failures: u32,
}

impl FailCap {
    pub fn failed(&mut self) {
        self.failures = self.failures.saturating_add(1);
    }

    pub fn locked(&self) -> bool {
        self.failures >= LOCK_AFTER
    }

    /// "Allow pairing again": the person pressed it.
    pub fn allow_again(&mut self) {
        self.failures = 0;
    }
}

/// Limits pairing prompts per device key and per remote address.
#[derive(Debug)]
pub struct Limiter {
    max: usize,
    attempts: HashMap<String, Vec<Instant>>,
    cooling: HashMap<String, Instant>,
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new(MAX_ATTEMPTS)
    }
}

impl Limiter {
    /// `max` requests per key within `WINDOW`.
    pub fn new(max: usize) -> Self {
        Self { max, attempts: HashMap::new(), cooling: HashMap::new() }
    }

    /// Records an attempt for `key` and says whether it may go ahead.
    pub fn allow(&mut self, key: &str, now: Instant) -> bool {
        self.evict(now);
        if self.cooling.get(key).is_some_and(|until| now < *until) {
            return false;
        }
        let list = self.attempts.entry(key.to_owned()).or_default();
        list.retain(|t| now.duration_since(*t) < WINDOW);
        if list.len() >= self.max {
            return false;
        }
        list.push(now);
        true
    }

    /// No more prompts from `key` for a minute (after Ignore or a cancel).
    pub fn cool_down(&mut self, key: &str, now: Instant) {
        self.cooling.insert(key.to_owned(), now + COOL_DOWN);
    }

    /// Keys with live state (for tests and bounds).
    pub fn tracked(&self) -> usize {
        self.attempts.len().max(self.cooling.len())
    }

    /// Forgets keys whose attempts and cool-downs have all expired, so a
    /// stream of made-up keys can't grow memory.
    fn evict(&mut self, now: Instant) {
        self.attempts.retain(|_, list| {
            list.retain(|t| now.duration_since(*t) < WINDOW);
            !list.is_empty()
        });
        self.cooling.retain(|_, until| now < *until);
    }
}

/// HKDF output length for ring.
struct Len(usize);

impl ring::hkdf::KeyType for Len {
    fn len(&self) -> usize {
        self.0
    }
}
