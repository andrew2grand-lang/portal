//! Pairing by typed code between cores (Mission 04 §2): one computer asks,
//! the other asks its person, shows a code, and the code is typed where the
//! pairing started. SPAKE2 underneath (`tests/spake2.rs`, `pairing_code.rs`).

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use common::{TestNode, ask_and_show, pair_step, paired_online, shown_code, start, start_in};
use portal_core::events::{Event, NoticeKind, PairStep, Snapshot};
use portal_core::identity::Identity;
use portal_core::model::{DeviceId, Os, Role};
use portal_core::net::link::{self, Link, LinkEvent};
use portal_core::net::{dial, listen};
use portal_core::node::PairAction;
use portal_core::pairing::{LOCK_AFTER, MAX_TRIES};
use portal_core::proto::{self, Frame, Hello, Msg};
use portal_core::tls;
use tokio::sync::mpsc;

fn free_tcp_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

async fn id_of(n: &TestNode) -> DeviceId {
    n.core.snapshot().await.unwrap().me.id
}

fn failed(s: &Snapshot) -> Option<String> {
    s.pairing.as_ref().and_then(|p| p.failed.clone())
}

fn trusts_anyone(s: &Snapshot) -> bool {
    s.peers.iter().any(|p| p.paired)
}

/// A wrong code: the right one with its last digit changed.
fn wrong(code: &str) -> String {
    let last = code.as_bytes()[5] - b'0';
    format!("{}{}", &code[..5], (last + 1) % 10)
}

/// PC dials the Mac by IP and asks to pair; the Mac shows the code.
async fn asked(mac: &mut TestNode, pc: &mut TestNode) -> String {
    pc.core.connect(&mac.addr()).await.unwrap();
    ask_and_show(mac, pc).await
}

/// PC types `code`; waits until it's checked (wrong, paired or over).
async fn type_code(pc: &mut TestNode, code: &str) -> Snapshot {
    pc.core.pairing(PairAction::Code(code.into())).await.unwrap();
    pc.wait_for("checked", |s| {
        s.pairing.as_ref().is_none_or(|p| p.failed.is_some() || matches!(p.step, PairStep::Wrong | PairStep::Paired))
    })
    .await
}

#[tokio::test]
async fn the_right_code_pairs_both_and_both_say_so() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    pc.core.connect(&mac.addr()).await.unwrap();
    pc.core.pair(&id_of(&mac).await).await.unwrap();

    // The Mac asks its person first, saying who and from where; no code yet
    let s = mac.wait_for("asked", |s| pair_step(s) == Some(PairStep::Asked)).await;
    let p = s.pairing.unwrap();
    assert_eq!((p.peer_name.as_str(), p.peer_addr.as_str(), p.mine), ("Desk PC", "127.0.0.1", false));
    assert_eq!(p.code, None);
    let s = pc.wait_for("asking", |s| pair_step(s) == Some(PairStep::Asking)).await;
    assert!(s.pairing.unwrap().mine);

    mac.core.pairing(PairAction::ShowCode).await.unwrap();
    let s = mac.wait_for("code", |s| shown_code(s).is_some()).await;
    let code = shown_code(&s).unwrap();
    assert!(code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()), "{code}");
    let s = pc.wait_for("type it", |s| pair_step(s) == Some(PairStep::Code)).await;
    assert_eq!(s.pairing.unwrap().code, None, "the asker never has the code");

    // Typed with a space, as it's shown
    pc.core.pairing(PairAction::Code(format!("{} {}", &code[..3], &code[3..]))).await.unwrap();
    pc.wait_for("pc paired", paired_online).await;
    mac.wait_for("mac paired", paired_online).await;
    for (n, other) in [(&mut pc, "MacBook Pro"), (&mut mac, "Desk PC")] {
        let (logs, notices) = n.drain_logs_keeping(|e| matches!(e, Event::Notice(x) if x.kind == NoticeKind::Paired));
        assert!(logs.iter().any(|l| l.contains(&format!("Paired with {other}"))), "{logs:#?}");
        assert_eq!(notices.len(), 1, "a notification: Paired with {other}");
    }
}

#[tokio::test]
async fn something_that_isnt_six_digits_isnt_a_try() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    let code = asked(&mut mac, &mut pc).await;
    let err = pc.core.pairing(PairAction::Code("12a 456".into())).await.unwrap_err();
    assert!(err.contains("6 digits"), "{err}");
    assert_eq!(pair_step(&pc.core.snapshot().await.unwrap()), Some(PairStep::Code));
    let s = type_code(&mut pc, &code).await;
    assert!(s.pairing.is_none_or(|p| p.step == PairStep::Paired));
    mac.wait_for("paired", paired_online).await;
}

#[tokio::test]
async fn a_typo_can_be_typed_again() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    let code = asked(&mut mac, &mut pc).await;

    let s = type_code(&mut pc, &wrong(&code)).await;
    assert!(!trusts_anyone(&s));
    let p = s.pairing.unwrap();
    assert_eq!((p.step, p.tries_left, p.failed), (PairStep::Wrong, MAX_TRIES - 1, None));
    // The Mac still shows the same code
    let s = mac.wait_for("wrong seen", |s| pair_step(s) == Some(PairStep::Wrong)).await;
    assert_eq!(shown_code(&s), Some(code.clone()));

    type_code(&mut pc, &code).await;
    pc.wait_for("pc paired", paired_online).await;
    mac.wait_for("mac paired", paired_online).await;
}

#[tokio::test]
async fn three_wrong_codes_end_it_on_both_sides() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    let code = asked(&mut mac, &mut pc).await;
    for n in 1..MAX_TRIES {
        let s = type_code(&mut pc, &wrong(&code)).await;
        assert_eq!(s.pairing.unwrap().tries_left, MAX_TRIES - n);
    }
    let s = type_code(&mut pc, &wrong(&code)).await;
    assert!(failed(&s).is_some_and(|f| f.contains("Nothing was paired")), "{:?}", failed(&s));
    let s = mac.wait_for("over", |s| failed(s).is_some()).await;
    assert!(failed(&s).unwrap().contains("typed wrong"), "{:?}", failed(&s));
    // The right code after that changes nothing
    assert!(pc.core.pairing(PairAction::Code(code)).await.is_ok());
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!trusts_anyone(&mac.core.snapshot().await.unwrap()));
    assert!(!trusts_anyone(&pc.core.snapshot().await.unwrap()));
}

#[tokio::test]
async fn wrong_codes_from_anyone_lock_pairing_until_it_is_allowed_again() {
    // Many asks from one address (every test computer is 127.0.0.1)
    let mut mac = start("MacBook Pro", |c| c.pair_asks = 100).await;
    let mut wrongs = 0;
    while wrongs < LOCK_AFTER {
        // A new asker each time: new keys cost nothing
        let mut pc = start("Stranger", |c| c.pair_asks = 100).await;
        let code = asked(&mut mac, &mut pc).await;
        loop {
            let s = type_code(&mut pc, &wrong(&code)).await;
            wrongs += 1;
            if failed(&s).is_some() {
                break;
            }
        }
        mac.wait_for("over", |s| failed(s).is_some()).await;
        mac.core.pairing(PairAction::Dismiss).await.unwrap();
    }
    let s = mac.wait_for("locked", |s| s.me.pairing_locked).await;
    assert!(s.pairing.is_none());
    // Review 04 (privacy): it says who typed the last wrong code, and when,
    // on screen and in Details
    let by = s.me.pairing_locked_by.clone().expect("who");
    assert!(by.contains("Stranger") && by.contains("127.0.0.1"), "{by}");
    assert!(s.me.pairing_locked_at_ms.is_some());
    let logs = mac.drain_logs();
    assert!(logs.iter().any(|l| l.starts_with("pair") && l.contains("Pairing is locked")), "{logs:#?}");

    // Locked: a new ask is turned down, without asking the Mac's person
    let mut pc = start("Desk PC", |_| {}).await;
    pc.core.connect(&mac.addr()).await.unwrap();
    pc.core.pair(&id_of(&mac).await).await.unwrap();
    let s = pc.wait_for("turned down", |s| failed(s).is_some()).await;
    assert!(failed(&s).unwrap().contains("isn’t taking pairing requests"), "{:?}", failed(&s));
    assert!(mac.core.snapshot().await.unwrap().pairing.is_none(), "no prompt on the Mac");

    // Its person allows pairing again
    mac.core.pairing(PairAction::AllowAgain).await.unwrap();
    mac.wait_for("unlocked", |s| !s.me.pairing_locked).await;
    pc.core.pairing(PairAction::Dismiss).await.unwrap();
    let code = ask_and_show(&mut mac, &mut pc).await;
    type_code(&mut pc, &code).await;
    mac.wait_for("paired", paired_online).await;
}

#[tokio::test]
async fn ignore_says_nothing_and_the_asker_gives_up_after_the_timeout() {
    let short = |c: &mut portal_core::Config| c.pair_timeout = Duration::from_secs(3);
    let mut mac = start("MacBook Pro", short).await;
    let mut pc = start("Desk PC", short).await;
    pc.core.connect(&mac.addr()).await.unwrap();
    let mac_id = id_of(&mac).await;
    pc.core.pair(&mac_id).await.unwrap();
    mac.wait_for("asked", |s| pair_step(s) == Some(PairStep::Asked)).await;
    mac.core.pairing(PairAction::Ignore).await.unwrap();
    assert!(mac.core.snapshot().await.unwrap().pairing.is_none());

    // The PC can't tell: still asking, until the timeout
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(pair_step(&pc.core.snapshot().await.unwrap()), Some(PairStep::Asking));
    let s = pc.wait_for("gave up", |s| failed(s).is_some()).await;
    assert!(failed(&s).unwrap().contains("No answer from MacBook Pro"), "{:?}", failed(&s));

    // Asking again straight away: the Mac isn't asked (and the PC still can't tell)
    pc.core.pairing(PairAction::Dismiss).await.unwrap();
    pc.core.pair(&mac_id).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(mac.core.snapshot().await.unwrap().pairing.is_none(), "no prompt within a minute of Ignore");
    assert_eq!(pair_step(&pc.core.snapshot().await.unwrap()), Some(PairStep::Asking));
}

#[tokio::test]
async fn cancelling_on_either_side_pairs_nobody() {
    for asker_cancels in [true, false] {
        let mut mac = start("MacBook Pro", |_| {}).await;
        let mut pc = start("Desk PC", |_| {}).await;
        asked(&mut mac, &mut pc).await;
        let (quits, told, name) =
            if asker_cancels { (&mut pc, &mut mac, "Desk PC") } else { (&mut mac, &mut pc, "MacBook Pro") };
        quits.core.pairing(PairAction::Cancel).await.unwrap();
        assert!(quits.core.snapshot().await.unwrap().pairing.is_none());
        let s = told.wait_for("told", |s| failed(s).is_some()).await;
        assert!(failed(&s).unwrap().contains(&format!("{name} cancelled")), "{:?}", failed(&s));
        assert!(!trusts_anyone(&s));
    }
}

#[tokio::test]
async fn a_code_nobody_types_times_out_on_both() {
    let short = |c: &mut portal_core::Config| c.pair_timeout = Duration::from_secs(2);
    let mut mac = start("MacBook Pro", short).await;
    let mut pc = start("Desk PC", short).await;
    asked(&mut mac, &mut pc).await;
    // Review 04 (privacy): both screens can count the code's time down
    for n in [&mac, &pc] {
        let shown = n.core.snapshot().await.unwrap().pairing.expect("pairing");
        assert!(shown.left_ms > 0 && shown.left_ms <= 2000, "{}: {}", n.name, shown.left_ms);
    }
    let s = mac.wait_for_within(Duration::from_secs(10), "mac timed out", |s| failed(s).is_some()).await;
    assert!(failed(&s).unwrap().contains("Nothing was paired"));
    assert!(failed(&s).unwrap().contains("The code expired"), "{:?}", failed(&s));
    pc.wait_for_within(Duration::from_secs(10), "pc told", |s| failed(s).is_some()).await;
    assert!(!trusts_anyone(&pc.core.snapshot().await.unwrap()));
}

// ---- Attacks --------------------------------------------------------------

/// A raw protocol speaker with its own key, claiming any id and name.
struct Raw {
    _dir: tempfile::TempDir,
    id: Identity,
}

fn raw() -> Raw {
    let dir = tempfile::tempdir().unwrap();
    let id = Identity::load_or_create(dir.path()).unwrap();
    Raw { _dir: dir, id }
}

fn claim(id: &DeviceId, name: &str, version: u32) -> Hello {
    Hello {
        proto: version,
        id: id.clone(),
        name: name.into(),
        os: Os::current(),
        role: Role::Both,
        app: "x".into(),
        port: 1,
    }
}

/// Dials `to` as a TLS client.
async fn raw_dial(me: &Raw, to: &TestNode, hello: Hello) -> (Link, mpsc::Receiver<LinkEvent>) {
    let tcp = dial::dial(to.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls = link::connect_tls(tcp, tls::client_config(&me.id, None).unwrap()).await.unwrap();
    let (tx, rx) = mpsc::channel(64);
    (link::handshake(tls, true, hello, tx).await.unwrap(), rx)
}

/// Listens; the first connection becomes a link (as the TLS server).
async fn raw_listen(
    me: &Raw,
    hello: Hello,
) -> (SocketAddr, tokio::task::JoinHandle<(Link, mpsc::Receiver<LinkEvent>)>) {
    let listeners = listen::bind(0, true).await.unwrap();
    let addr: SocketAddr = format!("127.0.0.1:{}", listeners.port).parse().unwrap();
    let cfg = tls::server_config(&me.id).unwrap();
    let task = tokio::spawn(async move {
        let (tcp, _) = listeners.accept().await.unwrap();
        let tls = link::accept_tls(tcp, cfg).await.unwrap();
        let (tx, rx) = mpsc::channel(64);
        (link::handshake(tls, false, hello, tx).await.unwrap(), rx)
    });
    (addr, task)
}

fn is_pairing(m: &Msg) -> bool {
    matches!(
        m,
        Msg::PairAsk
            | Msg::PairReady
            | Msg::PairKey { .. }
            | Msg::PairCheck { .. }
            | Msg::PairWrong { .. }
            | Msg::PairDone
            | Msg::PairStop
    )
}

async fn next_pairing(rx: &mut mpsc::Receiver<LinkEvent>) -> Option<Msg> {
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(ev) = rx.recv().await {
            if let LinkEvent::Msg(_, m) = ev
                && is_pairing(&m)
            {
                return Some(m);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

/// Mission 04 §2: a man in the middle, with a TLS session to each side,
/// relaying every pairing message as it is, and the code read off the real
/// screen: both sides fail.
#[tokio::test]
async fn a_man_in_the_middle_fails_on_both_sides() {
    let mut alice = start("Alice’s Mac", |_| {}).await;
    let mut bob = start("Bob’s PC", |_| {}).await;
    let mallory = raw();
    let (alice_id, bob_id) = (id_of(&alice).await, id_of(&bob).await);
    let v = proto::PROTOCOL_VERSION;

    // Mallory poses as Bob to Alice, and as Alice to Bob
    let (addr, as_bob) = raw_listen(&mallory, claim(&bob_id, "Bob’s PC", v)).await;
    alice.core.connect(&addr.to_string()).await.unwrap();
    let (to_alice, mut from_alice) = as_bob.await.unwrap();
    let (to_bob, mut from_bob) = raw_dial(&mallory, &bob, claim(&alice_id, "Alice’s Mac", v)).await;
    let relay = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(ev) = from_alice.recv() => if let LinkEvent::Msg(_, m) = ev && is_pairing(&m) { to_bob.send(m).await.ok(); },
                Some(ev) = from_bob.recv() => if let LinkEvent::Msg(_, m) = ev && is_pairing(&m) { to_alice.send(m).await.ok(); },
                else => break,
            }
        }
    });

    // Alice asks "Bob"; Bob's person shows the code; it's typed on Alice
    alice.core.pair(&bob_id).await.unwrap();
    bob.wait_for("asked", |s| pair_step(s) == Some(PairStep::Asked)).await;
    bob.core.pairing(PairAction::ShowCode).await.unwrap();
    let code = shown_code(&bob.wait_for("code", |s| shown_code(s).is_some()).await).unwrap();
    alice.wait_for("type it", |s| pair_step(s) == Some(PairStep::Code)).await;
    let mut last = None;
    for _ in 0..MAX_TRIES {
        let s = type_code(&mut alice, &code).await;
        last = failed(&s);
        if last.is_some() {
            break;
        }
        assert_eq!(pair_step(&s), Some(PairStep::Wrong), "the right code, and still not accepted");
    }
    assert!(last.is_some_and(|f| f.contains("Nothing was paired")));
    bob.wait_for("bob over", |s| failed(s).is_some()).await;
    assert!(!trusts_anyone(&alice.core.snapshot().await.unwrap()), "Alice trusts nobody");
    assert!(!trusts_anyone(&bob.core.snapshot().await.unwrap()), "Bob trusts nobody");
    relay.abort();
}

/// A "shower" that answers with its own confirmation copied from the
/// asker's: refused (different keys per role), nothing paired.
#[tokio::test]
async fn a_confirmation_echoed_back_is_refused() {
    let mut pc = start("Desk PC", |_| {}).await;
    let evil = raw();
    let evil_id = DeviceId::random();
    let (addr, server) = raw_listen(&evil, claim(&evil_id, "Echo", proto::PROTOCOL_VERSION)).await;
    pc.core.connect(&addr.to_string()).await.unwrap();
    let (link, mut rx) = server.await.unwrap();
    pc.core.pair(&evil_id).await.unwrap();
    assert_eq!(next_pairing(&mut rx).await, Some(Msg::PairAsk));
    link.send(Msg::PairReady).await.unwrap();
    pc.wait_for("type it", |s| pair_step(s) == Some(PairStep::Code)).await;
    pc.core.pairing(PairAction::Code("123456".into())).await.unwrap();
    let Some(Msg::PairKey { share }) = next_pairing(&mut rx).await else { panic!("a share") };
    // Its own share back as ours: K comes out the identity, or the keys differ
    link.send(Msg::PairKey { share }).await.unwrap();
    match next_pairing(&mut rx).await {
        Some(Msg::PairCheck { mac }) => {
            // Echo the asker's confirmation as the shower's
            link.send(Msg::PairCheck { mac }).await.unwrap();
            assert_eq!(next_pairing(&mut rx).await, Some(Msg::PairStop), "refused, and said so");
        }
        other => assert_eq!(other, Some(Msg::PairStop), "refused at once"),
    }
    let s = pc.wait_for("refused", |s| failed(s).is_some()).await;
    assert!(!trusts_anyone(&s));
}

/// No downgrade: protocol 2 can't connect at all, and its pairing messages
/// on a protocol 3 link start nothing.
#[tokio::test]
async fn old_pairing_messages_pair_nobody() {
    let mac = start("MacBook Pro", |_| {}).await;
    let old = raw();

    // A protocol-2 hello is turned away: told it's incompatible, and closed
    let tcp = dial::dial(mac.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let tls = link::connect_tls(tcp, tls::client_config(&old.id, None).unwrap()).await.unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    if link::handshake(tls, true, claim(&old.id.device_id, "Old", 2), tx).await.is_ok() {
        let turned_away = tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(ev) = rx.recv().await {
                match ev {
                    LinkEvent::Closed(..) => return true,
                    LinkEvent::Msg(_, Msg::Error { code: proto::ErrorCode::IncompatibleVersion, .. }) => return true,
                    _ => {}
                }
            }
            true
        })
        .await;
        assert_eq!(turned_away, Ok(true), "a protocol-2 link isn't kept");
    }
    let s = mac.core.snapshot().await.unwrap();
    assert!(s.pairing.is_none() && !trusts_anyone(&s));

    // Protocol 3, then the old messages, raw
    let tcp = dial::dial(mac.addr().parse().unwrap(), None, Duration::from_secs(3)).await.unwrap();
    let mut tls = link::connect_tls(tcp, tls::client_config(&old.id, None).unwrap()).await.unwrap();
    let hello = Frame::Control(Msg::Hello(claim(&old.id.device_id, "Old", proto::PROTOCOL_VERSION)));
    proto::write_frame(&mut tls, &hello).await.unwrap();
    assert!(matches!(proto::read_frame(&mut tls).await.unwrap(), Frame::Control(Msg::Hello(_))));
    use tokio::io::AsyncWriteExt;
    for json in [
        r#"{"t":"pair_request"}"#,
        r#"{"t":"pair_nonce","n":"0202020202020202020202020202020202020202020202020202020202020202"}"#,
        r#"{"t":"pair_confirm","ok":true}"#,
    ] {
        let mut frame = vec![1u8];
        frame.extend_from_slice(&(json.len() as u32).to_be_bytes());
        frame.extend_from_slice(json.as_bytes());
        tls.write_all(&frame).await.unwrap();
    }
    tls.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;
    let s = mac.core.snapshot().await.unwrap();
    assert!(s.pairing.is_none(), "no prompt, no code");
    assert!(!trusts_anyone(&s));
}

// ---- Trust after pairing ----------------------------------------------------

#[tokio::test]
async fn trust_survives_a_restart_and_reconnects_without_a_code() {
    let mp = free_tcp_port();
    let mut mac = start("MacBook Pro", |c| c.port = mp).await;
    let mut pc = start("Desk PC", |_| {}).await;
    common::pair(&mut mac, &mut pc).await;
    mac.core.shutdown().await;
    pc.wait_for("pc sees mac gone", |s| !paired_online(s)).await;
    let mut mac = start_in("MacBook Pro", mac.dir, |c| c.port = mp).await;
    mac.wait_for("reconnected", paired_online).await;
    pc.wait_for("reconnected", paired_online).await;
    assert!(mac.core.snapshot().await.unwrap().pairing.is_none(), "no pairing needed again");
}

#[tokio::test]
async fn removing_a_device_revokes_trust_at_once() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    common::pair(&mut mac, &mut pc).await;

    mac.core.remove_peer(&id_of(&pc).await).await;

    mac.wait_for("mac forgot pc", |s| !s.peers.iter().any(|p| p.paired)).await;
    pc.wait_for("pc told it was removed", |s| s.peers.iter().any(|p| p.revoked && !p.paired)).await;
}

/// Mission 05 §2.1: "MacBook Pro removed this pairing." → OK. The Mac becomes
/// just a computer nearby: not paired, not "removed", nothing asking to pair.
#[tokio::test]
async fn ok_after_a_removal_leaves_just_a_nearby_computer() {
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |_| {}).await;
    common::pair(&mut mac, &mut pc).await;
    let mac_id = id_of(&mac).await;
    mac.core.remove_peer(&id_of(&pc).await).await;
    pc.wait_for("pc told it was removed", |s| s.peers.iter().any(|p| p.revoked)).await;

    pc.core.remove_peer(&mac_id).await;
    pc.wait_for("the Mac listed as nearby, nothing more", |s| {
        s.peers.iter().any(|p| p.id == mac_id && !p.revoked && !p.paired) && s.pairing.is_none()
    })
    .await;
}

#[tokio::test]
async fn a_device_whose_key_changed_is_refused() {
    let pp = free_tcp_port();
    let mut mac = start("MacBook Pro", |_| {}).await;
    let mut pc = start("Desk PC", |c| c.port = pp).await;
    common::pair(&mut mac, &mut pc).await;
    pc.core.shutdown().await;
    // Same device id, new key (e.g. key file lost): the Mac must not trust it.
    std::fs::write(pc.dir.path().join("key.bin"), b"lost").unwrap();
    let mut pc = start_in("Desk PC", pc.dir, |c| c.port = pp).await;
    pc.core.connect(&mac.addr()).await.ok();

    let logs = async {
        let mut seen = Vec::new();
        for _ in 0..50 {
            seen.extend(mac.drain_logs());
            if seen.iter().any(|l| l.contains("security key changed")) {
                return seen;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        seen
    }
    .await;
    assert!(logs.iter().any(|l| l.contains("security key changed")), "{logs:?}");
    let s = mac.core.snapshot().await.unwrap();
    assert!(s.peers.iter().any(|p| p.paired && !p.online), "still paired, not connected: {:?}", s.peers);
    let _ = pc.wait_for("pc not trusted", |s| !paired_online(s)).await;
}
