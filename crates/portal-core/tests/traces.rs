//! Records progress traces from real transfers between two cores through a
//! throttled relay, for the time-left accuracy test in `transfer/estimate.rs`
//! (Mission 05 §1.3). Not part of the normal run:
//!
//!     cargo test -p portal-core --test traces -- --ignored --nocapture
//!
//! Writes `tests/fixtures/traces/<name>-<side>.csv`: one `ms,bytes,phase` line
//! every 100 ms from the offer being accepted to the end, where bytes is what
//! the window's numbers are made from on that side (confirmed by the receiver
//! on the sender; landed on the receiver) and phase is `r` (running), `p`
//! (paused) or `d` (done). The app restarts its estimator at each `p` → `r`.
mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::relay::{Relay, Shape};
use common::{TestNode, Transfers, free_tcp_port, pair_via, set_known_addrs, start, start_in};
use portal_core::Config;
use portal_core::events::{Snapshot, TransferPhase};

fn phase(s: &Snapshot) -> Option<TransferPhase> {
    s.transfer().map(|t| t.phase)
}

fn write_file(path: &Path, len: usize, seed: u8) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut x = seed as u32 | 1;
    let data: Vec<u8> = (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect();
    std::fs::write(path, data).unwrap();
}

fn pc_only_dials(c: &mut Config) {
    c.port = free_tcp_port();
    c.fault_listen_after = Some(Duration::from_secs(3600));
}

/// Starts the PC, pairs it with the Mac through the relay, then leaves it
/// knowing only the relay's address for the Mac (pairing also stores the
/// Mac's own address, which on one machine is reachable around the relay).
async fn pc_through(mac: &mut TestNode, relay: &Relay, tweak: impl Fn(&mut Config)) -> TestNode {
    let mut pc = start("Desk PC", &tweak).await;
    pair_via(mac, &mut pc, &relay.addr.to_string()).await;
    pc.core.shutdown().await;
    set_known_addrs(pc.dir.path(), &[relay.addr.to_string()]);
    let mut pc = start_in("Desk PC", pc.dir, &tweak).await;
    pc.wait_for("back through the relay", common::paired_online).await;
    pc
}

fn out_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/traces")
}

struct Recorder {
    began: Instant,
    rows: [Vec<(u64, u64, char)>; 2],
}

impl Recorder {
    async fn sample(&mut self, mac: &TestNode, pc: &TestNode) -> (Snapshot, Snapshot) {
        let (m, p) = (mac.core.snapshot().await.unwrap(), pc.core.snapshot().await.unwrap());
        let ms = self.began.elapsed().as_millis() as u64;
        let row = |s: &Snapshot| {
            let c = match phase(s) {
                Some(TransferPhase::Paused) => 'p',
                Some(TransferPhase::Done) => 'd',
                _ => 'r',
            };
            (ms, s.transfer().map_or(0, |t| t.bytes), c)
        };
        self.rows[0].push(row(&p));
        self.rows[1].push(row(&m));
        (m, p)
    }

    fn save(&self, name: &str) {
        std::fs::create_dir_all(out_dir()).unwrap();
        for (side, rows) in ["sender", "receiver"].iter().zip(&self.rows) {
            let text: String = rows.iter().map(|(ms, b, c)| format!("{ms},{b},{c}\n")).collect();
            std::fs::write(out_dir().join(format!("{name}-{side}.csv")), text).unwrap();
        }
    }
}

/// Sends `len` bytes at `rate` from the PC to the Mac, recording both sides.
/// With `drop_at` (a fraction), the link goes dead there for `away`, and the
/// sender re-reads what already arrived at `hash_rate` before continuing.
async fn record(name: &str, len: usize, rate: u64, drop_at: Option<(f64, Duration)>, hash_rate: Option<u64>) {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let relay = Relay::start(mac.addr().parse().unwrap(), Shape { delay: Duration::from_millis(8), rate }).await;
    let pc = pc_through(&mut mac, &relay, |c| {
        pc_only_dials(c);
        c.fault_hash_rate = hash_rate;
    })
    .await;

    let file = pc.dir.path().join(format!("src/{name}.bin"));
    write_file(&file, len, 3);
    pc.core.send(&mac.id().await, vec![file]).await.unwrap();
    mac.wait_for("offer", |s| phase(s) == Some(TransferPhase::Offered)).await;
    mac.accept(false).await;

    let mut rec = Recorder { began: Instant::now(), rows: [Vec::new(), Vec::new()] };
    let mut dropped: Option<Instant> = None;
    let mut back = false;
    loop {
        let (m, p) = rec.sample(&mac, &pc).await;
        let done = |s: &Snapshot| matches!(phase(s), Some(TransferPhase::Done | TransferPhase::Failed));
        if done(&m) && done(&p) {
            assert_eq!(phase(&m), Some(TransferPhase::Done));
            break;
        }
        if let Some((at, away)) = drop_at {
            let bytes = m.transfer().map_or(0, |t| t.bytes);
            if dropped.is_none() && bytes as f64 >= at * len as f64 {
                relay.drop_out();
                dropped = Some(Instant::now());
            }
            if !back && dropped.is_some_and(|d| d.elapsed() >= away) {
                relay.come_back();
                back = true;
            }
        }
        assert!(rec.began.elapsed() < Duration::from_secs(600), "{name}: took too long");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    rec.save(name);
    eprintln!("{name}: {} samples over {:?}", rec.rows[0].len(), rec.began.elapsed());
}

#[tokio::test]
#[ignore]
async fn record_traces() {
    // About 100 s at 4 MB/s.
    record("steady-4mbs", 400_000_000, 4_000_000, None, None).await;
    // The same with a 6 s drop at 40 %; the sender re-reads 160 MB at
    // 32 MB/s (5 s) before it continues: the owner's resume, scaled down.
    record("resume-4mbs", 400_000_000, 4_000_000, Some((0.4, Duration::from_secs(6))), Some(32_000_000)).await;
    // A short one: about 20 s in all.
    record("short-3mbs", 60_000_000, 3_000_000, None, None).await;
}
