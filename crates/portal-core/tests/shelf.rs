//! The Shelf between devices (Mission 05 §3.2): shared only by mutual
//! consent, never relayed, kept on disk (encrypted), and a removed item
//! never comes back. Cores in one process over loopback.

mod common;

use std::path::Path;
use std::time::Duration;

use common::{TestNode, pair, start, start_in};
use portal_core::events::Snapshot;

async fn share(a: &TestNode, b: &TestNode) {
    a.core.set_shelf_share(&b.id().await, true).await;
    b.core.set_shelf_share(&a.id().await, true).await;
}

fn texts(s: &Snapshot) -> Vec<String> {
    let mut v: Vec<String> = s.shelf.iter().filter_map(|i| i.preview.clone()).collect();
    v.sort();
    v
}

fn has(s: &Snapshot, text: &str) -> bool {
    s.shelf.iter().any(|i| i.preview.as_deref() == Some(text))
}

fn named(s: &Snapshot, name: &str) -> bool {
    s.shelf.iter().any(|i| i.name == name)
}

async fn id_of(node: &TestNode, text: &str) -> String {
    let s = node.core.snapshot().await.unwrap();
    s.shelf.iter().find(|i| i.preview.as_deref() == Some(text)).map(|i| i.id.clone()).expect("on the shelf")
}

/// Stays false for `for_` (nothing arrives that shouldn't).
async fn never(node: &TestNode, what: &str, for_: Duration, pred: impl Fn(&Snapshot) -> bool) {
    let until = tokio::time::Instant::now() + for_;
    while tokio::time::Instant::now() < until {
        let s = node.core.snapshot().await.unwrap();
        assert!(!pred(&s), "{}: {what}: {:?}", node.name, texts(&s));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn restart(node: TestNode) -> TestNode {
    let (name, port) = (node.name.clone(), node.port);
    node.core.shutdown().await;
    let TestNode { dir, .. } = node;
    start_in(&name, dir, |c| c.port = port).await
}

/// Every file under `dir`, as text where possible: no `secret` anywhere.
fn assert_no_plaintext(dir: &Path, secret: &str) {
    fn walk(dir: &Path, secret: &str) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, secret);
            } else {
                let bytes = std::fs::read(&p).unwrap();
                let hay = String::from_utf8_lossy(&bytes);
                assert!(!hay.contains(secret), "plaintext of a private item in {}", p.display());
            }
        }
    }
    walk(dir, secret);
}

#[tokio::test]
async fn a_triangle_converges_with_one_away_and_a_removal_never_comes_back() {
    let mut a = start("MacBook Pro", |_| {}).await;
    let mut b = start("Desk PC", |_| {}).await;
    let mut c = start("MacBook Air", |_| {}).await;
    pair(&mut a, &mut b).await;
    pair(&mut b, &mut c).await;
    pair(&mut a, &mut c).await;
    share(&a, &b).await;
    share(&b, &c).await;
    share(&a, &c).await;

    a.core.shelf_add("Vercel key", "VERCEL=one", false).await.unwrap();
    b.wait_for("b has it", |s| has(s, "VERCEL=one")).await;
    c.wait_for("c has it", |s| has(s, "VERCEL=one")).await;

    // B renames A's item: A and C both see it (C through A, the origin).
    let id = id_of(&b, "VERCEL=one").await;
    b.core.shelf_edit(&id, Some("Vercel token".into()), None).await.unwrap();
    a.wait_for("renamed on a", |s| named(s, "Vercel token")).await;
    c.wait_for("renamed on c", |s| named(s, "Vercel token")).await;

    // C goes away; meanwhile A adds one and B removes the first.
    let c_dir_name = c.name.clone();
    let c = {
        let port = c.port;
        c.core.shutdown().await;
        let TestNode { dir, .. } = c;
        (dir, port)
    };
    a.core.shelf_add("", "ADDRESS=1 Infinite Loop", false).await.unwrap();
    b.wait_for("b has the new one", |s| has(s, "ADDRESS=1 Infinite Loop")).await;
    b.core.shelf_remove(Some(vec![id.clone()])).await;
    a.wait_for("removed on a", |s| !has(s, "VERCEL=one")).await;

    // C is back: it converges, and the removed one doesn't come back anywhere.
    let mut c = start_in(&c_dir_name, c.0, move |cfg| cfg.port = c.1).await;
    let caught_up = |s: &Snapshot| has(s, "ADDRESS=1 Infinite Loop") && !has(s, "VERCEL=one");
    let until = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let s = c.core.snapshot().await.unwrap();
        if caught_up(&s) {
            break;
        }
        // Say what C had, so a failure shows where it stuck
        let peers: Vec<String> =
            s.peers.iter().map(|p| format!("{} online={} shelf={:?}", p.name, p.online, p.shelf)).collect();
        if tokio::time::Instant::now() >= until {
            let sa = a.core.snapshot().await.unwrap();
            let a_sees: Vec<String> =
                sa.peers.iter().map(|p| format!("{} online={} shelf={:?}", p.name, p.online, p.shelf)).collect();
            let logs: Vec<String> = c
                .drain_logs()
                .into_iter()
                .filter(|l| l.contains("helf") || l.contains("onnect") || l.contains("link"))
                .collect();
            panic!("c didn't catch up: shelf {:?}, peers {peers:?}; a sees {a_sees:?}; c's lines {logs:#?}", texts(&s));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _ = &mut c;
    for n in [&a, &b] {
        never(n, "the removed item came back", Duration::from_secs(2), |s| has(s, "VERCEL=one")).await;
    }
    let (sa, sb, sc) =
        (a.core.snapshot().await.unwrap(), b.core.snapshot().await.unwrap(), c.core.snapshot().await.unwrap());
    assert_eq!(texts(&sa), texts(&sb));
    assert_eq!(texts(&sb), texts(&sc));

    // Kept across a restart, and still not the removed one.
    let a = restart(a).await;
    let s = a.core.snapshot().await.unwrap();
    assert!(has(&s, "ADDRESS=1 Infinite Loop") && !has(&s, "VERCEL=one"), "{:?}", texts(&s));
    let _ = (&mut b, &mut c);
}

#[tokio::test]
async fn a_chain_never_relays() {
    // A–B and B–C paired and sharing; A and C don't know each other.
    let mut a = start("MacBook Pro", |_| {}).await;
    let mut b = start("Desk PC", |_| {}).await;
    let mut c = start("MacBook Air", |_| {}).await;
    pair(&mut a, &mut b).await;
    pair(&mut b, &mut c).await;
    share(&a, &b).await;
    share(&b, &c).await;

    a.core.shelf_add("", "from A", false).await.unwrap();
    b.core.shelf_add("", "from B", false).await.unwrap();
    c.core.shelf_add("", "from C", false).await.unwrap();
    b.wait_for("b has both neighbours'", |s| has(s, "from A") && has(s, "from C")).await;
    a.wait_for("a has b's", |s| has(s, "from B")).await;
    c.wait_for("c has b's", |s| has(s, "from B")).await;
    never(&a, "C's item reached A through B", Duration::from_secs(2), |s| has(s, "from C")).await;
    never(&c, "A's item reached C through B", Duration::from_secs(1), |s| has(s, "from A")).await;

    // B removes A's item: back at A (its origin), gone on B, never on C.
    let id = id_of(&b, "from A").await;
    b.core.shelf_remove(Some(vec![id])).await;
    a.wait_for("removed at its origin", |s| !has(s, "from A")).await;
}

#[tokio::test]
async fn nothing_is_shared_unless_both_say_yes_and_unsharing_takes_it_back() {
    let mut a = start("MacBook Pro", |_| {}).await;
    let mut b = start("Desk PC", |_| {}).await;
    pair(&mut a, &mut b).await;
    a.core.set_shelf_share(&b.id().await, true).await; // only A says yes
    a.core.shelf_add("", "only here", false).await.unwrap();
    b.core.shelf_add("", "only there", false).await.unwrap();
    never(&b, "shared without B's yes", Duration::from_secs(2), |s| has(s, "only here")).await;
    never(&a, "shared without B's yes", Duration::from_millis(500), |s| has(s, "only there")).await;

    // B says yes: both ways now.
    b.core.set_shelf_share(&a.id().await, true).await;
    b.wait_for("shared", |s| has(s, "only here")).await;
    a.wait_for("shared", |s| has(s, "only there")).await;

    // B stops sharing: A's items leave B, B's leave A.
    b.core.set_shelf_share(&a.id().await, false).await;
    b.wait_for("a's left b", |s| !has(s, "only here")).await;
    a.wait_for("b's left a", |s| !has(s, "only there")).await;
    assert!(has(&a.core.snapshot().await.unwrap(), "only here"), "a keeps its own");
}

#[tokio::test]
async fn private_items_leave_no_plaintext_in_the_data_folders_or_the_logs() {
    let mut a = start("MacBook Pro", |_| {}).await;
    let mut b = start("Desk PC", |_| {}).await;
    pair(&mut a, &mut b).await;
    share(&a, &b).await;
    const SECRET: &str = "sk_live_PLAINTEXT_CANARY_9431";
    a.core.shelf_add("Stripe key", SECRET, true).await.unwrap();
    let s = b.wait_for("b has it", |s| s.shelf.iter().any(|i| i.private)).await;
    let item = s.shelf.iter().find(|i| i.private).unwrap();
    assert_eq!(item.preview, None, "no preview of a private item");
    // Reveal and Copy work, through the core only.
    assert_eq!(b.core.shelf_reveal(&item.id).await.as_deref(), Some(SECRET));
    b.core.shelf_copy(&item.id).await.unwrap();

    // Every log line, with its technical detail (what Details shows and the
    // saved technical log holds), on both computers.
    let mut lines: Vec<String> = a.drain_lines().iter().map(|l| format!("{l:?}")).collect();
    lines.extend(b.drain_lines().iter().map(|l| format!("{l:?}")));
    let restarted = restart(a).await;
    for n in [&restarted, &b] {
        assert_no_plaintext(n.dir.path(), SECRET);
    }
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|l| !l.contains("PLAINTEXT_CANARY")), "a log line has the secret");
}
