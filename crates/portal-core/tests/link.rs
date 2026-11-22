//! A link = one TLS 1.3 connection after the hello exchange.

use std::net::SocketAddr;
use std::time::Duration;

use portal_core::identity::Identity;
use portal_core::model::{Os, Role};
use portal_core::net::link::{self, HandshakeError, Inbound, Link, LinkEvent};
use portal_core::net::{dial, listen};
use portal_core::proto::{self, Frame, Hello, Msg};
use portal_core::tls;
use tokio::sync::mpsc;

struct End {
    _dir: tempfile::TempDir,
    id: Identity,
}

fn end() -> End {
    let dir = tempfile::tempdir().unwrap();
    let id = Identity::load_or_create(dir.path()).unwrap();
    End { _dir: dir, id }
}

fn hello(id: &Identity, name: &str, proto_version: u32) -> Hello {
    Hello {
        proto: proto_version,
        id: id.device_id.clone(),
        name: name.into(),
        os: Os::current(),
        role: Role::Both,
        app: "test".into(),
        port: 1,
    }
}

/// Server accepts one connection, client dials it; returns both links.
async fn pair_of_links(
    server: &End,
    client: &End,
) -> (Link, mpsc::Receiver<LinkEvent>, Link, mpsc::Receiver<LinkEvent>) {
    let listeners = listen::bind(0, true).await.unwrap();
    let addr: SocketAddr = format!("127.0.0.1:{}", listeners.port).parse().unwrap();
    let server_cfg = tls::server_config(&server.id).unwrap();
    let server_hello = hello(&server.id, "Server", proto::PROTOCOL_VERSION);
    let (stx, srx) = mpsc::channel(64);
    let accept = tokio::spawn(async move {
        let (tcp, _) = listeners.accept().await.unwrap();
        let tls = link::accept_tls(tcp, server_cfg).await.unwrap();
        link::handshake(tls, false, server_hello, stx).await.unwrap()
    });
    let tcp = dial::dial(addr, None, Duration::from_secs(3)).await.unwrap();
    let tls =
        link::connect_tls(tcp, tls::client_config(&client.id, Some(server.id.fingerprint)).unwrap()).await.unwrap();
    let (ctx, crx) = mpsc::channel(64);
    let client_link =
        link::handshake(tls, true, hello(&client.id, "Client", proto::PROTOCOL_VERSION), ctx).await.unwrap();
    (accept.await.unwrap(), srx, client_link, crx)
}

async fn next_msg(rx: &mut mpsc::Receiver<LinkEvent>) -> Msg {
    match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await.expect("timed out").expect("closed") {
        LinkEvent::Msg(_, m) => m,
        LinkEvent::Closed(_, why) => panic!("link closed: {why}"),
    }
}

#[tokio::test]
async fn both_sides_learn_who_the_other_is_and_derive_the_same_session_secret() {
    let (s, c) = (end(), end());
    let (server_link, _srx, client_link, _crx) = pair_of_links(&s, &c).await;

    assert_eq!(server_link.info().hello.name, "Client");
    assert_eq!(server_link.info().hello.id, c.id.device_id);
    assert_eq!(server_link.info().peer_fp, c.id.fingerprint);
    assert!(!server_link.info().dialer);
    assert_eq!(client_link.info().hello.name, "Server");
    assert_eq!(client_link.info().peer_fp, s.id.fingerprint);
    assert!(client_link.info().dialer);
    assert_eq!(server_link.info().exporter, client_link.info().exporter);
    assert_ne!(server_link.info().exporter, [0u8; 32]);
}

#[tokio::test]
async fn control_messages_flow_both_ways() {
    let (s, c) = (end(), end());
    let (server_link, mut srx, client_link, mut crx) = pair_of_links(&s, &c).await;
    client_link.send(Msg::PairAsk).await.unwrap();
    assert_eq!(next_msg(&mut srx).await, Msg::PairAsk);
    server_link.send(Msg::PairStop).await.unwrap();
    assert_eq!(next_msg(&mut crx).await, Msg::PairStop);
}

#[tokio::test]
async fn transfer_messages_and_data_reach_the_sink_in_order() {
    let (s, c) = (end(), end());
    let (server_link, _srx, client_link, _crx) = pair_of_links(&s, &c).await;
    let (sink_tx, mut sink) = mpsc::channel(16);
    server_link.set_sink(Some(sink_tx));

    client_link
        .send_ordered(Msg::File { tid: 1, idx: 0, path: vec!["a".into()], size: 3, mtime: None, from: 0 })
        .await
        .unwrap();
    client_link.send_data(vec![1, 2, 3]).await.unwrap();
    client_link.send_ordered(Msg::Done { tid: 1 }).await.unwrap();

    let got = [sink.recv().await.unwrap(), sink.recv().await.unwrap(), sink.recv().await.unwrap()];
    assert!(matches!(&got[0], Inbound::Msg(Msg::File { idx: 0, .. })));
    assert!(matches!(&got[1], Inbound::Data(d) if d == &vec![1, 2, 3]));
    assert!(matches!(&got[2], Inbound::Msg(Msg::Done { tid: 1 })));
}

#[tokio::test]
async fn closing_one_side_is_reported_on_the_other() {
    let (s, c) = (end(), end());
    let (server_link, mut srx, client_link, _crx) = pair_of_links(&s, &c).await;
    client_link.close();
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(LinkEvent::Closed(id, _)) = srx.recv().await {
                return id;
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(closed, server_link.info().id);
    assert!(client_link.is_closed());
}

#[tokio::test]
async fn a_peer_on_another_protocol_version_gets_a_clear_error() {
    let (s, c) = (end(), end());
    let listeners = listen::bind(0, true).await.unwrap();
    let addr: SocketAddr = format!("127.0.0.1:{}", listeners.port).parse().unwrap();
    let server_cfg = tls::server_config(&s.id).unwrap();
    let server_hello = hello(&s.id, "Server", proto::PROTOCOL_VERSION);
    let accept = tokio::spawn(async move {
        let (tcp, _) = listeners.accept().await.unwrap();
        let tls = link::accept_tls(tcp, server_cfg).await.unwrap();
        link::handshake(tls, false, server_hello, mpsc::channel(4).0).await
    });
    let tcp = dial::dial(addr, None, Duration::from_secs(3)).await.unwrap();
    let mut tls = link::connect_tls(tcp, tls::client_config(&c.id, None).unwrap()).await.unwrap();
    proto::write_frame(&mut tls, &Frame::Control(Msg::Hello(hello(&c.id, "Future", 99)))).await.unwrap();
    use tokio::io::AsyncWriteExt;
    tls.flush().await.unwrap();

    match accept.await.unwrap() {
        Err(HandshakeError::Incompatible { theirs, name, .. }) => {
            assert_eq!(theirs, 99);
            assert_eq!(name, "Future");
        }
        other => panic!("expected Incompatible, got {:?}", other.map(|l| l.info().hello.name.clone())),
    }
    // The client got the server's hello and then an explicit error.
    let mut seen = Vec::new();
    while let Ok(Ok(frame)) = tokio::time::timeout(Duration::from_secs(2), proto::read_frame(&mut tls)).await {
        seen.push(frame);
    }
    assert!(
        seen.iter()
            .any(|f| matches!(f, Frame::Control(Msg::Error { code: proto::ErrorCode::IncompatibleVersion, .. }))),
        "{seen:?}"
    );
}

/// Cancel must be the last thing of a transfer the other side hears: file
/// data still queued when it's cancelled is dropped, not sent after it (the
/// other side has left the transfer by then, and a new one may be starting).
#[tokio::test]
async fn data_queued_when_a_transfer_is_cancelled_is_never_sent_after_the_cancel() {
    let (s, c) = (end(), end());
    let (server, _srx, client, _crx) = pair_of_links(&s, &c).await;
    // The receiver isn't reading: its sink holds one item, so the socket and
    // then the sender's queue fill up.
    let (sink_tx, mut sink) = mpsc::channel(1);
    server.set_sink(Some(sink_tx));
    let chunk = vec![7u8; 1024 * 1024];
    let mut queued = 0;
    while tokio::time::timeout(Duration::from_millis(300), client.send_data(chunk.clone())).await.is_ok() {
        queued += 1;
        assert!(queued < 1000, "the queue never filled");
    }
    client.discard_queued();
    let cancel = Msg::Cancel { tid: 9, reason: proto::CancelReason::User };
    client.send(cancel).await.unwrap();

    let mut seen = Vec::new();
    while let Ok(Some(item)) = tokio::time::timeout(Duration::from_secs(2), sink.recv()).await {
        seen.push(match item {
            Inbound::Data(_) => "data",
            Inbound::Msg(Msg::Cancel { .. }) => "cancel",
            Inbound::Msg(_) => "other",
        });
    }
    let at = seen.iter().position(|k| *k == "cancel").expect("the cancel arrived");
    assert!(seen[at + 1..].iter().all(|k| *k != "data"), "data after the cancel: {seen:?}");
}

/// Stray file data outside a transfer (from one that just ended) is dropped;
/// the connection stays up.
#[tokio::test]
async fn stray_file_data_outside_a_transfer_keeps_the_connection() {
    let (s, c) = (end(), end());
    let (server, mut srx, client, _crx) = pair_of_links(&s, &c).await;
    client.send_data(vec![1u8; 4096]).await.unwrap();
    client.send(Msg::Ping).await.unwrap();
    client.send(Msg::Accept { tid: 3 }).await.unwrap();
    assert!(matches!(next_msg(&mut srx).await, Msg::Accept { tid: 3 }));
    assert!(!server.is_closed());
}
