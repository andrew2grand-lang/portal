//! Mission 04 §1.1: the core reports how connecting to each paired device is
//! going; "isn't reachable" only after the grace period and real failed
//! attempts. Two cores in one process, loopback.

mod common;

use std::time::Duration;

use portal_core::events::{ReachState, ReachStep, Snapshot};

fn reach_of(s: &Snapshot) -> Option<portal_core::events::ReachView> {
    s.peers.iter().find(|p| p.paired).and_then(|p| p.reach.clone())
}

#[tokio::test]
async fn a_paired_device_is_connecting_first_and_unreachable_only_after_the_grace() {
    let grace = Duration::from_secs(6);
    let mut a = common::start("Mac", |c| c.reach_grace = grace).await;
    let mut b = common::start("PC", |_| {}).await;
    common::pair(&mut a, &mut b).await;
    assert!(reach_of(&a.core.snapshot().await.unwrap()).is_none(), "connected: nothing to report");

    b.core.shutdown().await;
    let s = a.wait_for("connecting", |s| reach_of(s).is_some()).await;
    let r = reach_of(&s).unwrap();
    assert_eq!(r.state, ReachState::Connecting, "not 'isn't reachable' the moment it drops");

    // Dials every second to a closed port: two failures well within the grace,
    // but still "connecting" until the grace is over.
    let s = a.wait_for("two failed dials", |s| reach_of(s).is_some_and(|r| r.attempts >= 2)).await;
    let r = reach_of(&s).unwrap();
    assert_eq!(r.state, ReachState::Connecting);
    assert_eq!(r.step, ReachStep::TryingKnown);

    let s = a.wait_for("unreachable", |s| reach_of(s).is_some_and(|r| r.state == ReachState::Unreachable)).await;
    let r = reach_of(&s).unwrap();
    assert!(
        r.since_ms + 6000
            <= std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64 + 50,
        "not before the grace"
    );
    let failure = r.last_failure.unwrap_or_default();
    assert!(failure.contains("127.0.0.1"), "says what failed: {failure}");
    assert!(r.attempts >= 2);
    // Review 04 (developer): what the window says, Details says too, once
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let logs = a.drain_logs();
    let said: Vec<_> = logs.iter().filter(|l| l.contains("isn’t reachable")).collect();
    assert_eq!(said.len(), 1, "{logs:#?}");
    assert!(said[0].contains("PC") && said[0].contains("127.0.0.1"), "{}", said[0]);
}

#[tokio::test]
async fn try_again_starts_over_as_connecting() {
    let mut a = common::start("Mac", |c| c.reach_grace = Duration::from_millis(1500)).await;
    let mut b = common::start("PC", |_| {}).await;
    common::pair(&mut a, &mut b).await;
    b.core.shutdown().await;
    a.wait_for("unreachable", |s| reach_of(s).is_some_and(|r| r.state == ReachState::Unreachable)).await;
    a.core.retry_discovery().await;
    let r = reach_of(&a.core.snapshot().await.unwrap()).unwrap();
    assert_eq!(r.state, ReachState::Connecting);
}

#[tokio::test]
async fn back_online_clears_the_reach_state() {
    let mut a = common::start("Mac", |c| c.reach_grace = Duration::from_millis(500)).await;
    let mut b = common::start("PC", |_| {}).await;
    common::pair(&mut a, &mut b).await;
    let port = b.port;
    b.core.shutdown().await;
    let dir = b.dir;
    a.wait_for("unreachable", |s| reach_of(s).is_some_and(|r| r.state == ReachState::Unreachable)).await;
    let mut b = common::start_in("PC", dir, |c| c.port = port).await;
    a.wait_for_within(Duration::from_secs(5), "back", common::paired_online).await;
    b.wait_for("back", common::paired_online).await;
    assert!(reach_of(&a.core.snapshot().await.unwrap()).is_none());
}
