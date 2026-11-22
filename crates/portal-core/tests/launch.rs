//! Mission 04 §1.1: two paired computers opened at about the same time
//! connect by themselves within 5 s, without Try again.
//!
//! Two `examples/peer.rs` **processes** on loopback (tested between processes
//! on one machine; not on real hardware). The first starts, the second starts
//! 0, 3, 8 or 20 s later; the leader (smaller device id) starts first in one
//! run and second in the other. Variants:
//! - `stored`: discovery off, both remember the right address.
//! - `stale`: discovery off, the second one remembers an old address for the
//!   first, so only the first can dial.
//! - `late`: as `stale`, and the second one's listener comes up 3 s after it
//!   starts.
//! - `found`: as `stale`, with (loopback) announces on, so a sighting helps.
//! - `restart`: the second one only ever dials its (old) stored address, so
//!   the first must dial it; it connects, quits, and opens again 20 s later
//!   (within the 90 s the first remembers where it saw it).
//!
//! Heavy (about two minutes): `#[ignore]`, run on `main` CI and by hand:
//! `cargo test -p portal-core --test launch -- --ignored --nocapture`

mod common;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Both up for this long without connecting fails the run.
const TARGET: Duration = Duration::from_secs(5);
/// How long to watch past the target, to report how late it was.
const WATCH: Duration = Duration::from_secs(20);
const LATE_LISTENER: f64 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Variant {
    Stored,
    Stale,
    Late,
    Found,
    Restart,
}

struct Pair {
    /// (data dir, tcp port, udp port, name, id)
    nodes: Vec<(tempfile::TempDir, u16, u16, String, String)>,
}

/// The profile folder (`target/debug`): the nearest folder above this test's
/// executable whose parent is the target folder. Cargo used to put test
/// executables in `target/debug/deps`; newer Cargo puts them deeper, in
/// `target/debug/build/<package>/<hash>/out`.
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from);
    exe.ancestors()
        .find(|p| match (&target, p.parent()) {
            (Some(t), Some(parent)) => parent == t.as_path(),
            (None, Some(parent)) => parent.file_name().is_some_and(|n| n == "target"),
            _ => false,
        })
        .map(Path::to_path_buf)
        .unwrap_or_else(|| exe.parent().unwrap().parent().unwrap().to_path_buf())
}

/// `examples/peer.rs`, built once if it isn't there yet.
fn peer_binary() -> PathBuf {
    static BIN: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        let bin = profile_dir().join("examples").join(format!("peer{}", std::env::consts::EXE_SUFFIX));
        if !bin.exists() {
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
            let ok = Command::new(cargo)
                .args(["build", "-p", "portal-core", "--example", "peer"])
                .status()
                .expect("cargo build --example peer");
            assert!(ok.success(), "building examples/peer.rs failed");
        }
        assert!(bin.exists(), "examples/peer.rs isn't at {}", bin.display());
        bin
    })
    .clone()
}

/// Two data folders, paired, with fixed ports, the leader first.
async fn paired_pair(tag: &str) -> Pair {
    let mut nodes = Vec::new();
    let mut started = Vec::new();
    for name in [format!("{tag} one"), format!("{tag} two")] {
        let (tcp, udp) = (common::free_tcp_port(), common::free_udp_port());
        let dir = tempfile::tempdir().unwrap();
        let node = common::start_in(&name, dir, |c| {
            c.port = tcp;
            c.udp_port = udp;
        })
        .await;
        started.push((node, tcp, udp, name));
    }
    let (mut a, mut b) = (started.remove(0), started.remove(0));
    common::pair(&mut a.0, &mut b.0).await;
    for (node, tcp, udp, name) in [a, b] {
        let id = node.id().await.to_string();
        node.core.shutdown().await;
        nodes.push((node.dir, tcp, udp, name, id));
    }
    nodes.sort_by(|x, y| x.4.cmp(&y.4));
    Pair { nodes }
}

/// Points one node's stored address for its peer at a closed port.
fn make_stale(dir: &Path) {
    common::set_known_addrs(dir, &[format!("127.0.0.1:{}", common::free_tcp_port())]);
}

struct Running {
    child: Child,
    lines: mpsc::Receiver<(Instant, String)>,
    log: Vec<String>,
    online_at: Option<Instant>,
}

impl Running {
    fn poll(&mut self) {
        while let Ok((at, line)) = self.lines.try_recv() {
            if line.starts_with("ONLINE ") && self.online_at.is_none() {
                self.online_at = Some(at);
            }
            self.log.push(line);
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn(
    bin: &Path,
    node: &(tempfile::TempDir, u16, u16, String, String),
    other_udp: u16,
    v: Variant,
    late: bool,
) -> Running {
    let mut cmd = Command::new(bin);
    cmd.args(["--loopback", "--name", &node.3, "--data"])
        .arg(node.0.path())
        .args(["--port", &node.1.to_string(), "--udp-port", &node.2.to_string(), "--no-mdns"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if matches!(v, Variant::Found | Variant::Restart) {
        cmd.args(["--announce-to", &format!("127.0.0.1:{other_udp}")]);
    } else {
        cmd.arg("--no-broadcast");
    }
    if late {
        cmd.args(["--listen-after", &LATE_LISTENER.to_string()]);
    }
    if v == Variant::Restart {
        cmd.arg("--blind");
    }
    let mut child = cmd.spawn().expect("start examples/peer.rs");
    let out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if tx.send((Instant::now(), line)).is_err() {
                return;
            }
        }
    });
    Running { child, lines: rx, log: Vec::new(), online_at: None }
}

/// Starts the two, `offset` apart. Returns how long after both were up they
/// were both connected (None: not within `TARGET + WATCH`), and the logs.
async fn run(v: Variant, leader_first: bool, offset: Duration) -> (Option<Duration>, Vec<String>) {
    let bin = peer_binary();
    let tag = format!("{v:?}-{}-{}", if leader_first { "L" } else { "F" }, offset.as_secs());
    let pair = paired_pair(&tag).await;
    let (first, second) = if leader_first { (0, 1) } else { (1, 0) };
    if v != Variant::Stored {
        make_stale(pair.nodes[second].0.path());
    }
    let mut a = spawn(&bin, &pair.nodes[first], pair.nodes[second].2, v, false);
    tokio::time::sleep(offset).await;
    let late = v == Variant::Late;
    let mut b = spawn(&bin, &pair.nodes[second], pair.nodes[first].2, v, late);
    if v == Variant::Restart {
        // Connected once, then quit; opened again 20 s later, still only
        // dialling its old stored address.
        let until = Instant::now() + TARGET + WATCH;
        while (a.online_at.is_none() || b.online_at.is_none()) && Instant::now() < until {
            a.poll();
            b.poll();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            b.online_at.is_some(),
            "{tag}: didn't connect the first time:\n{}",
            [a.log.clone(), b.log.clone()].concat().join("\n")
        );
        drop(b);
        make_stale(pair.nodes[second].0.path());
        tokio::time::sleep(Duration::from_secs(20)).await;
        a.poll();
        a.online_at = None;
        a.log.push("--- the second one opens again ---".into());
        b = spawn(&bin, &pair.nodes[second], pair.nodes[first].2, v, false);
    }
    let up = Instant::now() + if late { Duration::from_secs_f64(LATE_LISTENER) } else { Duration::ZERO };
    let deadline = up + TARGET + WATCH;
    loop {
        a.poll();
        b.poll();
        if let (Some(x), Some(y)) = (a.online_at, b.online_at) {
            let both = x.max(y);
            let took = both.saturating_duration_since(up);
            let logs = [a.log.clone(), b.log.clone()].concat();
            return (Some(took), logs);
        }
        if Instant::now() > deadline {
            let mut logs = vec![format!("--- {tag} first ---")];
            logs.extend(a.log.clone());
            logs.push(format!("--- {tag} second ---"));
            logs.extend(b.log.clone());
            return (None, logs);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "two processes, about two minutes: run on main CI and by hand"]
async fn two_apps_opened_together_connect_within_five_seconds() {
    let offsets = [0u64, 3, 8, 20];
    let variants = [Variant::Stored, Variant::Stale, Variant::Late, Variant::Found, Variant::Restart];
    let mut cases = Vec::new();
    for v in variants {
        for leader_first in [true, false] {
            // A restart's timing is set by the 20 s it stays closed.
            let offsets: &[u64] = if v == Variant::Restart { &[3] } else { &offsets };
            for o in offsets {
                cases.push((v, leader_first, Duration::from_secs(*o)));
            }
        }
    }
    let mut set = tokio::task::JoinSet::new();
    for (v, leader_first, offset) in cases {
        set.spawn(async move { ((v, leader_first, offset), run(v, leader_first, offset).await) });
    }
    let mut failures = Vec::new();
    let mut table = Vec::new();
    while let Some(done) = set.join_next().await {
        let ((v, leader_first, offset), (took, logs)) = done.unwrap();
        let who = if leader_first { "leader first" } else { "follower first" };
        let said = took.map(|t| format!("{:.1} s", t.as_secs_f64())).unwrap_or_else(|| "never".into());
        table.push(format!("{v:?}, {who}, second starts after {} s: connected {said}", offset.as_secs()));
        if took.is_none_or(|t| t > TARGET) {
            failures.push(format!("{v:?}, {who}, offset {} s: {said}\n{}", offset.as_secs(), logs.join("\n")));
        }
    }
    table.sort();
    println!("{}", table.join("\n"));
    assert!(
        failures.is_empty(),
        "{} of {} runs didn't connect within 5 s:\n\n{}",
        failures.len(),
        table.len(),
        failures.join("\n\n")
    );
}
