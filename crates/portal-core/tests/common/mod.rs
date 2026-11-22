//! Helpers for tests that run two cores on one machine.
#![allow(dead_code)]

pub mod relay;

use std::path::Path;
use std::time::Duration;

use portal_core::events::{Direction, Event, LogLine, PairStep, Snapshot, TransferPhase, TransferView};
use portal_core::model::DeviceId;
use portal_core::node::{Config, Core, PairAction};
use tokio::sync::mpsc;

pub struct TestNode {
    pub core: Core,
    pub events: mpsc::UnboundedReceiver<Event>,
    pub dir: tempfile::TempDir,
    pub port: u16,
    pub name: String,
    /// Log lines seen while waiting (so `drain_logs` still has them).
    seen: Vec<LogLine>,
}

pub fn free_udp_port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

pub fn free_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// Rewrites every paired device's last-known addresses in a stopped core's data folder.
pub fn set_known_addrs(dir: &Path, addrs: &[String]) {
    let path = dir.join("peers.json");
    let mut peers: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for p in peers.as_array_mut().unwrap() {
        p["addrs"] = serde_json::json!(addrs);
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&peers).unwrap()).unwrap();
}

pub async fn start(name: &str, tweak: impl FnOnce(&mut Config)) -> TestNode {
    let dir = tempfile::tempdir().unwrap();
    start_in(name, dir, tweak).await
}

pub async fn start_in(name: &str, dir: tempfile::TempDir, tweak: impl FnOnce(&mut Config)) -> TestNode {
    let mut cfg = Config::for_tests(dir.path());
    cfg.device_name = Some(name.to_owned());
    cfg.receive_dir = Some(dir.path().join("Received"));
    tweak(&mut cfg);
    let (core, events) = Core::start(cfg).await.unwrap();
    let port = core.snapshot().await.unwrap().me.port;
    TestNode { core, events, dir, port, name: name.to_owned(), seen: Vec::new() }
}

impl TestNode {
    pub fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    /// Waits (up to 10 s) until a snapshot satisfies `pred`, returning it.
    pub async fn wait_for(&mut self, what: &str, pred: impl Fn(&Snapshot) -> bool) -> Snapshot {
        self.wait_for_within(Duration::from_secs(10), what, pred).await
    }

    /// Like `wait_for`, with a custom deadline. Checks on every event and at
    /// least every 100 ms (progress changes don't send a snapshot).
    pub async fn wait_for_within(
        &mut self,
        deadline: Duration,
        what: &str,
        pred: impl Fn(&Snapshot) -> bool,
    ) -> Snapshot {
        tokio::time::timeout(deadline, async {
            loop {
                let current = self.core.snapshot().await.expect("core running");
                if pred(&current) {
                    return current;
                }
                match tokio::time::timeout(Duration::from_millis(100), self.events.recv()).await {
                    Ok(None) => panic!("core stopped while waiting for {what}"),
                    Ok(Some(Event::Log(l))) => self.seen.push(l),
                    _ => {}
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{}: timed out waiting for {what}", self.name))
    }

    /// Like `drain_logs`, also returning the other events `keep` picks.
    pub fn drain_logs_keeping(&mut self, keep: impl Fn(&Event) -> bool) -> (Vec<String>, Vec<Event>) {
        let mut out: Vec<String> = std::mem::take(&mut self.seen).iter().map(line).collect();
        let mut kept = Vec::new();
        while let Ok(e) = self.events.try_recv() {
            match e {
                Event::Log(l) => out.push(line(&l)),
                other if keep(&other) => kept.push(other),
                _ => {}
            }
        }
        (out, kept)
    }

    /// All log lines seen so far, as "tag text (detail)" (drains pending events).
    #[allow(dead_code)]
    pub fn drain_logs(&mut self) -> Vec<String> {
        self.drain_lines().iter().map(line).collect()
    }

    /// All log lines seen so far, as the core sent them (drains pending events).
    pub fn drain_lines(&mut self) -> Vec<LogLine> {
        let mut out = std::mem::take(&mut self.seen);
        while let Ok(e) = self.events.try_recv() {
            if let Event::Log(l) = e {
                out.push(l);
            }
        }
        out
    }
}

/// "tag text (detail)", as the technical log writes it
fn line(l: &LogLine) -> String {
    match &l.detail {
        Some(d) => format!("{} {} ({d})", l.tag, l.text),
        None => format!("{} {}", l.tag, l.text),
    }
}

/// Most tests have one transfer at a time: the newest one on screen.
pub trait Transfers {
    fn transfer(&self) -> Option<&TransferView>;
}

impl Transfers for Snapshot {
    fn transfer(&self) -> Option<&TransferView> {
        self.transfers.last()
    }
}

impl TestNode {
    /// The newest transfer that matches, by id (panics if none).
    pub async fn tid_where(&self, what: &str, f: impl Fn(&TransferView) -> bool) -> portal_core::proto::TransferId {
        let s = self.core.snapshot().await.expect("core running");
        s.transfers.iter().rev().find(|t| f(t)).unwrap_or_else(|| panic!("{}: no {what} transfer", self.name)).tid
    }

    /// Accepts the waiting offer.
    pub async fn accept(&self, always: bool) {
        let tid =
            self.tid_where("offered", |t| t.direction == Direction::In && t.phase == TransferPhase::Offered).await;
        self.core.accept(tid, always).await;
    }

    pub async fn decline(&self) {
        let tid =
            self.tid_where("offered", |t| t.direction == Direction::In && t.phase == TransferPhase::Offered).await;
        self.core.decline(tid).await;
    }

    /// Cancels the newest transfer that's still going.
    pub async fn cancel(&self) {
        let tid = self
            .tid_where("active", |t| {
                matches!(
                    t.phase,
                    TransferPhase::Preparing | TransferPhase::Offered | TransferPhase::Running | TransferPhase::Paused
                )
            })
            .await;
        self.core.cancel(tid).await;
    }

    /// Clears the newest finished transfer.
    pub async fn dismiss(&self) {
        let tid = self
            .tid_where("finished", |t| {
                !matches!(
                    t.phase,
                    TransferPhase::Preparing | TransferPhase::Offered | TransferPhase::Running | TransferPhase::Paused
                )
            })
            .await;
        self.core.dismiss(tid).await;
    }

    pub async fn send_again(&self) -> Result<(), String> {
        let tid = self.tid_where("stopped send", |t| t.can_send_again).await;
        self.core.send_again(tid).await
    }

    pub async fn id(&self) -> DeviceId {
        self.core.snapshot().await.unwrap().me.id
    }

    pub fn received(&self) -> std::path::PathBuf {
        self.dir.path().join("Received")
    }
}

pub fn paired_online(s: &Snapshot) -> bool {
    s.peers.iter().any(|p| p.paired && p.online)
}

/// `b` dials `a` by IP and they pair: `b` asks, `a` shows the code, it's
/// typed on `b` (Mission 04 §2).
pub async fn pair(a: &mut TestNode, b: &mut TestNode) {
    let addr = a.addr();
    pair_via(a, b, &addr).await;
}

/// Where a pairing is, if there is one.
#[allow(dead_code)]
pub fn pair_step(s: &Snapshot) -> Option<PairStep> {
    s.pairing.as_ref().filter(|p| p.failed.is_none()).map(|p| p.step)
}

/// The code on the showing computer's screen.
#[allow(dead_code)]
pub fn shown_code(s: &Snapshot) -> Option<String> {
    s.pairing.as_ref().and_then(|p| p.code.clone())
}

/// `b` asks `a` to pair and `a` shows its code; the code.
#[allow(dead_code)]
pub async fn ask_and_show(a: &mut TestNode, b: &mut TestNode) -> String {
    b.core.pair(&a.id().await).await.unwrap();
    a.wait_for("asked", |s| pair_step(s) == Some(PairStep::Asked)).await;
    a.core.pairing(PairAction::ShowCode).await.unwrap();
    let s = a.wait_for("code shown", |s| shown_code(s).is_some()).await;
    b.wait_for("type the code", |s| pair_step(s) == Some(PairStep::Code)).await;
    shown_code(&s).unwrap()
}

/// Like `pair`, with `b` dialling `a` at `addr` (e.g. a relay in front of it).
pub async fn pair_via(a: &mut TestNode, b: &mut TestNode, addr: &str) {
    let (ida, idb) = (a.id().await, b.id().await);
    b.core.connect(addr).await.unwrap();
    let code = ask_and_show(a, b).await;
    b.core.pairing(PairAction::Code(code)).await.unwrap();
    // That device in particular (either may be paired with others already)
    b.wait_for("paired", |s| s.peers.iter().any(|p| p.id == ida && p.paired && p.online)).await;
    a.wait_for("paired", |s| s.peers.iter().any(|p| p.id == idb && p.paired && p.online)).await;
}

fn names_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|r| r.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default()
}

/// Nothing partial in the receive folder: at most an empty staging folder.
/// Cleanup runs in the receiver's own task, so allow it a moment.
pub async fn assert_nothing_partial(receive: &Path) {
    let staging = receive.join(".seamdrop-incoming");
    for _ in 0..50 {
        if names_in(receive).iter().all(|n| n == ".seamdrop-incoming") && names_in(&staging).is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("partial files left behind: {:?} / {:?}", names_in(receive), names_in(&staging));
}
