//! Noticing that the network changed under us (a new Wi-Fi network, a new
//! address from DHCP, waking from sleep) and recovering on our own: look
//! again, re-dial at once, and drop links that went dead instead of waiting
//! for them to time out.

use std::net::Ipv4Addr;
use std::time::{Duration, Instant, SystemTime};

use super::Node;
use crate::events::Tone;
use crate::net::iface::{Iface, IfaceKind};
use crate::net::link::Link;
use crate::proto::Msg;

/// After a change, a link that hasn't answered a ping by now is dead.
pub(super) const PROBE_TIMEOUT: Duration = Duration::from_secs(6);
/// More wall-clock time than monotonic time passed between two ticks: the
/// computer slept (macOS's monotonic clock stops while asleep).
const SLEEP_GAP: Duration = Duration::from_secs(5);
/// After the person asks to look (Pair another computer, Try again), sweep
/// for this long even while paired devices are connected.
const ASKED_FOR: Duration = Duration::from_secs(60);

/// Keep sweeping the network (Mission 05 §1.8): with nothing paired always,
/// on the usual backoff, because the list should show every computer and one
/// found by broadcast says nothing about the others (Sam's Air saw the PC,
/// so it stopped asking, and never found macbook one); when paired, while
/// none of them is connected, or for a minute after the person asked to look.
pub(super) fn wants_sweep(nothing_paired: bool, all_paired_away: bool, asked_recently: bool) -> bool {
    nothing_paired || all_paired_away || asked_recently
}

/// Alone this long (nothing found, or no paired device reachable): sweep
/// the local network by unicast (see `net::discovery`).
const SWEEP_FIRST: Duration = Duration::from_secs(2);

/// Time between sweeps after `done` of them: sweeps at 2, 6, 12 and 20 s
/// while connecting matters most, then 30 s, 1, 2, 4, then every 5 min.
pub(super) fn sweep_gap(done: u32) -> Duration {
    match done {
        0 | 1 => Duration::from_secs(4),
        2 => Duration::from_secs(6),
        3 => Duration::from_secs(8),
        n => Duration::from_secs(30 << (n - 4).min(4)).min(Duration::from_secs(300)),
    }
}

/// The addresses that decide who we can reach: IPv4 on Wi-Fi and wired
/// interfaces. (IPv6 privacy addresses rotate on their own; that's no change.)
pub(super) fn network_identity(ifaces: &[Iface]) -> Vec<(IfaceKind, Ipv4Addr)> {
    let mut out: Vec<_> = ifaces
        .iter()
        .filter(|i| matches!(i.kind, IfaceKind::Wifi | IfaceKind::Ethernet | IfaceKind::Bridge))
        .flat_map(|i| i.v4.iter().map(move |(a, _)| (i.kind, *a)))
        .collect();
    out.sort();
    out
}

/// "Wi-Fi 192.168.1.23", "no network"
fn describe(identity: &[(IfaceKind, Ipv4Addr)]) -> String {
    if identity.is_empty() {
        return "no network".into();
    }
    let parts: Vec<String> = identity
        .iter()
        .map(|(kind, ip)| {
            let word = if *kind == IfaceKind::Wifi { "Wi-Fi" } else { "Ethernet" };
            format!("{word} {ip}")
        })
        .collect();
    parts.join(", ")
}

/// How long the computer slept between two ticks, if it did.
pub(super) fn slept(before: (SystemTime, Instant), now: (SystemTime, Instant)) -> Option<Duration> {
    let wall = now.0.duration_since(before.0).unwrap_or_default();
    let mono = now.1.duration_since(before.1);
    wall.checked_sub(mono).filter(|gap| *gap >= SLEEP_GAP)
}

impl Node {
    /// The interface list changed. Only a change in the addresses we use
    /// counts as moving networks.
    pub(super) fn on_ifaces(&mut self, list: Vec<Iface>) {
        if *self.ifaces == list {
            return;
        }
        let before = network_identity(&self.ifaces);
        let after = network_identity(&list);
        self.ifaces = std::sync::Arc::new(list);
        self.dirty = true;
        if before != after {
            self.on_network_change(
                "The network changed",
                Some(format!("{} → {}", describe(&before), describe(&after))),
            );
        } else if let Some(d) = &self.discovery {
            d.announce_now();
        }
    }

    /// Something moved: look again, re-dial without waiting, and check every
    /// link still works (a link over the old network is dead but doesn't
    /// know it yet).
    pub(super) fn on_network_change(&mut self, what: &str, detail: Option<String>) {
        self.say("net", what, detail, Some(Tone::Dim));
        self.reach.start_all(Instant::now());
        self.upgrade_tried.clear();
        self.facts.known_silent = false;
        self.restart_sweeps();
        if let Some(d) = &self.discovery {
            d.announce_now();
        }
        let now = Instant::now();
        for link in self.links.values() {
            link.try_send(Msg::Ping);
            self.probing.insert(link.id(), now);
        }
        self.dirty = true;
    }

    /// Sweep again soon if we're (still) alone.
    pub(super) fn restart_sweeps(&mut self) {
        self.sweep.last = None;
        self.sweep.done = 0;
        self.sweep.alone_since = None;
    }

    /// Whether to keep sweeping (see `wants_sweep`). A device disconnected
    /// here isn't being looked for.
    fn alone(&self) -> bool {
        let paired: Vec<_> = self.store.peers().iter().filter(|p| p.trusted).collect();
        let wanted: Vec<_> = paired.iter().filter(|p| !p.disconnected).collect();
        let all_away = !wanted.is_empty() && wanted.iter().all(|t| self.link_for(&t.id).is_none());
        let asked = self.sweep.asked.is_some_and(|a| a.elapsed() < ASKED_FOR);
        wants_sweep(paired.is_empty(), all_away, asked)
    }

    /// The last resort when announces don't get through: ask every address.
    pub(super) fn maybe_sweep(&mut self) {
        if self.cfg.loopback_only || !self.cfg.broadcast {
            return;
        }
        if !self.alone() {
            self.restart_sweeps();
            return;
        }
        let since = *self.sweep.alone_since.get_or_insert_with(Instant::now);
        let due = match self.sweep.last {
            None => since.elapsed() >= SWEEP_FIRST,
            Some(last) => last.elapsed() >= sweep_gap(self.sweep.done),
        };
        if !due {
            return;
        }
        self.sweep.last = Some(Instant::now());
        self.sweep.done += 1;
        if self.sweep.done == 1 {
            self.say("search", "Asking every address on this network directly", None, Some(Tone::Dim));
        }
        if let Some(d) = &self.discovery {
            d.sweep();
        }
    }

    /// Every tick: did we just wake up, and did probed links answer?
    pub(super) fn watch_network(&mut self) {
        let now = (SystemTime::now(), Instant::now());
        if let Some(gap) = slept(self.last_tick, now) {
            self.on_network_change("Woke from sleep", Some(format!("after about {} min", (gap.as_secs() + 30) / 60)));
        }
        self.last_tick = now;

        let mut dead: Vec<Link> = Vec::new();
        self.probing.retain(|id, since| {
            let Some(link) = self.links.get(id) else { return false };
            if link.heard_since(*since) {
                return false;
            }
            if since.elapsed() >= PROBE_TIMEOUT {
                dead.push(link.clone());
                return false;
            }
            true
        });
        for link in dead {
            self.say(
                "link",
                format!("{} hasn’t answered since the network changed; reconnecting", link.info().hello.name),
                None,
                None,
            );
            link.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_nothing_paired_it_keeps_sweeping_after_finding_one_computer() {
        // Before Mission 05 "alone" with nothing paired meant "nothing seen
        // at all", so the first computer found stopped the sweeps.
        assert!(wants_sweep(true, false, false));
        assert!(!wants_sweep(false, false, false), "paired and connected: no sweeping");
        assert!(wants_sweep(false, true, false), "every paired device away");
        assert!(wants_sweep(false, false, true), "asked to look for another computer");
    }

    fn iface(kind: IfaceKind, v4: &[&str], v6: &[&str]) -> Iface {
        Iface {
            index: 4,
            name: "en0".into(),
            label: "Wi-Fi".into(),
            kind,
            v4: v4.iter().map(|a| (a.parse().unwrap(), 24)).collect(),
            v6: v6.iter().map(|a| a.parse().unwrap()).collect(),
            has_gateway: true,
            is_default: true,
        }
    }

    #[test]
    fn a_new_ipv4_address_is_a_network_change_and_a_new_ipv6_one_is_not() {
        let home = network_identity(&[iface(IfaceKind::Wifi, &["192.168.1.23"], &["2a02::1"])]);
        let rotated = network_identity(&[iface(IfaceKind::Wifi, &["192.168.1.23"], &["2a02::2"])]);
        let moved = network_identity(&[iface(IfaceKind::Wifi, &["192.168.1.40"], &["2a02::1"])]);
        let vpn = network_identity(&[
            iface(IfaceKind::Wifi, &["192.168.1.23"], &[]),
            iface(IfaceKind::Vpn, &["10.8.0.2"], &[]),
        ]);
        assert_eq!(home, rotated);
        assert_ne!(home, moved);
        assert_eq!(home, vpn, "a VPN coming up doesn't move us");
        assert_eq!(describe(&home), "Wi-Fi 192.168.1.23");
        assert_eq!(describe(&[]), "no network");
    }

    #[test]
    fn sweeps_come_quickly_at_first_then_thin_out() {
        let gaps: Vec<u64> = (1..=10).map(|n| sweep_gap(n).as_secs()).collect();
        assert_eq!(gaps, [4, 6, 8, 30, 60, 120, 240, 300, 300, 300]);
        let mut at = SWEEP_FIRST.as_secs();
        let times: Vec<u64> = (1..=4)
            .map(|n| {
                let t = at;
                at += sweep_gap(n).as_secs();
                t
            })
            .collect();
        assert_eq!(times, [2, 6, 12, 20]);
    }

    #[test]
    fn sleeping_shows_as_wall_time_passing_without_monotonic_time() {
        let (wall, mono) = (SystemTime::now(), Instant::now());
        let later = mono + Duration::from_secs(1);
        assert_eq!(slept((wall, mono), (wall + Duration::from_secs(1), later)), None, "an ordinary tick");
        assert_eq!(
            slept((wall, mono), (wall + Duration::from_secs(601), later)),
            Some(Duration::from_secs(600)),
            "ten minutes asleep"
        );
        assert_eq!(slept((wall, mono), (wall - Duration::from_secs(60), later)), None, "clock set back");
    }
}
