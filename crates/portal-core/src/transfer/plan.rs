//! Sender side: walk what was dropped, once, into an offer and a file list.

use std::collections::HashSet;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

use crate::fsafe;
use crate::model::Role;
use crate::proto::{DirEntry, ItemKind, Mtime, Offer, OfferItem, SkipNote, SkipReason, TransferId};

/// At most this many skip notes travel in an offer (the count is still exact locally).
const MAX_SKIP_NOTES: usize = 1000;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlannedFile {
    pub src: PathBuf,
    pub rel: Vec<String>,
    pub size: u64,
    pub mtime: Option<Mtime>,
}

#[derive(Clone, Debug)]
pub struct Planned {
    pub offer: Offer,
    pub dirs: Vec<DirEntry>,
    pub files: Vec<PlannedFile>,
    pub skipped: Vec<SkipNote>,
}

pub fn mtime_of(meta: &Metadata) -> Option<Mtime> {
    let t = filetime::FileTime::from_last_modification_time(meta);
    Some(Mtime { s: t.unix_seconds(), ns: t.nanoseconds() })
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

/// `name`, or "name (2).ext", "name (3).ext"… if an earlier item took it
/// (ignoring case and Unicode normalisation, like the receiving disk may).
fn unique_top_name(name: String, is_dir: bool, taken: &mut HashSet<String>) -> String {
    let mut candidate = name.clone();
    let mut n = 2;
    while !taken.insert(fsafe::fold(&candidate)) {
        candidate = fsafe::numbered(&name, n, is_dir);
        n += 1;
    }
    candidate
}

/// Never follows symlinks; skips junk when asked; folders keep their structure.
/// Items picked from different places may share a name: later ones are numbered.
pub fn plan(tid: TransferId, inputs: &[PathBuf], skip_junk: bool) -> std::io::Result<Planned> {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut items = Vec::new();
    let mut taken = HashSet::new();

    for input in inputs {
        let meta = std::fs::symlink_metadata(input)?;
        let ft = meta.file_type();
        let name = if ft.is_file() || ft.is_dir() {
            unique_top_name(name_of(input), ft.is_dir(), &mut taken)
        } else {
            name_of(input)
        };
        if ft.is_symlink() {
            skipped.push(SkipNote { path: vec![name], reason: SkipReason::Symlink });
        } else if ft.is_file() {
            files.push(PlannedFile {
                src: input.clone(),
                rel: vec![name.clone()],
                size: meta.len(),
                mtime: mtime_of(&meta),
            });
            items.push(OfferItem { name, kind: ItemKind::File, bytes: meta.len(), files: 1 });
        } else if ft.is_dir() {
            let (before_files, before_bytes) = (files.len(), files.iter().map(|f| f.size).sum::<u64>());
            dirs.push(DirEntry { path: vec![name.clone()], mtime: mtime_of(&meta) });
            walk(input, std::slice::from_ref(&name), skip_junk, &mut dirs, &mut files, &mut skipped);
            let bytes = files.iter().map(|f| f.size).sum::<u64>() - before_bytes;
            items.push(OfferItem { name, kind: ItemKind::Folder, bytes, files: (files.len() - before_files) as u64 });
        } else {
            skipped.push(SkipNote { path: vec![name], reason: SkipReason::Special });
        }
    }

    let offer = Offer {
        tid,
        total_bytes: files.iter().map(|f| f.size).sum(),
        file_count: files.len() as u64,
        dir_count: dirs.len() as u64,
        skipped: skipped.iter().take(MAX_SKIP_NOTES).cloned().collect(),
        items,
        sender_role: Role::Both,
    };
    Ok(Planned { offer, dirs, files, skipped })
}

/// What one path would send, for the list before sending: its name, kind,
/// size and file count, counted the way `plan` will. None if it can't be
/// read, or it's a link or something else that isn't sent.
pub fn describe(path: &Path, skip_junk: bool) -> Option<OfferItem> {
    plan(0, &[path.to_path_buf()], skip_junk).ok()?.offer.items.into_iter().next()
}

fn walk(
    dir: &Path,
    rel: &[String],
    skip_junk: bool,
    dirs: &mut Vec<DirEntry>,
    files: &mut Vec<PlannedFile>,
    skipped: &mut Vec<SkipNote>,
) {
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(Result::ok).collect(),
        Err(_) => {
            skipped.push(SkipNote { path: rel.to_vec(), reason: SkipReason::Unreadable });
            return;
        }
    };
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let mut path = rel.to_vec();
        path.push(name.clone());
        if skip_junk && fsafe::is_junk(&name) {
            skipped.push(SkipNote { path, reason: SkipReason::Junk });
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
            skipped.push(SkipNote { path, reason: SkipReason::Unreadable });
            continue;
        };
        let ft = meta.file_type();
        if ft.is_symlink() {
            skipped.push(SkipNote { path, reason: SkipReason::Symlink });
        } else if ft.is_dir() {
            dirs.push(DirEntry { path: path.clone(), mtime: mtime_of(&meta) });
            walk(&entry.path(), &path, skip_junk, dirs, files, skipped);
        } else if ft.is_file() {
            files.push(PlannedFile { src: entry.path(), rel: path, size: meta.len(), mtime: mtime_of(&meta) });
        } else {
            skipped.push(SkipNote { path, reason: SkipReason::Special });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describing_counts_like_sending_does() {
        let dir = tempfile::tempdir().unwrap();
        let photos = dir.path().join("Photos");
        std::fs::create_dir_all(photos.join("2024")).unwrap();
        std::fs::write(photos.join("a.jpg"), [0u8; 300]).unwrap();
        std::fs::write(photos.join("2024/b.jpg"), [0u8; 700]).unwrap();
        std::fs::write(photos.join(".DS_Store"), [0u8; 50]).unwrap();
        let folder = describe(&photos, true).unwrap();
        assert_eq!(
            (folder.name.as_str(), folder.kind, folder.bytes, folder.files),
            ("Photos", ItemKind::Folder, 1000, 2)
        );
        assert_eq!(describe(&photos, false).unwrap().files, 3, "system files count when they're sent");
        let file = describe(&photos.join("a.jpg"), true).unwrap();
        assert_eq!((file.kind, file.bytes, file.files), (ItemKind::File, 300, 1));
        assert!(describe(&dir.path().join("gone.txt"), true).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_not_something_to_send() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(dir.path().join("real.txt"), dir.path().join("link.txt")).unwrap();
        assert!(describe(&dir.path().join("link.txt"), true).is_none());
    }
}
