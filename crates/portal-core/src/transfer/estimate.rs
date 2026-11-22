//! Speed and time left that people can read (Mission 04 §1.2, redone in
//! Mission 05 §1.3).
//!
//! Fed `(time, bytes)` about ten times a second. Bytes are what the other
//! side has *acknowledged* on the sender (`progress` reports), what arrived
//! on the receiver.
//!
//! - **Nothing counts until bytes move.** At a (re)start the window keeps
//!   starting again until the first new byte, so the quiet while the sender
//!   re-reads what already arrived after a resume never counts (Mission 05
//!   found that it did: the estimator restarted before the re-read, and the
//!   first estimate after it read 514 minutes). Meanwhile, after a resume:
//!   "Checking what already arrived…".
//! - **Speed** over the last `WINDOW` of moving time, shown at most every
//!   2 s, rounded to 2 significant figures. Nothing moving for `STOPPED`:
//!   no speed at all (never a number draining away).
//! - **Time left**, in this priority:
//!   1. none until the speed has settled (the window spans `STEADY` and its
//!      two halves agree): "Working out time left…";
//!   2. a big correction (more than `BIG` off what's shown, held `BIG_HOLD`)
//!      shows at once, whatever else;
//!   3. small changes stay calm: no A → B → A within `NO_RETURN` (they
//!      follow a smoothed estimate, and the countdown holds rather than step
//!      back onto words left within the minute);
//!   4. updates come as often as the time scale allows: every few seconds
//!      for short times, at most every `DWELL_MAX` for long ones (`dwell`);
//!   5. a whole transfer under `SHORT` shows no time at all, then "A few
//!      seconds" near the end.
//!
//!   What's shown counts down by itself between updates, so the words move
//!   on as time passes ("About 6 minutes", then 5).

use std::collections::VecDeque;
use std::time::Duration;

use serde::Serialize;

/// The speed is measured over this much of the latest moving time.
pub const WINDOW: Duration = Duration::from_secs(6);
/// The first estimate waits for the window to be full…
const STEADY: Duration = WINDOW;
/// …with its two halves within this of each other (or twice the span).
const STEADY_WITHIN: f64 = 0.35;
const SPEED_EVERY: Duration = Duration::from_secs(2);
/// Nothing moved for this long: no speed is shown.
pub const STOPPED: Duration = Duration::from_secs(2);
/// A correction this far off what's shown (either way)… (the spec says
/// "~30 %"; 20 % keeps every shown time within its ±25 % accuracy, even
/// during the hold)
const BIG: f64 = 0.2;
/// …held this long, shows at once.
const BIG_HOLD: Duration = Duration::from_secs(3);
/// A value shown and left within this long isn't shown again by a small change.
const NO_RETURN: Duration = Duration::from_secs(61);
/// Small changes wait at least this long, and at most `DWELL_MAX`…
const DWELL_MIN: Duration = Duration::from_secs(3);
const DWELL_MAX: Duration = Duration::from_secs(20);
/// Smoothing for small changes, in seconds: a twentieth of the time left, within these.
const TAU: (f64, f64) = (4.0, 30.0);
/// A whole transfer shorter than this shows no time left…
const SHORT: f64 = 30.0;
/// …until this close to the end.
const FEW: f64 = 10.0;

/// Time left, in the words the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Eta {
    /// "Working out time left…"
    WorkingOut,
    /// "Checking what already arrived…" (a resume, before bytes move again)
    Checking,
    /// Nothing (a short transfer, until "A few seconds")
    Hidden,
    /// "About 6 minutes" (2 or more)
    Minutes { n: u32 },
    /// "About 2½ minutes" (2½ to 4½)
    MinutesAndAHalf { n: u32 },
    /// "About a minute and a half"
    MinuteAndAHalf,
    /// "About a minute"
    AboutAMinute,
    /// "About 40 seconds" (10 s steps, 10 to 100)
    Seconds { n: u32 },
    /// "A few seconds"
    FewSeconds,
}

impl Eta {
    /// The words for a number of seconds. Each band is narrow enough that
    /// the words alone are never more than 25 % or 15 s off.
    pub fn of(secs: f64) -> Eta {
        let tens = || Eta::Seconds { n: ((secs / 10.0).round() as u32 * 10).clamp(10, 100) };
        if secs >= 285.0 {
            Eta::Minutes { n: ((secs / 60.0).round() as u32).max(5) }
        } else if secs >= 105.0 {
            // Half minutes up to 5: whole ones would be up to 30 s off.
            // Leaning a little low: time left is judged against a smaller
            // number as it runs out, so the same seconds weigh more.
            let halves = ((secs / 30.0 - 0.15).round() as u32).clamp(4, 9);
            if halves.is_multiple_of(2) {
                Eta::Minutes { n: halves / 2 }
            } else {
                Eta::MinutesAndAHalf { n: halves / 2 }
            }
        } else if (85.0..95.0).contains(&secs) {
            Eta::MinuteAndAHalf
        } else if secs >= 65.0 {
            tens()
        } else if secs >= 55.0 {
            Eta::AboutAMinute
        } else if secs >= 12.0 {
            tens()
        } else {
            Eta::FewSeconds
        }
    }

    /// The number of seconds the words stand for (None: no time shown).
    pub fn secs(&self) -> Option<f64> {
        Some(match self {
            Eta::WorkingOut | Eta::Checking | Eta::Hidden => return None,
            Eta::Minutes { n } => *n as f64 * 60.0,
            Eta::MinutesAndAHalf { n } => *n as f64 * 60.0 + 30.0,
            Eta::MinuteAndAHalf => 90.0,
            Eta::AboutAMinute => 60.0,
            Eta::Seconds { n } => *n as f64,
            Eta::FewSeconds => 6.0,
        })
    }

    pub fn words(&self) -> String {
        match self {
            Eta::WorkingOut => "Working out time left…".into(),
            Eta::Checking => "Checking what already arrived…".into(),
            Eta::Hidden => String::new(),
            Eta::Minutes { n } => format!("About {n} minutes"),
            Eta::MinutesAndAHalf { n } => format!("About {n}½ minutes"),
            Eta::MinuteAndAHalf => "About a minute and a half".into(),
            Eta::AboutAMinute => "About a minute".into(),
            Eta::Seconds { n } => format!("About {n} seconds"),
            Eta::FewSeconds => "A few seconds".into(),
        }
    }
}

/// What to show now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shown {
    /// Bytes per second, 2 significant figures; None until there's a window,
    /// and while nothing moves.
    pub speed: Option<f64>,
    pub eta: Eta,
    /// The windowed speed itself (for Details and the log).
    pub rate: f64,
}

/// Rounds to 2 significant figures.
pub fn two_figures(v: f64) -> f64 {
    if v <= 0.0 || !v.is_finite() {
        return 0.0;
    }
    let mag = 10f64.powi(v.log10().floor() as i32 - 1);
    (v / mag).round() * mag
}

/// How long a small change waits, for a shown time of `secs`.
fn dwell(secs: f64) -> Duration {
    Duration::from_secs_f64(secs / 12.0).clamp(DWELL_MIN, DWELL_MAX)
}

/// Why the words last changed (the calm test lets only a big correction
/// take the words back to a value left within the minute).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    First,
    Big,
    Small,
    Countdown,
}

#[derive(Debug)]
pub struct Estimator {
    total: u64,
    /// After a resume: "Checking…" until bytes move.
    resumed: bool,
    /// (time, bytes), from just before the first new byte.
    samples: VecDeque<(Duration, u64)>,
    /// When bytes started moving (None: not yet since the (re)start), and
    /// how many were there then.
    moving: Option<Duration>,
    moved_from: u64,
    /// When the bytes last went up.
    last_move: Duration,
    steady: bool,
    speed: Option<(Duration, f64)>,
    /// What's shown: since when, and the seconds it stood for then (counts
    /// down from there). None: a word without a number (working out…).
    shown: (Duration, Option<f64>, Eta),
    /// A big correction waiting to hold: since when.
    big: Option<Duration>,
    /// Words shown before, and when they were left.
    left: Vec<(Eta, Duration)>,
    /// The words last returned (to notice the countdown moving them on).
    said: Eta,
    /// The estimate smoothed for small changes: (when, seconds), counting down.
    smooth: Option<(Duration, f64)>,
    /// Why the words last changed, and whether `show` ran in this sample.
    change: Change,
    shown_now: bool,
}

impl Estimator {
    /// Starts measuring at `at` with `bytes` already there.
    pub fn new(total: u64, at: Duration, bytes: u64) -> Self {
        Self {
            total,
            resumed: false,
            samples: VecDeque::from([(at, bytes)]),
            moving: None,
            moved_from: bytes,
            last_move: at,
            steady: false,
            speed: None,
            shown: (at, None, Eta::WorkingOut),
            big: None,
            left: Vec::new(),
            said: Eta::WorkingOut,
            smooth: None,
            change: Change::First,
            shown_now: false,
        }
    }

    /// Running again after a pause: start over from here, and say it's
    /// checking until bytes move.
    pub fn restart(&mut self, at: Duration, bytes: u64) {
        *self = Self::new(self.total, at, bytes);
        self.resumed = true;
        self.shown.2 = Eta::Checking;
    }

    fn rate_between(&self, from: Duration, to: Duration) -> Option<f64> {
        let at = |t: Duration| self.samples.iter().rev().find(|(s, _)| *s <= t).or(self.samples.front()).copied();
        let ((t0, b0), (t1, b1)) = (at(from)?, at(to)?);
        let dt = t1.checked_sub(t0)?.as_secs_f64();
        (dt >= 0.5).then(|| b1.saturating_sub(b0) as f64 / dt)
    }

    pub fn sample(&mut self, at: Duration, bytes: u64) -> Shown {
        let last = self.samples.back().map_or(0, |s| s.1);
        if self.moving.is_none() {
            if bytes <= last {
                // Nothing has moved since the (re)start: the window starts here.
                self.samples.clear();
                self.samples.push_back((at, bytes));
                self.last_move = at;
                return Shown { speed: None, eta: self.shown.2, rate: 0.0 };
            }
            self.moving = self.samples.front().map(|s| s.0);
            self.moved_from = self.samples.front().map_or(0, |s| s.1);
            if self.shown.2 == Eta::Checking {
                self.shown = (at, None, Eta::WorkingOut);
            }
        }
        if bytes > last {
            self.last_move = at;
        }
        self.samples.push_back((at, bytes));
        // Keep one sample at or just before the window's start.
        while self.samples.len() > 2 && at.saturating_sub(self.samples[1].0) >= WINDOW {
            self.samples.pop_front();
        }
        let (t0, _) = self.samples[0];
        let span = at.saturating_sub(t0);
        let rate = self.rate_between(t0, at).unwrap_or(0.0);
        let stopped = at.saturating_sub(self.last_move) >= STOPPED;

        // Speed: at most every 2 s, 2 significant figures; none while stopped.
        if stopped {
            self.speed = None;
        } else if span >= SPEED_EVERY && self.speed.is_none_or(|(t, _)| at.saturating_sub(t) >= SPEED_EVERY) {
            self.speed = Some((at, two_figures(rate)));
        }

        // Rule 1: nothing until the speed has settled.
        if !self.steady && span >= STEADY {
            let mid = t0 + span / 2;
            let agree = match (self.rate_between(t0, mid), self.rate_between(mid, at)) {
                (Some(a), Some(b)) if a > 0.0 && b > 0.0 => (a - b).abs() / a.max(b) <= STEADY_WITHIN,
                _ => false,
            };
            self.steady = agree || span >= STEADY * 2;
        }
        self.shown_now = false;
        if self.steady && !stopped && rate > 0.0 {
            let raw = self.total.saturating_sub(bytes) as f64 / rate;
            let smooth = match self.smooth {
                None => raw,
                Some((t, e)) => {
                    let dt = at.saturating_sub(t).as_secs_f64();
                    let predicted = (e - dt).max(0.0);
                    let tau = (predicted / 20.0).clamp(TAU.0, TAU.1);
                    predicted + (raw - predicted) * (dt / tau).min(1.0)
                }
            };
            self.smooth = Some((at, smooth));
            // The average since bytes started moving: slow to follow a real
            // change, but it doesn't wobble with the Wi-Fi.
            let moved = at.saturating_sub(self.moving.unwrap_or(at)).as_secs_f64();
            let average = (moved > 0.0).then(|| bytes.saturating_sub(self.moved_from) as f64 / moved);
            let steady = average.filter(|a| *a > 0.0).map_or(smooth, |a| self.total.saturating_sub(bytes) as f64 / a);
            self.consider(at, raw, smooth, steady);
        } else if let Some((t, e)) = self.smooth {
            self.smooth = Some((at, (e - at.saturating_sub(t).as_secs_f64()).max(0.0)));
        }
        let eta = self.eta_at(at);
        if eta != self.said {
            if self.said.secs().is_some() {
                self.left.push((self.said, at));
            }
            if !self.shown_now {
                // Back onto recent words only because holding was too far off.
                self.change = if self.returning(at, eta) { Change::Big } else { Change::Countdown };
            }
            self.said = eta;
        }
        Shown { speed: self.speed.map(|(_, s)| s), eta, rate }
    }

    /// The words now: what was shown, counted down since. The countdown
    /// never steps back onto words left within the last minute (it holds).
    fn eta_at(&self, at: Duration) -> Eta {
        match self.shown {
            (_, _, Eta::FewSeconds) => Eta::FewSeconds,
            (since, Some(secs), shown) => {
                let left = (secs - at.saturating_sub(since).as_secs_f64()).max(0.0);
                let words = Eta::of(left);
                let fresh = since == at;
                // Held only while the held words are still close.
                let close = self.said.secs().is_some_and(|h| (h - left).abs() <= (left * 0.15).max(10.0));
                if !fresh && words != self.said && close && self.returning(at, words) {
                    self.said
                } else if fresh {
                    shown
                } else {
                    words
                }
            }
            (_, None, eta) => eta,
        }
    }

    fn returning(&self, at: Duration, words: Eta) -> bool {
        self.left.iter().any(|(v, when)| *v == words && at.saturating_sub(*when) < NO_RETURN)
    }

    /// The number behind what's shown now (counted down), if any.
    fn shown_secs(&self, at: Duration) -> Option<f64> {
        let (since, secs, _) = self.shown;
        secs.map(|s| (s - at.saturating_sub(since).as_secs_f64()).max(0.0))
    }

    fn consider(&mut self, at: Duration, raw: f64, smooth: f64, average: f64) {
        let moving_for = self.moving.map_or(0.0, |m| at.saturating_sub(m).as_secs_f64());
        // Rule 5: a short transfer shows nothing, then "A few seconds".
        let short = self.shown.2 == Eta::Hidden || (self.shown.1.is_none() && moving_for + raw < SHORT);
        if short && raw > SHORT * 2.0 {
            // Much slower than it looked: not short after all.
        } else if short {
            let eta = if raw <= FEW { Eta::FewSeconds } else { Eta::Hidden };
            if eta != self.shown.2 {
                self.show(at, (eta == Eta::FewSeconds).then_some(raw), eta, Change::First);
            }
            return;
        }
        let Some(shown) = self.shown_secs(at) else {
            // The first estimate replaces "Working out" at once.
            self.smooth = Some((at, raw));
            return self.show(at, Some(raw), Eta::of(raw), Change::First);
        };
        // Rule 2: a big correction, held, shows at once. Its value is the
        // middle one of the window's, the smoothed and the average since it
        // started moving: a real change gets there within seconds (window
        // and smoothed), a few slow seconds of Wi-Fi don't (window alone).
        let mut three = [raw, smooth, average];
        three.sort_by(f64::total_cmp);
        let target = three[1];
        let off = (target - shown).abs();
        if off / shown.max(1.0) > BIG && off > 15.0 {
            let since = *self.big.get_or_insert(at);
            if at.saturating_sub(since) >= BIG_HOLD {
                self.smooth = Some((at, target));
                self.show(at, Some(target), Eta::of(target), Change::Big);
            }
            return;
        }
        self.big = None;
        // "A few seconds" stays to the end (unless a big correction).
        let now = self.eta_at(at);
        let candidate = Eta::of(smooth);
        if now == Eta::FewSeconds {
            return;
        }
        if candidate == now {
            // Same words: follow the better number quietly, so the countdown
            // moves on to the next words at the right moment.
            if let Some(n) = self.shown.1.as_mut() {
                *n = smooth + at.saturating_sub(self.shown.0).as_secs_f64();
            }
            return;
        }
        // Rules 3 and 4: small changes wait their turn and never go back.
        let dwelt = at.saturating_sub(self.shown.0) >= dwell(shown);
        if dwelt && !self.returning(at, candidate) {
            self.show(at, Some(smooth), candidate, Change::Small);
        }
    }

    fn show(&mut self, at: Duration, secs: Option<f64>, eta: Eta, change: Change) {
        self.left.retain(|(_, when)| at.saturating_sub(*when) < NO_RETURN);
        self.shown = (at, secs, eta);
        self.big = None;
        self.change = change;
        self.shown_now = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: f64 = 1e6;
    const TICK: Duration = Duration::from_millis(100);

    /// A tiny deterministic random source (no dependency).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
    }

    #[derive(Clone, Copy)]
    enum Event {
        /// Nothing moves from `at` for `secs`.
        Stall { at: f64, secs: f64 },
        /// Three times the speed from `at` for `secs`.
        Burst { at: f64, secs: f64 },
        /// The connection drops at `at` and is back `away` later (the app
        /// restarts its estimator then); the sender re-reads what arrived for
        /// `check` more seconds before bytes move again.
        Resume { at: f64, away: f64, check: f64 },
    }

    /// Everything shown over a whole transfer, tick by tick.
    struct Trace {
        /// (time, bytes, shown, why the words last changed)
        ticks: Vec<(f64, f64, Shown, Change)>,
        restarts: Vec<f64>,
        end: f64,
    }

    impl Trace {
        fn changes(&self) -> Vec<(f64, Eta, Change)> {
            let mut out: Vec<(f64, Eta, Change)> = Vec::new();
            for (t, _, s, c) in &self.ticks {
                if out.last().map(|c| c.1) != Some(s.eta) {
                    out.push((*t, s.eta, *c));
                }
            }
            out
        }
    }

    /// A transfer of `total` at `rate` with Wi-Fi swings of ±`swing`
    /// (a new speed each second), through `events`.
    fn run(total: f64, rate: f64, swing: f64, events: &[Event], seed: u64) -> Trace {
        let mut rng = Rng(seed);
        let mut est = Estimator::new(total as u64, Duration::ZERO, 0);
        let (mut t, mut bytes) = (0.0f64, 0.0f64);
        let mut factor = 1.0;
        let mut trace = Trace { ticks: vec![], restarts: vec![], end: 0.0 };
        while bytes < total {
            t += TICK.as_secs_f64();
            if ((t * 10.0).round() as u64).is_multiple_of(10) {
                factor = 1.0 + swing * (2.0 * rng.next() - 1.0);
            }
            let mut r = rate * factor;
            for e in events {
                match *e {
                    Event::Stall { at, secs } if t >= at && t < at + secs => r = 0.0,
                    Event::Burst { at, secs } if t >= at && t < at + secs => r *= 3.0,
                    Event::Resume { at, away, check } if t >= at && t < at + away + check => r = 0.0,
                    _ => {}
                }
            }
            for e in events {
                if let Event::Resume { at, away, .. } = *e
                    && (t - (at + away)).abs() < 0.05
                {
                    est.restart(Duration::from_secs_f64(t), bytes as u64);
                    trace.restarts.push(t);
                }
            }
            bytes = (bytes + r * TICK.as_secs_f64()).min(total);
            let shown = est.sample(Duration::from_secs_f64(t), bytes as u64);
            trace.ticks.push((t, bytes, shown, est.change));
        }
        trace.end = t;
        trace
    }

    /// A recorded trace (`tests/fixtures/traces`, from `tests/traces.rs`):
    /// restarts where the app restarts its estimator (paused → running).
    fn replay(name: &str) -> Trace {
        let path = format!("{}/tests/fixtures/traces/{name}.csv", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
        let rows: Vec<(f64, f64, char)> = text
            .lines()
            .map(|l| {
                let mut f = l.split(',');
                let ms: f64 = f.next().unwrap().parse().unwrap();
                let b: f64 = f.next().unwrap().parse().unwrap();
                (ms / 1000.0, b, f.next().unwrap().chars().next().unwrap())
            })
            .collect();
        let total = rows.iter().map(|r| r.1).fold(0.0, f64::max);
        let mut est = Estimator::new(total as u64, Duration::ZERO, 0);
        let mut trace = Trace { ticks: vec![], restarts: vec![], end: 0.0 };
        let mut was = 'r';
        for (t, b, phase) in rows {
            if phase == 'd' {
                trace.end = t;
                break;
            }
            if phase == 'p' {
                was = 'p';
                continue;
            }
            if was == 'p' {
                est.restart(Duration::from_secs_f64(t), b as u64);
                trace.restarts.push(t);
                was = 'r';
            }
            let shown = est.sample(Duration::from_secs_f64(t), b as u64);
            trace.ticks.push((t, b, shown, est.change));
            trace.end = t;
        }
        // The real finish: when the last byte landed.
        if let Some(last) = trace.ticks.iter().find(|x| x.1 >= total) {
            trace.end = last.0;
        }
        trace
    }

    /// From the first time shown on, the shown time is within ±25 % or
    /// ±15 s of the real finish. Judged from the last restart on: before a
    /// drop, the finish it later had couldn't be known.
    fn accurate(name: &str, trace: &Trace) {
        let from = trace.restarts.last().copied().unwrap_or(0.0);
        for (t, _, s, _) in trace.ticks.iter().filter(|x| x.0 >= from) {
            let Some(shown) = s.eta.secs() else { continue };
            // To the tick (0.1 s), so float noise can't decide it.
            let real = ((trace.end - t) * 10.0).round() / 10.0;
            let off = (shown - real).abs();
            assert!(
                off <= (real * 0.25).max(15.0),
                "{name}: at {t:.1} s showed {:?} ({shown:.0} s) with {real:.0} s really left",
                s.eta
            );
        }
    }

    /// Calm and honest, between restarts.
    fn calm(name: &str, trace: &Trace) {
        let changes = trace.changes();
        let mut cuts = vec![0.0];
        cuts.extend(trace.restarts.iter().copied());
        cuts.push(f64::MAX);
        for w in cuts.windows(2) {
            let seg: Vec<(f64, Eta, Change)> =
                changes.iter().copied().filter(|(t, _, _)| *t >= w[0] && *t < w[1]).collect();
            // Never A → B → A within 60 s, unless the way back was a big
            // correction (rule 2 overrides rule 3).
            for i in 2..seg.len() {
                let (a, b, c) = (seg[i - 2], seg[i - 1], seg[i]);
                assert!(
                    !(a.1 == c.1 && c.0 - b.0 < 60.0 && c.2 != Change::Big && a.1.secs().is_some()),
                    "{name}: {:?} → {:?} → {:?} ({:?}) at {:.0}–{:.0} s",
                    a.1,
                    b.1,
                    c.1,
                    c.2,
                    b.0,
                    c.0
                );
            }
        }
        // Reaches "A few seconds" (or nothing at all for a short one), and stays.
        if let Some(few) = changes.iter().position(|c| c.1 == Eta::FewSeconds) {
            assert!(changes[few..].iter().all(|c| c.1 == Eta::FewSeconds), "{name}: bounced after a few seconds");
        } else {
            panic!("{name}: never said a few seconds: {changes:?}");
        }
        // Speed: no more often than every 2 s, 2 significant figures.
        let mut speeds: Vec<(f64, f64)> = Vec::new();
        for (t, _, s, _) in &trace.ticks {
            if let Some(v) = s.speed
                && speeds.last().map(|x| x.1) != Some(v)
            {
                speeds.push((*t, v));
            }
        }
        for w in speeds.windows(2) {
            assert!(w[1].0 - w[0].0 >= 1.99, "{name}: speed changed after {:.1} s", w[1].0 - w[0].0);
        }
        for (_, v) in &speeds {
            assert_eq!(*v, two_figures(*v), "{name}: {v} has more than 2 figures");
        }
    }

    /// The share of ticks (from the first time shown on) within ±25 % or 15 s.
    fn share_accurate(trace: &Trace) -> (usize, usize) {
        let mut n = (0, 0);
        for (t, _, s, _) in &trace.ticks {
            let Some(shown) = s.eta.secs() else { continue };
            let real = ((trace.end - t) * 10.0).round() / 10.0;
            n.1 += 1;
            if (shown - real).abs() <= (real * 0.25).max(15.0) {
                n.0 += 1;
            }
        }
        n
    }

    #[test]
    fn wifi_swings_of_forty_percent_stay_calm() {
        // the owner's 10 GB over Wi-Fi: 30 minutes down to the end, the speed
        // jumping by up to 40 % every second with no memory.
        let mut n = (0, 0);
        for seed in 1..=20 {
            let trace = run(10e9, 5.5 * MB, 0.4, &[], seed);
            calm(&format!("swings ±40 %, seed {seed}"), &trace);
            let (a, b) = share_accurate(&trace);
            n = (n.0 + a, n.1 + b);
        }
        let share = n.0 as f64 / n.1 as f64;
        eprintln!("±40 % swings: {:.2} % of shown times within ±25 % or 15 s", share * 100.0);
        assert!(share >= 0.99, "only {:.2} % within", share * 100.0);
    }

    #[test]
    fn wifi_swings_of_thirty_percent_stay_calm_and_accurate() {
        for seed in 1..=20 {
            let trace = run(10e9, 5.5 * MB, 0.3, &[], seed);
            calm(&format!("swings ±30 %, seed {seed}"), &trace);
            accurate(&format!("swings ±30 %, seed {seed}"), &trace);
        }
    }

    #[test]
    fn a_stall_and_a_burst_stay_calm_and_accurate() {
        for seed in 1..=10 {
            let events = [Event::Stall { at: 120.0, secs: 12.0 }, Event::Burst { at: 300.0, secs: 6.0 }];
            let trace = run(2e9, 4.0 * MB, 0.4, &events, seed);
            calm(&format!("stall and burst, seed {seed}"), &trace);
        }
    }

    #[test]
    fn after_a_resume_it_checks_then_works_it_out_from_moving_bytes_only() {
        // the owner's: 10 GB at 21 MB/s, Wi-Fi off at 48 %, both apps quit and
        // reopened; the sender re-reads 4.8 GB (about a minute) first.
        for seed in 1..=10 {
            let events = [Event::Resume { at: 230.0, away: 20.0, check: 60.0 }];
            let trace = run(10e9, 21.0 * MB, 0.3, &events, seed);
            let name = format!("resume, seed {seed}");
            calm(&name, &trace);
            accurate(&name, &trace);
            let back = trace.restarts[0];
            // While it re-reads: "Checking what already arrived…", no speed.
            for (t, _, s, _) in trace.ticks.iter().filter(|x| x.0 > back && x.0 < back + 59.0) {
                assert_eq!(s.eta, Eta::Checking, "{name}: at {t:.1} s");
                assert_eq!(s.speed, None, "{name}: a speed while checking at {t:.1} s");
            }
            // Bytes move again at back + 60: an estimate within ~10 s.
            let first = trace.ticks.iter().find(|x| x.0 > back + 60.0 && x.2.eta.secs().is_some()).unwrap();
            assert!(
                first.0 - (back + 60.0) <= 10.0,
                "{name}: first estimate {:.1} s after bytes moved",
                first.0 - back - 60.0
            );
            // And the speed is the real one once shown (within 40 % of 21 MB/s).
            let speed = trace.ticks.iter().find(|x| x.0 > back + 60.0 && x.2.speed.is_some()).unwrap().2.speed.unwrap();
            assert!((speed - 21.0 * MB).abs() <= 0.4 * 21.0 * MB, "{name}: first speed after the re-read {speed}");
        }
    }

    #[test]
    fn nothing_moving_for_2_seconds_shows_no_speed() {
        let events = [Event::Stall { at: 60.0, secs: 20.0 }];
        let trace = run(2e9, 4.0 * MB, 0.0, &events, 1);
        for (t, _, s, _) in trace.ticks.iter().filter(|x| x.0 >= 62.1 && x.0 < 80.0) {
            assert_eq!(s.speed, None, "a draining speed at {t:.1} s");
        }
        assert!(trace.ticks.iter().any(|x| x.0 > 85.0 && x.2.speed.is_some()), "speed back once it moves");
    }

    #[test]
    fn short_transfers_show_no_minutes_then_a_few_seconds() {
        for seed in 1..=10 {
            // About 25 s in all.
            let trace = run(200.0 * MB, 8.0 * MB, 0.4, &[], seed);
            let name = format!("short, seed {seed}");
            calm(&name, &trace);
            for (t, _, s, _) in &trace.ticks {
                assert!(
                    matches!(s.eta, Eta::WorkingOut | Eta::Hidden | Eta::FewSeconds),
                    "{name}: showed {:?} at {t:.1} s",
                    s.eta
                );
            }
        }
    }

    #[test]
    fn updates_come_every_few_seconds_when_the_time_is_short() {
        // 90 s left at the first estimate: the words follow it down.
        let trace = run(400.0 * MB, 4.0 * MB, 0.1, &[], 3);
        let changes = trace.changes();
        let shown: Vec<_> = changes.iter().filter(|c| c.1.secs().is_some()).collect();
        assert!(shown.len() >= 4, "too few updates for a 100 s transfer: {changes:?}");
    }

    #[test]
    fn recorded_loopback_traces_stay_calm_and_accurate() {
        for name in ["steady-4mbs", "resume-4mbs", "short-3mbs"] {
            for side in ["sender", "receiver"] {
                let n = format!("{name}-{side}");
                let trace = replay(&n);
                calm(&n, &trace);
                accurate(&n, &trace);
                if name.starts_with("short") {
                    assert!(trace.ticks.iter().all(|x| !matches!(x.2.eta, Eta::Minutes { .. })), "{n}: minutes");
                }
            }
        }
    }

    #[test]
    fn words_and_figures() {
        assert_eq!(Eta::of(6.2 * 60.0).words(), "About 6 minutes");
        assert_eq!(Eta::of(125.0).words(), "About 2 minutes");
        assert_eq!(Eta::of(150.0).words(), "About 2½ minutes");
        assert_eq!(Eta::of(290.0).words(), "About 5 minutes");
        assert_eq!(Eta::of(90.0).words(), "About a minute and a half");
        assert_eq!(Eta::of(78.0).words(), "About 80 seconds");
        assert_eq!(Eta::of(101.0).words(), "About 100 seconds");
        assert_eq!(Eta::of(60.0).words(), "About a minute");
        assert_eq!(Eta::of(40.0).words(), "About 40 seconds");
        assert_eq!(Eta::of(19.0).words(), "About 20 seconds");
        assert_eq!(Eta::of(4.0).words(), "A few seconds");
        assert_eq!(two_figures(5_432_100.0), 5_400_000.0);
        assert_eq!(two_figures(118_400_000.0), 120_000_000.0);
        assert_eq!(two_figures(987.0), 990.0);
    }
}
