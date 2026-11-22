//! Where each paired device's connection stands (Mission 04 §1.1).
//!
//! A *connecting period* starts when the app starts, wakes, moves networks,
//! on Try again or Connect, and when a link drops. During its first `BURST`
//! the device is dialled every second; after that every 2, 4, 8, then 15 s.
//! A fresh sighting dials at once (without starting a new period).
//!
//! The UI shows "Connecting to …" until the period is older than the grace
//! **and** at least `MIN_FAILURES` real attempts failed; only then "isn't
//! reachable". It never decides this from timers of its own.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::events::{ReachState, ReachStep, ReachView};
use crate::model::DeviceId;
use crate::util::now_ms;

/// Dial every second for this long after a connecting period starts.
pub const BURST: Duration = Duration::from_secs(20);
/// "Isn't reachable" no sooner than this after a period starts (the default
/// of `Config::reach_grace`).
pub const GRACE: Duration = Duration::from_secs(20);
/// …and only after this many attempts failed.
pub const MIN_FAILURES: u32 = 2;
const FAST: Duration = Duration::from_secs(1);
const MAX_RETRY: Duration = Duration::from_secs(15);
/// With no address to try, "not found" counts as a failed attempt this often.
const LOOK_EVERY: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct Reach {
    since: Instant,
    since_ms: u64,
    attempts: u32,
    failures: u32,
    last_failure: Option<String>,
    last_look: Option<Instant>,
    next: Instant,
    delay: Duration,
    step: ReachStep,
    /// Details has said "isn't reachable" for this connecting period.
    told: bool,
}

impl Reach {
    fn new(now: Instant) -> Self {
        Self {
            since: now,
            since_ms: now_ms(),
            attempts: 0,
            failures: 0,
            last_failure: None,
            last_look: None,
            next: now,
            delay: Duration::ZERO,
            step: ReachStep::Looking,
            told: false,
        }
    }
}

#[derive(Debug)]
pub(super) struct Reaches {
    map: HashMap<DeviceId, Reach>,
    grace: Duration,
}

impl Reaches {
    pub fn new(grace: Duration) -> Self {
        Self { map: HashMap::new(), grace }
    }

    fn entry(&mut self, id: &DeviceId, now: Instant) -> &mut Reach {
        self.map.entry(id.clone()).or_insert_with(|| Reach::new(now))
    }

    /// A new connecting period for one device (Connect, a dropped link).
    pub fn start(&mut self, id: &DeviceId, now: Instant) {
        self.map.insert(id.clone(), Reach::new(now));
    }

    /// A new connecting period for every device that isn't connected
    /// (start, wake, a network change, Try again).
    pub fn start_all(&mut self, now: Instant) {
        for r in self.map.values_mut() {
            *r = Reach::new(now);
        }
    }

    /// Connected: nothing to report until it drops.
    pub fn connected(&mut self, id: &DeviceId) {
        self.map.remove(id);
    }

    pub fn forget(&mut self, id: &DeviceId) {
        self.map.remove(id);
    }

    /// Time to dial again?
    pub fn due(&self, id: &DeviceId, now: Instant) -> bool {
        self.map.get(id).is_none_or(|r| now >= r.next)
    }

    /// A dial is starting: count it and book the next one.
    pub fn dialing(&mut self, id: &DeviceId, now: Instant, step: ReachStep) {
        let r = self.entry(id, now);
        r.attempts += 1;
        r.step = step;
        r.delay = if now.duration_since(r.since) < BURST { FAST } else { (r.delay * 2).clamp(FAST * 2, MAX_RETRY) };
        r.next = now + r.delay;
    }

    pub fn failed(&mut self, id: &DeviceId, why: String) {
        let now = Instant::now();
        let r = self.entry(id, now);
        r.failures += 1;
        r.last_failure = Some(why);
    }

    /// No address to try: looking. Counts as a failed attempt now and then,
    /// so a device that's nowhere to be found ends up "isn't reachable" too.
    pub fn looking(&mut self, id: &DeviceId, now: Instant) {
        let r = self.entry(id, now);
        r.step = ReachStep::Looking;
        if r.last_look.is_none_or(|t| now.duration_since(t) >= LOOK_EVERY) {
            r.last_look = Some(now);
            if now.duration_since(r.since) >= LOOK_EVERY {
                r.failures += 1;
                r.last_failure = Some("Not found on this network".into());
            }
        }
    }

    /// Seen on the network just now: dial at the next chance.
    pub fn soon(&mut self, id: &DeviceId, now: Instant) {
        self.entry(id, now).next = now;
    }

    /// Don't dial for a while (it said it disconnected from us).
    pub fn hold(&mut self, id: &DeviceId, now: Instant, wait: Duration) {
        let r = self.entry(id, now);
        r.next = now + wait;
        r.delay = MAX_RETRY;
    }

    /// Devices that just became "isn't reachable", with why: each once per
    /// connecting period, for Details.
    pub fn newly_unreachable(&mut self) -> Vec<(DeviceId, Option<String>)> {
        let grace = self.grace;
        self.map
            .iter_mut()
            .filter(|(_, r)| !r.told && r.since.elapsed() >= grace && r.failures >= MIN_FAILURES)
            .map(|(id, r)| {
                r.told = true;
                (id.clone(), r.last_failure.clone())
            })
            .collect()
    }

    pub fn view(&self, id: &DeviceId) -> Option<ReachView> {
        let r = self.map.get(id)?;
        let unreachable = r.since.elapsed() >= self.grace && r.failures >= MIN_FAILURES;
        Some(ReachView {
            state: if unreachable { ReachState::Unreachable } else { ReachState::Connecting },
            since_ms: r.since_ms,
            attempts: r.attempts,
            last_failure: r.last_failure.clone(),
            next_try_ms: (r.next > Instant::now())
                .then(|| now_ms() + r.next.saturating_duration_since(Instant::now()).as_millis() as u64),
            step: r.step,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> DeviceId {
        DeviceId::random()
    }

    #[test]
    fn dials_every_second_for_the_first_twenty_seconds_then_backs_off() {
        let mut r = Reaches::new(GRACE);
        let d = id();
        let t0 = Instant::now();
        r.start(&d, t0);
        let mut at = t0;
        let mut gaps = Vec::new();
        for _ in 0..26 {
            assert!(r.due(&d, at));
            r.dialing(&d, at, ReachStep::TryingKnown);
            let next = r.map[&d].next;
            gaps.push(next.duration_since(at).as_secs());
            at = next;
        }
        assert!(gaps[..20].iter().all(|g| *g == 1), "{gaps:?}");
        assert_eq!(&gaps[20..], [2, 4, 8, 15, 15, 15]);
    }

    #[test]
    fn isnt_reachable_only_after_the_grace_and_two_failures() {
        let mut r = Reaches::new(Duration::from_millis(50));
        let d = id();
        r.start(&d, Instant::now());
        assert_eq!(r.view(&d).unwrap().state, ReachState::Connecting);
        r.failed(&d, "Nothing answered".into());
        r.failed(&d, "Nothing answered".into());
        assert_eq!(r.view(&d).unwrap().state, ReachState::Connecting, "too early");
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(r.view(&d).unwrap().state, ReachState::Unreachable);
        r.start_all(Instant::now());
        assert_eq!(r.view(&d).unwrap().state, ReachState::Connecting, "a new period starts over");
        assert_eq!(r.view(&d).unwrap().attempts, 0);
    }

    #[test]
    fn a_sighting_dials_at_once_without_starting_over() {
        let mut r = Reaches::new(GRACE);
        let d = id();
        let t0 = Instant::now();
        r.start(&d, t0);
        r.dialing(&d, t0, ReachStep::TryingKnown);
        r.failed(&d, "x".into());
        let later = t0 + Duration::from_millis(300);
        assert!(!r.due(&d, later));
        r.soon(&d, later);
        assert!(r.due(&d, later));
        assert_eq!(r.view(&d).unwrap().attempts, 1, "same period");
    }
}
