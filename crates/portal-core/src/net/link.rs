//! A link: one authenticated TLS 1.3 connection to a peer, after the hello
//! exchange. A writer task sends control messages ahead of file data; a reader
//! task routes transfer traffic to the active transfer's sink (so file data
//! stays in order with its messages) and everything else to the node.
//!
//! Liveness (Mission 05 §1.2): each side pings every second, whatever else it
//! sends, and **any bytes read count as alive** (a 1 MiB frame takes over 3 s
//! on a 300 KB/s link, so whole frames can't be the measure). Nothing read
//! for `SILENCE` closes the link, so a transfer pauses on the side that's
//! still there within about 3 s. Time the reader spends handing data to a
//! full transfer queue (this computer's disk) never counts as silence.

use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use tokio::io::{AsyncRead, AsyncWriteExt, BufWriter, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};

use crate::consts::ALPN;
use crate::model::Fingerprint;
use crate::proto::{self, ErrorCode, Frame, Hello, Msg};

pub type LinkId = u64;

/// Nothing at all read from the peer for this long closes the link (it
/// pings every `PING_EVERY`, so a live peer is never this quiet).
pub const SILENCE: Duration = Duration::from_millis(2500);
/// Each side pings this often.
pub const PING_EVERY: Duration = Duration::from_secs(1);
/// How often the watchdog looks.
const WATCH_EVERY: Duration = Duration::from_millis(100);
/// The watchdog this late means this computer stalled, not the link.
const STALLED_HERE: Duration = Duration::from_secs(1);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const EXPORTER_LABEL: &[u8] = b"EXPORTER-portal-pairing-v1";

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Microseconds since the first link, for "last heard from" stamps.
fn now_us() -> u64 {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

#[derive(Clone, Debug)]
pub struct LinkInfo {
    pub id: LinkId,
    /// The peer's hello.
    pub hello: Hello,
    /// Fingerprint of the certificate the peer presented in this session.
    pub peer_fp: Fingerprint,
    /// True if we opened the connection.
    pub dialer: bool,
    pub remote: SocketAddr,
    pub local: SocketAddr,
    /// TLS exporter secret bound to this session (pairing code input).
    pub exporter: [u8; 32],
}

#[derive(Debug)]
pub enum LinkEvent {
    Msg(LinkId, Msg),
    Closed(LinkId, String),
}

/// What the reader hands a running transfer.
#[derive(Debug)]
pub enum Inbound {
    Msg(Msg),
    Data(Vec<u8>),
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("{name} runs protocol version {theirs}")]
    Incompatible { theirs: u32, name: String, id: crate::model::DeviceId },
    #[error("the other side refused: {0:?} {1}")]
    Remote(ErrorCode, String),
    #[error("no reply in time")]
    Timeout,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Core(#[from] crate::Error),
    /// We turned it away ourselves: its device is disconnected here.
    #[error("disconnected here")]
    TurnedAway,
}

#[derive(Clone)]
pub struct Link {
    inner: Arc<Inner>,
}

struct Inner {
    info: LinkInfo,
    /// Urgent control messages: jump ahead of the ordered queue.
    ctrl: mpsc::Sender<Msg>,
    /// File data and the transfer messages that must stay in order with it,
    /// each with the `epoch` it was queued in.
    ordered: mpsc::Sender<(u64, Frame)>,
    /// Bumped by `discard_queued`: queued frames from an older epoch are dropped.
    epoch: AtomicU64,
    sink: Mutex<Option<mpsc::Sender<Inbound>>>,
    closed: watch::Sender<bool>,
    /// When any bytes last arrived from the peer (`now_us`).
    heard: AtomicU64,
    /// The reader is handing something on (a full transfer queue: our disk,
    /// not the network), so the quiet isn't the peer's.
    delivering: AtomicBool,
    /// The watchdog closed the link: nothing heard for `SILENCE`.
    silent: AtomicBool,
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Link({} {} {})", self.inner.info.id, self.inner.info.hello.name, self.inner.info.remote)
    }
}

impl Link {
    pub fn info(&self) -> &LinkInfo {
        &self.inner.info
    }

    pub fn id(&self) -> LinkId {
        self.inner.info.id
    }

    /// Queues an urgent control message (ahead of any queued file data).
    pub async fn send(&self, msg: Msg) -> crate::Result<()> {
        self.inner.ctrl.send(msg).await.map_err(|_| crate::Error::Protocol("link closed".into()))
    }

    /// Queues a control message without waiting; false if the queue is full or closed.
    pub fn try_send(&self, msg: Msg) -> bool {
        self.inner.ctrl.try_send(msg).is_ok()
    }

    /// Queues file data. Waits when the queue is full (backpressure).
    pub async fn send_data(&self, data: Vec<u8>) -> crate::Result<()> {
        let epoch = self.inner.epoch.load(Ordering::SeqCst);
        self.inner
            .ordered
            .send((epoch, Frame::Data(data)))
            .await
            .map_err(|_| crate::Error::Protocol("link closed".into()))
    }

    /// Queues a transfer message in order with file data (dirs, file, file_end, done).
    pub async fn send_ordered(&self, msg: Msg) -> crate::Result<()> {
        let epoch = self.inner.epoch.load(Ordering::SeqCst);
        self.inner
            .ordered
            .send((epoch, Frame::Control(msg)))
            .await
            .map_err(|_| crate::Error::Protocol("link closed".into()))
    }

    /// Drops file data and transfer messages still queued (a cancelled
    /// transfer's), so nothing of it follows the `cancel`.
    pub fn discard_queued(&self) {
        self.inner.epoch.fetch_add(1, Ordering::SeqCst);
    }

    /// Routes transfer messages and data to `sink` (None: back to the node).
    pub fn set_sink(&self, sink: Option<mpsc::Sender<Inbound>>) {
        *self.inner.sink.lock().unwrap_or_else(|e| e.into_inner()) = sink;
    }

    /// Says goodbye and closes the connection.
    pub fn close(&self) {
        let _ = self.inner.ctrl.try_send(Msg::Bye);
        self.inner.closed.send_replace(true);
    }

    pub fn is_closed(&self) -> bool {
        *self.inner.closed.borrow()
    }

    /// Something (a frame of any kind) arrived from the peer after `since`.
    pub fn heard_since(&self, since: Instant) -> bool {
        let since_us = now_us().saturating_sub(since.elapsed().as_micros() as u64);
        self.inner.heard.load(Ordering::Relaxed) > since_us
    }

    /// Resolves when the link has closed.
    pub async fn closed(&self) {
        let mut rx = self.inner.closed.subscribe();
        let _ = rx.wait_for(|c| *c).await;
    }
}

pub async fn accept_tls(tcp: TcpStream, config: Arc<rustls::ServerConfig>) -> crate::Result<TlsStream<TcpStream>> {
    let tls = tokio::time::timeout(HANDSHAKE_TIMEOUT, TlsAcceptor::from(config).accept(tcp))
        .await
        .map_err(|_| crate::Error::Protocol("TLS handshake timed out".into()))??;
    Ok(TlsStream::from(tls))
}

pub async fn connect_tls(tcp: TcpStream, config: Arc<rustls::ClientConfig>) -> crate::Result<TlsStream<TcpStream>> {
    let name = rustls::pki_types::ServerName::try_from(crate::tls::SERVER_NAME).expect("static name");
    let tls = tokio::time::timeout(HANDSHAKE_TIMEOUT, TlsConnector::from(config).connect(name, tcp))
        .await
        .map_err(|_| crate::Error::Protocol("TLS handshake timed out".into()))??;
    Ok(TlsStream::from(tls))
}

fn session_facts(stream: &TlsStream<TcpStream>) -> crate::Result<(Fingerprint, [u8; 32], SocketAddr, SocketAddr)> {
    let (tcp, common): (&TcpStream, &rustls::CommonState) = match stream {
        TlsStream::Client(s) => (s.get_ref().0, s.get_ref().1),
        TlsStream::Server(s) => (s.get_ref().0, s.get_ref().1),
    };
    if common.alpn_protocol() != Some(ALPN) {
        return Err(crate::Error::Protocol("peer is not speaking this app's protocol".into()));
    }
    let peer_fp =
        crate::tls::peer_fingerprint(common).ok_or_else(|| crate::Error::Protocol("no peer certificate".into()))?;
    let mut exporter = [0u8; 32];
    match stream {
        TlsStream::Client(s) => s.get_ref().1.export_keying_material(&mut exporter, EXPORTER_LABEL, None)?,
        TlsStream::Server(s) => s.get_ref().1.export_keying_material(&mut exporter, EXPORTER_LABEL, None)?,
    };
    Ok((peer_fp, exporter, tcp.peer_addr()?, tcp.local_addr()?))
}

/// The key the peer presented in this TLS session.
pub fn peer_fingerprint(stream: &TlsStream<TcpStream>) -> Option<Fingerprint> {
    let common: &rustls::CommonState = match stream {
        TlsStream::Client(s) => s.get_ref().1,
        TlsStream::Server(s) => s.get_ref().1,
    };
    crate::tls::peer_fingerprint(common)
}

/// Answers the peer's hello with an error instead of ours, and hangs up.
pub async fn refuse(stream: TlsStream<TcpStream>, code: ErrorCode, msg: &str) {
    let (_rd, wr) = tokio::io::split(stream);
    let mut wr = BufWriter::new(wr);
    let _ = proto::write_frame(&mut wr, &Frame::Control(Msg::Error { code, msg: msg.to_owned() })).await;
    let _ = wr.flush().await;
    let _ = wr.shutdown().await;
}

/// Exchanges hellos and starts the link's reader and writer tasks.
pub async fn handshake(
    stream: TlsStream<TcpStream>,
    dialer: bool,
    mine: Hello,
    events: mpsc::Sender<LinkEvent>,
) -> Result<Link, HandshakeError> {
    let (peer_fp, exporter, remote, local) = session_facts(&stream)?;
    let (mut rd, wr) = tokio::io::split(stream);
    let mut wr = BufWriter::with_capacity(256 * 1024, wr);

    proto::write_frame(&mut wr, &Frame::Control(Msg::Hello(mine))).await?;
    wr.flush().await?;
    let theirs = match tokio::time::timeout(HANDSHAKE_TIMEOUT, proto::read_frame(&mut rd)).await {
        Err(_) => return Err(HandshakeError::Timeout),
        Ok(Err(e)) => return Err(e.into()),
        Ok(Ok(Frame::Control(Msg::Hello(h)))) => h,
        Ok(Ok(Frame::Control(Msg::Error { code, msg }))) => return Err(HandshakeError::Remote(code, msg)),
        Ok(Ok(_)) => return Err(crate::Error::Protocol("expected hello".into()).into()),
    };
    if let Err(code) = proto::check_version(theirs.proto) {
        let msg = format!("this device runs protocol {}, you run {}", proto::PROTOCOL_VERSION, theirs.proto);
        let _ = proto::write_frame(&mut wr, &Frame::Control(Msg::Error { code, msg })).await;
        let _ = wr.flush().await;
        let _ = wr.shutdown().await;
        return Err(HandshakeError::Incompatible { theirs: theirs.proto, name: theirs.name, id: theirs.id });
    }

    let info = LinkInfo {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        hello: theirs,
        peer_fp,
        dialer,
        remote,
        local,
        exporter,
    };
    let (ctrl_tx, ctrl_rx) = mpsc::channel(256);
    let (ordered_tx, ordered_rx) = mpsc::channel(8);
    let (closed_tx, closed_rx) = watch::channel(false);
    let inner = Arc::new(Inner {
        info,
        ctrl: ctrl_tx,
        ordered: ordered_tx,
        epoch: AtomicU64::new(0),
        sink: Mutex::new(None),
        closed: closed_tx,
        heard: AtomicU64::new(now_us()),
        delivering: AtomicBool::new(false),
        silent: AtomicBool::new(false),
    });

    tokio::spawn(write_loop(wr, ctrl_rx, ordered_rx, closed_rx.clone(), inner.clone()));
    tokio::spawn(keep_alive(inner.clone(), closed_rx.clone()));
    let rd = Heard { inner: rd, link: inner.clone() };
    tokio::spawn(read_loop(rd, inner.clone(), events, closed_rx));
    Ok(Link { inner })
}

/// The read half, marking `heard` whenever any bytes arrive.
struct Heard {
    inner: tokio::io::ReadHalf<TlsStream<TcpStream>>,
    link: Arc<Inner>,
}

impl AsyncRead for Heard {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let r = Pin::new(&mut self.inner).poll_read(cx, buf);
        if matches!(r, Poll::Ready(Ok(()))) && buf.filled().len() > before {
            self.link.heard.store(now_us(), Ordering::Relaxed);
        }
        r
    }
}

/// Pings every `PING_EVERY` and closes the link after `SILENCE` with
/// nothing read (unless the reader is busy handing data on).
///
/// If this watchdog itself was held up (this computer stalled: a blocked
/// runtime, heavy load, waking from sleep), the quiet counts only from when
/// it woke: the bytes the peer sent meanwhile are waiting in the socket, and
/// the reader gets its turn before anything is called dead.
async fn keep_alive(inner: Arc<Inner>, mut closed: watch::Receiver<bool>) {
    let mut tick = tokio::time::interval(WATCH_EVERY);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut pinged: Option<Instant> = None;
    let mut last_tick = Instant::now();
    let mut awake_since = now_us();
    loop {
        tokio::select! {
            _ = closed.wait_for(|c| *c) => return,
            _ = tick.tick() => {}
        }
        // Over a second late: this computer stalled (a busy moment's
        // jitter is far shorter, and mustn't keep the clock from running).
        if last_tick.elapsed() > STALLED_HERE {
            awake_since = now_us();
        }
        last_tick = Instant::now();
        if pinged.is_none_or(|at| at.elapsed() >= PING_EVERY) {
            pinged = Some(Instant::now());
            let _ = inner.ctrl.try_send(Msg::Ping);
        }
        if inner.delivering.load(Ordering::Relaxed) {
            continue;
        }
        let since = inner.heard.load(Ordering::Relaxed).max(awake_since);
        if now_us().saturating_sub(since) >= SILENCE.as_micros() as u64 {
            inner.silent.store(true, Ordering::Relaxed);
            inner.closed.send_replace(true);
            return;
        }
    }
}

type Reader = Heard;
type Writer = BufWriter<tokio::io::WriteHalf<TlsStream<TcpStream>>>;

async fn write_loop(
    mut wr: Writer,
    mut ctrl: mpsc::Receiver<Msg>,
    mut ordered: mpsc::Receiver<(u64, Frame)>,
    mut closed: watch::Receiver<bool>,
    inner: Arc<Inner>,
) {
    let result: crate::Result<()> = async {
        loop {
            let frame = tokio::select! {
                biased;
                _ = closed.wait_for(|c| *c) => break,
                Some(msg) = ctrl.recv() => Some(Frame::Control(msg)),
                // From a transfer that was cancelled: dropped
                Some((epoch, frame)) = ordered.recv() => (epoch >= inner.epoch.load(Ordering::SeqCst)).then_some(frame),
                else => break,
            };
            if let Some(frame) = frame {
                proto::write_frame(&mut wr, &frame).await?;
            }
            if ctrl.is_empty() && ordered.is_empty() {
                wr.flush().await?;
            }
        }
        // Closing: send whatever control messages are queued (e.g. bye, cancel).
        while let Ok(msg) = ctrl.try_recv() {
            proto::write_frame(&mut wr, &Frame::Control(msg)).await?;
        }
        wr.flush().await?;
        wr.shutdown().await?;
        Ok(())
    }
    .await;
    if let Err(e) = result {
        tracing::debug!("link {} writer stopped: {e}", inner.info.id);
    }
    inner.closed.send_replace(true);
}

async fn read_loop(
    mut rd: Reader,
    inner: Arc<Inner>,
    events: mpsc::Sender<LinkEvent>,
    mut closed: watch::Receiver<bool>,
) {
    let id = inner.info.id;
    // Hands something on; the time it waits for room isn't the peer's quiet.
    async fn deliver<T>(inner: &Inner, to: &mpsc::Sender<T>, item: T) -> bool {
        inner.delivering.store(true, Ordering::Relaxed);
        let ok = to.send(item).await.is_ok();
        inner.heard.store(now_us(), Ordering::Relaxed);
        inner.delivering.store(false, Ordering::Relaxed);
        ok
    }
    let reason = loop {
        let frame = tokio::select! {
            _ = closed.wait_for(|c| *c) => break if inner.silent.load(Ordering::Relaxed) {
                format!("nothing heard for {:.1} s", SILENCE.as_secs_f32())
            } else {
                "closed here".to_owned()
            },
            r = proto::read_frame(&mut rd) => match r {
                Err(e) => break close_reason(&e),
                Ok(frame) => frame,
            },
        };
        let sink = inner.sink.lock().unwrap_or_else(|e| e.into_inner()).clone();
        match frame {
            Frame::Data(bytes) => match sink {
                Some(sink) => {
                    deliver(&inner, &sink, Inbound::Data(bytes)).await;
                }
                // The tail of a transfer that just ended here: already in
                // flight when it ended, so it's dropped, not an error.
                None => tracing::debug!("link {id}: {} bytes of file data outside a transfer, dropped", bytes.len()),
            },
            Frame::Control(Msg::Ping) => {
                let _ = inner.ctrl.try_send(Msg::Pong);
            }
            Frame::Control(Msg::Pong | Msg::Unknown) => {}
            Frame::Control(Msg::Bye) => break "the other side closed the connection".to_owned(),
            Frame::Control(msg) => match sink {
                Some(sink) if msg.transfer_id().is_some() => {
                    deliver(&inner, &sink, Inbound::Msg(msg)).await;
                }
                _ => {
                    if !deliver(&inner, &events, LinkEvent::Msg(id, msg)).await {
                        break "node stopped".to_owned();
                    }
                }
            },
        }
    };
    inner.closed.send_replace(true);
    // Wake a transfer waiting on this link.
    inner.sink.lock().unwrap_or_else(|e| e.into_inner()).take();
    let _ = events.send(LinkEvent::Closed(id, reason)).await;
}

/// A short, readable reason for the log instead of raw I/O error text.
fn close_reason(e: &crate::Error) -> String {
    use std::io::ErrorKind::*;
    match e {
        crate::Error::Io(io) => match io.kind() {
            UnexpectedEof => "connection dropped".into(),
            ConnectionReset | ConnectionAborted | BrokenPipe => "connection reset".into(),
            TimedOut => "timed out".into(),
            _ => io.to_string(),
        },
        other => other.to_string(),
    }
}
