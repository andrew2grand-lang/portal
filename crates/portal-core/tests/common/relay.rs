//! Stand-ins for a real network between two cores on one machine.
//!
//! - `Relay`: a TCP relay that behaves like so-so Wi-Fi: extra latency, a
//!   bandwidth cap, and "dropping out" (nothing passes and nothing closes,
//!   which is what a dead Wi-Fi link looks like to TCP).
//! - `Tarpit`: an address that accepts connections and never says a word
//!   (an old IP address now owned by some other device).

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};

#[derive(Clone, Copy, Debug, Default)]
pub struct Shape {
    /// Added to everything, each way.
    pub delay: Duration,
    /// Bytes per second each way (0: unlimited).
    pub rate: u64,
}

pub struct Relay {
    pub addr: SocketAddr,
    down: watch::Sender<bool>,
    stop: watch::Sender<bool>,
}

impl Relay {
    pub async fn start(target: SocketAddr, shape: Shape) -> Relay {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (down, down_rx) = watch::channel(false);
        let (stop, stop_rx) = watch::channel(false);
        let mut stopped = stop_rx.clone();
        tokio::spawn(async move {
            loop {
                let client = tokio::select! {
                    _ = until(&mut stopped, true) => return,
                    r = listener.accept() => match r {
                        Ok((c, _)) => c,
                        Err(_) => return,
                    },
                };
                let (down, stop) = (down_rx.clone(), stop_rx.clone());
                tokio::spawn(async move {
                    let Ok(server) = TcpStream::connect(target).await else { return };
                    let _ = (client.set_nodelay(true), server.set_nodelay(true));
                    let (cr, cw) = client.into_split();
                    let (sr, sw) = server.into_split();
                    tokio::join!(pipe(cr, sw, shape, down.clone(), stop.clone()), pipe(sr, cw, shape, down, stop));
                });
            }
        });
        Relay { addr, down, stop }
    }

    /// The link goes dead: nothing passes either way, nothing is closed.
    pub fn drop_out(&self) {
        self.down.send_replace(true);
    }

    /// The network is back: traffic passes again.
    pub fn come_back(&self) {
        self.down.send_replace(false);
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

async fn pipe(
    mut from: OwnedReadHalf,
    mut to: OwnedWriteHalf,
    shape: Shape,
    down: watch::Receiver<bool>,
    stop: watch::Receiver<bool>,
) {
    let (tx, mut rx) = mpsc::channel::<(Instant, Vec<u8>)>(64);
    let (mut down_r, mut down_w, mut stop_r, mut stop_w) = (down.clone(), down, stop.clone(), stop);
    let reader = async move {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            tokio::select! {
                _ = until(&mut stop_r, true) => return,
                r = from.read(&mut buf) => match r {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        if tx.send((Instant::now(), buf[..n].to_vec())).await.is_err() {
                            return;
                        }
                    }
                },
            }
            // While dropped out, stop reading so the sender's TCP stalls.
            if !until(&mut down_r, false).await {
                return;
            }
        }
    };
    let writer = async move {
        let mut next_free = Instant::now();
        loop {
            let (at, data) = tokio::select! {
                _ = until(&mut stop_w, true) => return,
                m = rx.recv() => match m {
                    Some(m) => m,
                    None => break,
                },
            };
            // Paced from the schedule, not from "now", so timer overshoot
            // doesn't add up.
            let due = (at + shape.delay).max(next_free);
            tokio::time::sleep_until(tokio::time::Instant::from_std(due)).await;
            // Nothing passes while the link is down (held, as TCP would).
            if !until(&mut down_w, false).await || to.write_all(&data).await.is_err() {
                return;
            }
            if shape.rate > 0 {
                next_free = due + Duration::from_secs_f64(data.len() as f64 / shape.rate as f64);
            }
        }
        let _ = to.shutdown().await;
    };
    tokio::join!(reader, writer);
}

/// Waits until the flag is `want`; false if the sender went away.
async fn until(rx: &mut watch::Receiver<bool>, want: bool) -> bool {
    rx.wait_for(|v| *v == want).await.is_ok()
}

pub struct Tarpit {
    pub addr: SocketAddr,
    task: tokio::task::JoinHandle<()>,
}

impl Tarpit {
    pub async fn start() -> Tarpit {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((s, _)) = listener.accept().await {
                held.push(s);
            }
        });
        Tarpit { addr, task }
    }
}

impl Drop for Tarpit {
    fn drop(&mut self) {
        self.task.abort();
    }
}
