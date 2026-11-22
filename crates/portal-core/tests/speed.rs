//! Mission 04 §1.2: the sender's progress is what the receiver confirmed,
//! "Working out time left…" comes first, and the speed shown is close to the
//! real one. Two cores in one process, loopback, the sender held to 8 MB/s.

mod common;

use std::time::{Duration, Instant};

use portal_core::events::{Event, ProgressView};
use portal_core::transfer::estimate::Eta;

const MB: u64 = 1_000_000;

#[tokio::test]
async fn the_sender_measures_what_arrived_and_works_out_time_left_first() {
    let mut mac = common::start("Mac", |c| c.fault_rate = Some(8 * MB)).await;
    let mut pc = common::start("PC", |c| c.fault_free_space = Some(u64::MAX / 4)).await;
    common::pair(&mut mac, &mut pc).await;
    pc.core.set_incoming(&mac.id().await, portal_core::model::Incoming::Always).await;
    let src = mac.dir.path().join("big.bin");
    std::fs::write(&src, vec![7u8; (72 * MB) as usize]).unwrap();
    mac.drain_logs();
    let to = pc.id().await;
    mac.core.send(&to, vec![src]).await.unwrap();

    let started = Instant::now();
    let mut seen: Vec<(Duration, ProgressView)> = Vec::new();
    let deadline = started + Duration::from_secs(30);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), mac.events.recv()).await {
            Ok(Some(Event::Progress(p))) => {
                let done = p.bytes == p.total;
                seen.push((started.elapsed(), p));
                if done {
                    break;
                }
            }
            Ok(Some(_)) | Err(_) => {}
            Ok(None) => panic!("core stopped"),
        }
    }
    assert!(seen.len() > 20, "progress events: {}", seen.len());
    let first = seen[0].0;
    // The first seconds of the run: time left is being worked out, speed not shown yet.
    for (at, p) in &seen {
        if *at < first + Duration::from_secs(4) {
            assert_eq!(p.eta, Eta::WorkingOut, "at {at:?}");
        }
        if *at < first + Duration::from_millis(1500) {
            assert_eq!(p.speed, None, "at {at:?}");
        }
    }
    // Later: a speed within 25 % of the 8 MB/s it's held to, from confirmed bytes.
    let late: Vec<f64> =
        seen.iter().filter(|(at, _)| *at > first + Duration::from_secs(4)).filter_map(|(_, p)| p.speed).collect();
    assert!(!late.is_empty(), "a speed is shown after a few seconds");
    for s in &late {
        assert!((*s - 8e6).abs() < 2e6, "speed {s} vs 8 MB/s");
    }
    assert!(seen.iter().any(|(_, p)| p.eta != Eta::WorkingOut), "time left is shown once worked out");
}
