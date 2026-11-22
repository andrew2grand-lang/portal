//! Receiver pipeline: validate every path, write into a hidden staging
//! folder on the same volume, verify BLAKE3 per file, and only when the whole
//! transfer checks out move each item into place without replacing anything.
//!
//! What has arrived is saved every second by the writer thread (so it never
//! records bytes that aren't on disk): when the connection drops, the
//! transfer pauses with its staging folder kept, and a later run picks up
//! from that state.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tokio::sync::{mpsc, watch};

use super::journal::{self, InState, Partial, Saved};
use super::{CHECKPOINT_EVERY, Failure, LISTED, Progress, REPORT_EVERY};
use crate::fsafe::{self, Rename, Target};
use crate::net::link::{Inbound, Link};
use crate::proto::{B32, CancelReason, Msg, Mtime, RenameNote, Summary};

pub struct RecvCtx {
    pub link: Link,
    pub inbound: mpsc::Receiver<Inbound>,
    pub cancel: watch::Receiver<bool>,
    pub progress: Progress,
    /// Where the state is saved (`<app data>/transfers/<tid>.json`).
    pub journal: PathBuf,
    /// The transfer so far: fresh, or restored by `restore` to resume.
    pub state: InState,
    /// The partial file's hash so far, when resuming inside a file.
    pub hasher: Option<blake3::Hasher>,
    /// Fault injection for tests: writing beyond this many bytes fails as "disk full".
    pub write_limit: Option<u64>,
    /// Hearing nothing from the sender for this long pauses the transfer.
    pub stall: std::time::Duration,
}

#[derive(Debug, Default)]
pub struct Received {
    pub summary: Summary,
    /// The folder the files were saved in.
    pub saved_in: PathBuf,
    /// Final top-level paths, in order.
    pub saved: Vec<PathBuf>,
    pub renames: Vec<Rename>,
    /// Top-level names that already existed: (wanted, saved as).
    pub numbered: Vec<(String, String)>,
    /// Saved paths longer than 260 characters (Windows).
    pub long: Vec<PathBuf>,
}

enum WriteCmd {
    Dir(PathBuf),
    Open {
        path: PathBuf,
    },
    /// A partial file from before the pause: keep its first `keep` bytes.
    Reopen {
        path: PathBuf,
        keep: u64,
        hasher: Box<blake3::Hasher>,
    },
    Data(Vec<u8>),
    Close {
        expect: [u8; 32],
        name: String,
        mtime: Option<Mtime>,
    },
    /// Save the transfer's state; the writer adds the open file's bytes and hash.
    Checkpoint(Box<InState>),
    Finish,
}

enum WriteEvent {
    Closed,
    Failed(Failure),
    Finished,
}

pub async fn run(ctx: RecvCtx) -> Result<Received, Failure> {
    let staging = ctx.state.staging.clone();
    let abort = Arc::new(AtomicBool::new(false));
    let (wtx, wrx) = mpsc::channel::<WriteCmd>(16);
    let (etx, erx) = mpsc::channel::<WriteEvent>(64);
    let writer = {
        let (abort, limit, journal) = (abort.clone(), ctx.write_limit, ctx.journal.clone());
        std::thread::spawn(move || writer(wrx, etx, limit, &journal, &abort))
    };
    let result = receive(ctx, wtx, erx).await;
    match &result {
        // Paused: the writer finishes what's queued and saves the state.
        Err(Failure::Lost) => {
            let _ = tokio::task::spawn_blocking(move || writer.join()).await;
        }
        Err(_) => {
            // Stop the writer before deleting anything, or a queued command
            // could recreate the staging folder after it's gone.
            abort.store(true, Ordering::SeqCst);
            let _ = tokio::task::spawn_blocking(move || writer.join()).await;
            cleanup(&staging);
        }
        Ok(_) => {}
    }
    result
}

/// Removes a transfer's staging folder, and `.seamdrop-incoming` if it's empty.
pub fn cleanup(staging: &Path) {
    let _ = std::fs::remove_dir_all(staging);
    if let Some(parent) = staging.parent() {
        let _ = std::fs::remove_dir(parent);
    }
}

/// Before resuming: checks the partial file still holds exactly the bytes
/// that were saved (re-hashing them), and returns the hash to continue with.
/// A partial file that changed on disk, or a staging folder that's gone, is
/// started again; the returned note says so.
pub fn restore(state: &mut InState) -> (Option<blake3::Hasher>, Option<String>) {
    if !state.staging.is_dir() {
        let note = (state.bytes() > 0).then(|| {
            "The part that had arrived was gone from the receive folder, so it’s being received again.".to_owned()
        });
        state.namer = fsafe::Namer::new(state.target);
        state.dir_times.clear();
        (state.dirs, state.done, state.bytes_done, state.partial) = (0, 0, 0, None);
        let _ = std::fs::create_dir_all(&state.staging);
        return (None, note);
    }
    let Some(p) = state.partial.as_mut().filter(|p| p.written > 0) else { return (None, None) };
    let path = join(&state.staging, &p.local);
    let verified = (|| -> std::io::Result<Option<blake3::Hasher>> {
        let mut file = std::fs::File::open(&path)?;
        if file.metadata()?.len() < p.written {
            return Ok(None);
        }
        let mut hasher = blake3::Hasher::new();
        let mut left = p.written;
        let mut buf = vec![0u8; super::CHUNK];
        while left > 0 {
            let n = left.min(buf.len() as u64) as usize;
            file.read_exact(&mut buf[..n])?;
            hasher.update(&buf[..n]);
            left -= n as u64;
        }
        Ok((hasher.finalize().as_bytes() == &p.hash.0).then_some(hasher))
    })();
    match verified {
        Ok(Some(hasher)) => (Some(hasher), None),
        _ => {
            let name = p.wire.join("/");
            p.written = 0;
            (None, Some(format!("“{name}” changed on disk while paused, so it’s being received again from the start.")))
        }
    }
}

/// The file being received: index, size, bytes so far, display name, time,
/// and its sender and local paths (for checkpoints).
struct Current {
    idx: u64,
    size: u64,
    got: u64,
    name: String,
    mtime: Option<Mtime>,
    wire: Vec<String>,
    local: Vec<String>,
}

async fn receive(
    ctx: RecvCtx,
    wtx: mpsc::Sender<WriteCmd>,
    mut erx: mpsc::Receiver<WriteEvent>,
) -> Result<Received, Failure> {
    let RecvCtx { link, mut inbound, mut cancel, progress, state, hasher, stall, .. } = ctx;
    let tid = state.tid;
    let keep_dates = state.keep_dates;
    let (offer, staging, receive_dir, target) =
        (state.offer.clone(), state.staging.clone(), state.receive_dir.clone(), state.target);

    let mut namer = state.namer.clone();
    let mut dir_times = state.dir_times.clone();
    let mut dirs = state.dirs;
    let mut current: Option<Current> = None;
    // The partial file a resumed transfer continues, with its hash so far.
    let mut resumed: Option<(Partial, Option<blake3::Hasher>)> = state.partial.clone().map(|p| (p, hasher));
    let mut next_idx = state.done;
    let mut files = state.done;
    let mut bytes = state.bytes_done;
    let mut last_report = Instant::now();
    let mut last_checkpoint = Instant::now();

    let fail = |link: &Link, f: Failure| {
        let reason = match &f {
            Failure::CancelledHere => CancelReason::User,
            other => CancelReason::Error { code: other.code().into(), msg: other.detail() },
        };
        link.try_send(Msg::Cancel { tid, reason });
        Err(f)
    };
    let protocol = |what: &str| Failure::Protocol(what.to_owned());
    // What has arrived, for the writer to save (it adds the open file's bytes).
    // A partial file still waiting for the sender to continue it is kept as
    // it was, or the next resume would start it again under a new name.
    let checkpoint = |namer: &fsafe::Namer,
                      dir_times: &[(Vec<String>, Mtime)],
                      dirs: u64,
                      current: &Option<Current>,
                      resumed: &Option<(Partial, Option<blake3::Hasher>)>,
                      next_idx: u64,
                      bytes: u64| {
        let mut s = state.clone();
        s.namer = namer.clone();
        s.dir_times = dir_times.to_vec();
        s.dirs = dirs;
        s.done = next_idx - u64::from(current.is_some());
        s.bytes_done = bytes - current.as_ref().map_or(0, |c| c.got);
        s.partial = match current {
            Some(c) => Some(Partial {
                idx: c.idx,
                wire: c.wire.clone(),
                local: c.local.clone(),
                size: c.size,
                mtime: c.mtime,
                written: 0,
                hash: B32([0; 32]),
            }),
            None => resumed.as_ref().map(|(p, _)| p.clone()),
        };
        WriteCmd::Checkpoint(Box::new(s))
    };

    loop {
        tokio::select! {
            biased;
            _ = super::cancelled(&mut cancel) => return fail(&link, Failure::CancelledHere),
            Some(ev) = erx.recv() => match ev {
                WriteEvent::Closed => files += 1,
                WriteEvent::Failed(f) => return fail(&link, f),
                WriteEvent::Finished => {}
            },
            m = tokio::time::timeout(stall, inbound.recv()) => {
                let msg = match m {
                    Err(_) | Ok(None) => {
                        // Paused: save where it got to (the writer drains first).
                        let _ = wtx.send(checkpoint(&namer, &dir_times, dirs, &current, &resumed, next_idx, bytes)).await;
                        return Err(Failure::Lost);
                    }
                    Ok(Some(m)) => m,
                };
                match msg {
                    Inbound::Msg(Msg::Dirs { entries, .. }) => {
                        progress.started.store(true, Ordering::Relaxed);
                        for e in entries {
                            if let Err(err) = fsafe::validate_components(&e.path) {
                                return fail(&link, protocol(&format!("refused folder path: {err}")));
                            }
                            let local = namer.map_dir(&e.path);
                            // Dated on arrival unless "Keep original dates" (Mission 05 §4)
                            if let Some(t) = e.mtime.filter(|_| keep_dates) {
                                dir_times.push((local.clone(), t));
                            }
                            dirs += 1;
                            let _ = wtx.send(WriteCmd::Dir(join(&staging, &local))).await;
                        }
                    }
                    Inbound::Msg(Msg::File { idx, path, size, mtime, from, .. }) => {
                        progress.started.store(true, Ordering::Relaxed);
                        if current.is_some() || idx != next_idx {
                            return fail(&link, protocol("file out of order"));
                        }
                        if let Err(err) = fsafe::validate_components(&path) {
                            return fail(&link, protocol(&format!("refused file path: {err}")));
                        }
                        let name = path.join("/");
                        progress.set_current(path.last().map(String::as_str).unwrap_or_default());
                        let local = match resumed.take() {
                            // The file that was arriving when it paused: same place.
                            Some((p, hasher)) if p.idx == idx => {
                                let keep = if from > 0 && from == p.written { from } else { 0 };
                                if from != keep {
                                    return fail(&link, protocol("resumed at the wrong place"));
                                }
                                let hasher = match hasher.filter(|_| keep > 0) {
                                    Some(h) => h,
                                    None => blake3::Hasher::new(),
                                };
                                if keep == 0 && p.written > 0 {
                                    // Started again (it changed): its bytes don't count.
                                    progress.set(progress.bytes().saturating_sub(p.written));
                                }
                                let path = join(&staging, &p.local);
                                let _ = wtx.send(WriteCmd::Reopen { path, keep, hasher: Box::new(hasher) }).await;
                                bytes += keep;
                                p.local
                            }
                            other => {
                                if from != 0 || other.is_some() {
                                    return fail(&link, protocol("resumed at the wrong place"));
                                }
                                let local = namer.map_file(&path);
                                let _ = wtx.send(WriteCmd::Open { path: join(&staging, &local) }).await;
                                local
                            }
                        };
                        current = Some(Current { idx, size, got: from, name, mtime, wire: path, local });
                        next_idx += 1;
                    }
                    Inbound::Data(chunk) => {
                        let Some(c) = current.as_mut() else {
                            return fail(&link, protocol("data outside a file"));
                        };
                        c.got += chunk.len() as u64;
                        if c.got > c.size {
                            return fail(&link, protocol("more data than announced"));
                        }
                        bytes += chunk.len() as u64;
                        progress.add(chunk.len() as u64);
                        let _ = wtx.send(WriteCmd::Data(chunk)).await;
                        if last_report.elapsed() >= REPORT_EVERY {
                            last_report = Instant::now();
                            link.try_send(Msg::Progress { tid, bytes });
                        }
                        if last_checkpoint.elapsed() >= CHECKPOINT_EVERY {
                            last_checkpoint = Instant::now();
                            let _ = wtx.send(checkpoint(&namer, &dir_times, dirs, &current, &resumed, next_idx, bytes)).await;
                        }
                    }
                    Inbound::Msg(Msg::FileEnd { idx, blake3, .. }) => {
                        let Some(c) = current.take() else {
                            return fail(&link, protocol("file end without a file"));
                        };
                        if c.idx != idx || c.got != c.size {
                            return fail(&link, protocol("file ended early"));
                        }
                        let mtime = c.mtime.filter(|_| keep_dates);
                        let _ = wtx.send(WriteCmd::Close { expect: blake3.0, name: c.name, mtime }).await;
                        // Many small files: report here too (data alone may be too short to).
                        if last_report.elapsed() >= REPORT_EVERY {
                            last_report = Instant::now();
                            link.try_send(Msg::Progress { tid, bytes });
                        }
                        if last_checkpoint.elapsed() >= CHECKPOINT_EVERY {
                            last_checkpoint = Instant::now();
                            let _ = wtx.send(checkpoint(&namer, &dir_times, dirs, &current, &resumed, next_idx, bytes)).await;
                        }
                    }
                    Inbound::Msg(Msg::Done { .. }) => {
                        if current.is_some() || next_idx != offer.file_count {
                            return fail(&link, protocol("transfer ended early"));
                        }
                        // Everything arrived: the sender stops measuring speed
                        // here while this side checks and saves.
                        link.try_send(Msg::Progress { tid, bytes });
                        let _ = wtx.send(WriteCmd::Finish).await;
                        // Wait for every file to be flushed and verified.
                        loop {
                            match erx.recv().await {
                                Some(WriteEvent::Closed) => files += 1,
                                Some(WriteEvent::Finished) => break,
                                Some(WriteEvent::Failed(f)) => return fail(&link, f),
                                None => return fail(&link, Failure::Io("writer stopped".into())),
                            }
                        }
                        let out = tokio::task::spawn_blocking({
                            let (staging, receive_dir) = (staging.clone(), receive_dir.clone());
                            let top = namer.top.clone();
                            move || finalize(&staging, &receive_dir, &top, &mut dir_times, target)
                        })
                        .await
                        .map_err(|_| Failure::Io("finishing".into()));
                        let (saved, numbered) = match out {
                            Ok(Ok(v)) => v,
                            Ok(Err(f)) | Err(f) => return fail(&link, f),
                        };
                        // Paths over 260 characters: some Windows apps can't open them.
                        let long = if target == Target::Windows { long_paths(&saved) } else { Vec::new() };
                        let long_paths = long.len() as u64;
                        let summary = Summary {
                            files,
                            bytes,
                            renamed: namer.renames.len() as u64,
                            skipped: offer.skipped.len() as u64,
                            long_paths,
                            saved_in: Some(receive_dir.display().to_string()),
                            renames: namer
                                .renames
                                .iter()
                                .take(LISTED)
                                .map(|r| RenameNote { from: r.from.clone(), to: r.to.clone(), reason: r.reason.clone() })
                                .collect(),
                        };
                        link.try_send(Msg::Complete { tid, summary: summary.clone() });
                        return Ok(Received { summary, saved_in: receive_dir.clone(), saved, renames: namer.renames, numbered, long });
                    }
                    // The sender is re-reading what's here: answer, so it
                    // doesn't take the quiet for a stall either.
                    Inbound::Msg(Msg::Progress { .. }) => {
                        link.try_send(Msg::Progress { tid, bytes });
                    }
                    Inbound::Msg(Msg::Cancel { reason, .. }) => {
                        return Err(match reason {
                            CancelReason::User => Failure::CancelledThere { code: None, msg: String::new() },
                            CancelReason::Error { code, msg } => Failure::CancelledThere { code: Some(code), msg },
                        });
                    }
                    Inbound::Msg(_) => {}
                }
            }
        }
    }
}

fn join(base: &Path, parts: &[String]) -> PathBuf {
    let mut p = base.to_path_buf();
    for part in parts {
        p.push(part);
    }
    p
}

/// The file being written: handle, hash so far, path, bytes in it.
struct OpenFile {
    file: std::fs::File,
    hasher: blake3::Hasher,
    path: PathBuf,
    written: u64,
}

/// Blocking: writes and hashes, one file at a time, and saves checkpoints.
/// Stops at once when `abort` is set.
fn writer(
    mut rx: mpsc::Receiver<WriteCmd>,
    tx: mpsc::Sender<WriteEvent>,
    limit: Option<u64>,
    journal: &Path,
    abort: &AtomicBool,
) {
    let mut written = 0u64;
    let mut open: Option<OpenFile> = None;
    // Small files closed since the last checkpoint and not yet flushed: the
    // checkpoint flushes them before it counts them as done (a power cut
    // could otherwise leave a "done" file empty, and it's never checked again).
    let mut unsynced: Vec<PathBuf> = Vec::new();
    while let Some(cmd) = rx.blocking_recv() {
        if abort.load(Ordering::SeqCst) {
            return;
        }
        let step: Result<Option<WriteEvent>, Failure> = (|| match cmd {
            WriteCmd::Dir(path) => {
                std::fs::create_dir_all(&path).map_err(|e| Failure::from_io(&e, "creating a folder"))?;
                Ok(None)
            }
            WriteCmd::Open { path } => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| Failure::from_io(&e, "creating a folder"))?;
                }
                // Staging is this transfer's own folder: a file already there
                // is from before a crash that the saved state doesn't know of.
                let file = std::fs::File::create(&path).map_err(|e| Failure::from_io(&e, "creating a file"))?;
                open = Some(OpenFile { file, hasher: blake3::Hasher::new(), path, written: 0 });
                Ok(None)
            }
            WriteCmd::Reopen { path, keep, hasher } => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| Failure::from_io(&e, "creating a folder"))?;
                }
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path)
                    .map_err(|e| Failure::from_io(&e, "opening a file"))?;
                file.set_len(keep).map_err(|e| Failure::from_io(&e, "opening a file"))?;
                file.seek(SeekFrom::Start(keep)).map_err(|e| Failure::from_io(&e, "opening a file"))?;
                open = Some(OpenFile { file, hasher: *hasher, path, written: keep });
                Ok(None)
            }
            WriteCmd::Data(chunk) => {
                let f = open.as_mut().ok_or_else(|| Failure::Protocol("data without a file".into()))?;
                if limit.is_some_and(|l| written + chunk.len() as u64 > l) {
                    return Err(Failure::DiskFull);
                }
                f.file.write_all(&chunk).map_err(|e| Failure::from_io(&e, "writing"))?;
                f.hasher.update(&chunk);
                f.written += chunk.len() as u64;
                written += chunk.len() as u64;
                Ok(None)
            }
            WriteCmd::Close { expect, name, mtime } => {
                let OpenFile { file, hasher, path, .. } =
                    open.take().ok_or_else(|| Failure::Protocol("close without a file".into()))?;
                // Flush big files to disk before they're moved into place; for
                // thousands of small files a sync each would dominate the time.
                if file.metadata().map(|m| m.len() >= 8 * 1024 * 1024).unwrap_or(false) {
                    file.sync_data().ok();
                } else {
                    unsynced.push(path.clone());
                }
                drop(file);
                if hasher.finalize().as_bytes() != &expect {
                    return Err(Failure::Corrupt(name));
                }
                if let Some(t) = mtime {
                    let _ = filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(t.s, t.ns));
                }
                Ok(Some(WriteEvent::Closed))
            }
            WriteCmd::Checkpoint(mut state) => {
                unsynced
                    .retain(|p| std::fs::OpenOptions::new().write(true).open(p).and_then(|f| f.sync_data()).is_err());
                if !unsynced.is_empty() {
                    tracing::warn!("couldn't flush received files to disk; keeping the previous checkpoint");
                    return Ok(None);
                }
                if let (Some(p), Some(f)) = (state.partial.as_mut(), open.as_ref()) {
                    p.written = f.written;
                    p.hash = B32(*f.hasher.clone().finalize().as_bytes());
                }
                if let Err(e) = journal::write_atomic(journal, &Saved::In(state)) {
                    tracing::warn!("couldn't save the transfer's progress: {e}");
                }
                Ok(None)
            }
            WriteCmd::Finish => Ok(Some(WriteEvent::Finished)),
        })();
        match step {
            Ok(Some(ev)) => {
                let finished = matches!(ev, WriteEvent::Finished);
                // Nobody listening (the transfer paused): keep going, so
                // what's queued, and the last checkpoint, still get written.
                let _ = tx.blocking_send(ev);
                if finished {
                    return;
                }
            }
            Ok(None) => {}
            Err(f) => {
                let _ = tx.blocking_send(WriteEvent::Failed(f));
                return;
            }
        }
    }
}

type Finalized = (Vec<PathBuf>, Vec<(String, String)>);

/// Folder times (deepest first), then each top-level item into place with a
/// no-replace rename, numbering on collision.
fn finalize(
    staging: &Path,
    receive_dir: &Path,
    top: &[(String, bool)],
    dir_times: &mut [(Vec<String>, Mtime)],
    target: Target,
) -> Result<Finalized, Failure> {
    finalize_with(staging, receive_dir, top, dir_times, target, portal_sys::fs::rename_noreplace)
}

/// `finalize`, with the rename step injectable so tests can make it fail.
/// If any move fails, the items already moved go back into staging and are
/// deleted with it: a failed transfer leaves nothing in the receive folder.
fn finalize_with(
    staging: &Path,
    receive_dir: &Path,
    top: &[(String, bool)],
    dir_times: &mut [(Vec<String>, Mtime)],
    target: Target,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<Finalized, Failure> {
    // A device's own folder may not be there yet (or was removed meanwhile)
    let _ = std::fs::create_dir_all(receive_dir);
    dir_times.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
    for (rel, t) in dir_times.iter() {
        let _ = filetime::set_file_mtime(join(staging, rel), filetime::FileTime::from_unix_time(t.s, t.ns));
    }
    let mut saved = Vec::new();
    // (where it went, where it came from) for rolling back
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut numbered = Vec::new();
    for (name, is_dir) in top {
        let src = staging.join(name);
        if !src.exists() {
            continue;
        }
        let mut n = 1;
        let dest = loop {
            let candidate = if n == 1 { name.clone() } else { fsafe::numbered_fitting(name, n, *is_dir, target) };
            let dest = receive_dir.join(&candidate);
            match rename(&src, &dest) {
                Ok(()) => {
                    if n > 1 {
                        numbered.push((name.clone(), candidate));
                    }
                    break dest;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 10_000 => n += 1,
                Err(e) => {
                    for (placed, from) in moved.iter().rev() {
                        if portal_sys::fs::rename_noreplace(placed, from).is_err() {
                            tracing::warn!("couldn't roll back {}", placed.display());
                        }
                    }
                    return Err(Failure::from_io(&e, "moving into place"));
                }
            }
        };
        moved.push((dest.clone(), src.clone()));
        if *is_dir && let Some((_, t)) = dir_times.iter().find(|(p, _)| p.len() == 1 && &p[0] == name) {
            let _ = filetime::set_file_mtime(&dest, filetime::FileTime::from_unix_time(t.s, t.ns));
        }
        saved.push(dest);
    }
    cleanup(staging);
    Ok((saved, numbered))
}

/// Paths longer than Windows' classic 260-character limit.
fn long_paths(saved: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(p: &Path, n: &mut Vec<PathBuf>) {
        if p.as_os_str().to_string_lossy().encode_utf16().count() > 260 {
            n.push(p.to_path_buf());
        }
        if p.is_dir()
            && let Ok(rd) = std::fs::read_dir(p)
        {
            for e in rd.flatten() {
                walk(&e.path(), n);
            }
        }
    }
    let mut n = Vec::new();
    for p in saved {
        walk(p, &mut n);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_move_rolls_back_items_already_moved() {
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join(".seamdrop-incoming/t1");
        let receive = root.path().join("Received");
        std::fs::create_dir_all(staging.join("Photos")).unwrap();
        std::fs::write(staging.join("Photos/a.jpg"), b"a").unwrap();
        std::fs::write(staging.join("notes.txt"), b"n").unwrap();
        std::fs::create_dir_all(&receive).unwrap();
        let top = vec![("Photos".to_string(), true), ("notes.txt".to_string(), false)];
        let calls = std::cell::Cell::new(0);
        let flaky = |from: &Path, to: &Path| {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                return Err(std::io::Error::other("disk went away"));
            }
            portal_sys::fs::rename_noreplace(from, to)
        };

        let result = finalize_with(&staging, &receive, &top, &mut [], Target::Mac, flaky);

        assert!(result.is_err());
        assert!(std::fs::read_dir(&receive).unwrap().next().is_none(), "nothing left in the receive folder");
        assert!(staging.join("Photos/a.jpg").exists(), "moved back so cleanup removes it");
    }
}
