//! Sender pipeline: a reader thread hashes files 1 MiB at a time; a pump
//! task streams them; the control loop watches for cancel, progress reports
//! and the receiver's final report.
//!
//! A resumed transfer starts at the folder entry, file and byte the receiver
//! asked for. The skipped part of that file is read and hashed again (never
//! sent), so the receiver still checks the whole file; if the file changed
//! since it was chosen, it's sent again from the start, with a note.

use std::io::{Read, Seek};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch};

use super::plan::{Planned, PlannedFile};
use super::{CHUNK, Failure, Progress};
use crate::net::link::{Inbound, Link};
use crate::proto::{B32, CancelReason, Msg, Mtime, Summary};

enum ReadItem {
    Start {
        idx: u64,
        rel: Vec<String>,
        size: u64,
        mtime: Option<Mtime>,
        from: u64,
    },
    Chunk(Vec<u8>),
    End {
        idx: u64,
        hash: [u8; 32],
    },
    /// Still re-reading the part the receiver has (this many bytes so far).
    Checking(u64),
    Failed(Failure),
}

/// Where a (resumed) transfer starts: folder entries, file index, byte, and
/// the receiver's hash of that file's first `offset` bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Start {
    pub dirs: u64,
    pub file: u64,
    pub offset: u64,
    pub prefix: Option<B32>,
}

/// Fault injection for tests.
#[derive(Clone, Debug, Default)]
pub struct Faults {
    /// Close the link once, after this many bytes have been sent.
    pub cut_at: Option<(u64, Arc<AtomicBool>)>,
    /// Send at most this many bytes per second.
    pub rate: Option<u64>,
    /// Re-read the part the receiver already has at most this many bytes per second.
    pub hash_rate: Option<u64>,
}

pub struct SendCtx {
    pub link: Link,
    pub planned: Planned,
    pub start: Start,
    pub inbound: mpsc::Receiver<Inbound>,
    pub cancel: watch::Receiver<bool>,
    pub progress: Progress,
    /// "this Mac" / "this PC", for notes.
    pub here: &'static str,
    pub faults: Faults,
    /// Hearing nothing from the receiver for this long pauses the transfer.
    pub stall: Duration,
}

pub async fn run(ctx: SendCtx) -> Result<Summary, Failure> {
    let SendCtx { link, planned, start, mut inbound, mut cancel, progress, here, faults, stall } = ctx;
    let tid = planned.offer.tid;

    let (tx, rx) = mpsc::channel::<ReadItem>(8);
    let notes = progress.clone();
    let (hash_rate, keepalive) = (faults.hash_rate, stall / 4);
    std::thread::spawn(move || read_all(planned.files, start, here, keepalive, hash_rate, &notes, tx));

    let pump_link = link.clone();
    let pump_progress = progress.clone();
    let dirs: Vec<_> = planned.dirs.into_iter().skip(start.dirs as usize).collect();
    let mut pump = tokio::spawn(async move { pump(pump_link, tid, dirs, rx, pump_progress, faults).await });
    let mut pumped = false;

    loop {
        tokio::select! {
            biased;
            _ = super::cancelled(&mut cancel) => {
                // Nothing still queued may follow the cancel.
                pump.abort();
                link.discard_queued();
                let _ = link.send(Msg::Cancel { tid, reason: CancelReason::User }).await;
                return Err(Failure::CancelledHere);
            }
            r = &mut pump, if !pumped => {
                pumped = true;
                match r {
                    Ok(Ok(())) => {}
                    Ok(Err(f)) => {
                        if f != Failure::Lost {
                            link.discard_queued();
                            let reason = CancelReason::Error { code: f.code().into(), msg: f.detail() };
                            let _ = link.send(Msg::Cancel { tid, reason }).await;
                        }
                        return Err(f);
                    }
                    Err(_) => return Err(Failure::Lost),
                }
            }
            m = tokio::time::timeout(stall, inbound.recv()) => match m {
                Err(_) | Ok(None) => {
                    pump.abort();
                    return Err(Failure::Lost);
                }
                // What the receiver has: speed and time left are measured from it.
                Ok(Some(Inbound::Msg(Msg::Progress { bytes, .. }))) => progress.set_acked(bytes),
                Ok(Some(Inbound::Msg(Msg::Complete { summary, .. }))) if pumped => return Ok(summary),
                Ok(Some(Inbound::Msg(Msg::Cancel { reason, .. }))) => {
                    pump.abort();
                    link.discard_queued();
                    return Err(match reason {
                        CancelReason::User => Failure::CancelledThere { code: None, msg: String::new() },
                        CancelReason::Error { code, msg } => Failure::CancelledThere { code: Some(code), msg },
                    });
                }
                Ok(Some(other)) => {
                    pump.abort();
                    link.discard_queued();
                    return Err(Failure::Protocol(format!("unexpected while sending: {other:?}")));
                }
            },
        }
    }
}

async fn pump(
    link: Link,
    tid: u64,
    dirs: Vec<crate::proto::DirEntry>,
    mut rx: mpsc::Receiver<ReadItem>,
    progress: Progress,
    faults: Faults,
) -> Result<(), Failure> {
    let lost = |_| Failure::Lost;
    for chunk in dirs.chunks(500) {
        link.send_ordered(Msg::Dirs { tid, entries: chunk.to_vec() }).await.map_err(lost)?;
    }
    let began = Instant::now();
    let mut sent = 0u64;
    while let Some(item) = rx.recv().await {
        match item {
            ReadItem::Start { idx, rel, size, mtime, from } => {
                progress.set_current(rel.last().map(String::as_str).unwrap_or_default());
                link.send_ordered(Msg::File { tid, idx, path: rel, size, mtime, from }).await.map_err(lost)?;
            }
            ReadItem::Chunk(bytes) => {
                let n = bytes.len() as u64;
                link.send_data(bytes).await.map_err(lost)?;
                progress.add(n);
                sent += n;
                if let Some(rate) = faults.rate.filter(|r| *r > 0) {
                    let due = Duration::from_secs_f64(sent as f64 / rate as f64);
                    if let Some(wait) = due.checked_sub(began.elapsed()) {
                        tokio::time::sleep(wait).await;
                    }
                }
                if let Some((at, fired)) = &faults.cut_at
                    && progress.bytes() >= *at
                    && !fired.swap(true, Ordering::SeqCst)
                {
                    link.close();
                }
            }
            ReadItem::End { idx, hash } => {
                link.send_ordered(Msg::FileEnd { tid, idx, blake3: B32(hash) }).await.map_err(lost)?;
                progress.files_done.fetch_add(1, Ordering::Relaxed);
            }
            ReadItem::Checking(bytes) => {
                link.send_ordered(Msg::Progress { tid, bytes }).await.map_err(lost)?;
            }
            ReadItem::Failed(f) => return Err(f),
        }
    }
    link.send_ordered(Msg::Done { tid }).await.map_err(lost)
}

fn mtime_now(src: &std::path::Path) -> Option<(u64, Option<Mtime>)> {
    let meta = std::fs::metadata(src).ok()?;
    Some((meta.len(), super::plan::mtime_of(&meta)))
}

/// Blocking: reads and hashes each file once, in 1 MiB chunks, from `start`.
fn read_all(
    files: Vec<PlannedFile>,
    start: Start,
    here: &str,
    keepalive: Duration,
    hash_rate: Option<u64>,
    notes: &Progress,
    tx: mpsc::Sender<ReadItem>,
) {
    for (idx, f) in files.into_iter().enumerate().skip(start.file as usize) {
        let idx = idx as u64;
        let shown = f.rel.join("/");
        // Send the file as it is now; say so if it changed since it was chosen.
        let (size, mtime) = match mtime_now(&f.src) {
            Some(now) => now,
            None => {
                let _ = tx.blocking_send(ReadItem::Failed(Failure::ReadFailed(shown)));
                return;
            }
        };
        let changed = size != f.size || mtime != f.mtime;
        let mut from = if idx == start.file { start.offset } else { 0 };
        let again = || {
            notes.note(format!(
                "“{shown}” changed on {here} while the transfer was paused, so it was sent again from the start."
            ))
        };
        if changed && from > 0 {
            again();
            from = 0;
        } else if changed {
            tracing::info!("{shown} changed since it was chosen; sending it as it is now");
        }
        let from = from.min(size);
        let result = (|| -> Result<[u8; 32], Failure> {
            let mut file = std::fs::File::open(&f.src).map_err(|e| Failure::ReadFailed(format!("{shown}: {e}")))?;
            let changed_while = || Failure::ReadFailed(format!("{shown} changed while sending"));
            // The part the receiver already has: hashed again, not sent. It
            // must be what the receiver has, or the file starts again.
            let mut hasher = blake3::Hasher::new();
            let mut buf = vec![0u8; CHUNK];
            let (began, mut posted) = (Instant::now(), Instant::now());
            let mut done = 0u64;
            while done < from {
                let n = (from - done).min(CHUNK as u64) as usize;
                file.read_exact(&mut buf[..n]).map_err(|_| changed_while())?;
                hasher.update(&buf[..n]);
                done += n as u64;
                if let Some(rate) = hash_rate.filter(|r| *r > 0) {
                    let due = Duration::from_secs_f64(done as f64 / rate as f64);
                    if let Some(wait) = due.checked_sub(began.elapsed()) {
                        std::thread::sleep(wait);
                    }
                }
                if posted.elapsed() >= keepalive {
                    posted = Instant::now();
                    if tx.blocking_send(ReadItem::Checking(done)).is_err() {
                        return Err(Failure::CancelledHere);
                    }
                }
            }
            let mut from = from;
            if from > 0 && start.prefix.is_some_and(|p| hasher.finalize().as_bytes() != &p.0) {
                again();
                from = 0;
                hasher = blake3::Hasher::new();
                file.rewind().map_err(|_| changed_while())?;
            }
            debug_assert_eq!(file.stream_position().ok(), Some(from));
            if tx.blocking_send(ReadItem::Start { idx, rel: f.rel.clone(), size, mtime, from }).is_err() {
                return Err(Failure::CancelledHere);
            }
            let mut left = size - from;
            while left > 0 {
                let n = left.min(CHUNK as u64) as usize;
                let mut buf = vec![0u8; n];
                file.read_exact(&mut buf).map_err(|_| changed_while())?;
                hasher.update(&buf);
                left -= n as u64;
                if tx.blocking_send(ReadItem::Chunk(buf)).is_err() {
                    return Err(Failure::CancelledHere);
                }
            }
            Ok(*hasher.finalize().as_bytes())
        })();
        let item = match result {
            Ok(hash) => ReadItem::End { idx, hash },
            Err(Failure::CancelledHere) => return,
            Err(f) => ReadItem::Failed(f),
        };
        if tx.blocking_send(item).is_err() {
            return;
        }
    }
}
