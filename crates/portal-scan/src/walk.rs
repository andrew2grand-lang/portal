//! Walking the folder: what counts as the project (honouring .gitignore),
//! what won't be sent (environments, caches, build output, ignored files),
//! and links. Never follows a symlink.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use ignore::{DirEntry, WalkBuilder, WalkState};
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct FileEntry {
    /// Relative to the project root, forward slashes.
    pub rel: String,
    pub abs: PathBuf,
    pub size: u64,
    /// Unix permission bits say executable (always false on Windows).
    pub executable: bool,
}

#[derive(Clone, Debug)]
pub struct HeavyDir {
    pub rel: String,
    pub abs: PathBuf,
    /// "pythonEnv", "nodeModules", "cache", "buildOutput".
    pub kind: &'static str,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Clone, Debug)]
pub struct Symlink {
    pub rel: String,
    /// What it points to, as stored.
    pub target: String,
    /// Points outside the project (or nowhere).
    pub outside: bool,
}

#[derive(Debug, Default)]
pub struct Inventory {
    /// Files that are the project.
    pub files: Vec<FileEntry>,
    /// Files .gitignore leaves out (outside heavy folders). Still checked for secrets.
    pub ignored: Vec<FileEntry>,
    pub heavy: Vec<HeavyDir>,
    pub symlinks: Vec<Symlink>,
    pub unreadable: Vec<String>,
}

/// OS clutter: never part of a project.
pub fn is_os_junk(name: &str) -> bool {
    matches!(name, ".DS_Store" | "Thumbs.db" | "desktop.ini" | "ehthumbs.db" | ".Spotlight-V100" | ".Trashes")
        || name.starts_with("._")
}

/// A folder that's rebuilt on the other side rather than sent.
fn heavy_kind(dir: &Path, name: &str) -> Option<&'static str> {
    match name {
        "node_modules" => Some("nodeModules"),
        ".venv" | "venv" | ".virtualenv" => Some("pythonEnv"),
        "env" | ".env" if dir.join("pyvenv.cfg").is_file() => Some("pythonEnv"),
        "__pycache__" | ".mypy_cache" | ".pytest_cache" | ".ruff_cache" | ".tox" | ".nox" | ".ipynb_checkpoints"
        | ".parcel-cache" | ".turbo" | ".gradle" | ".eggs" => Some("cache"),
        ".next" | ".nuxt" | ".svelte-kit" | "DerivedData" | "Pods" => Some("buildOutput"),
        "target" if dir.parent().is_some_and(|p| p.join("Cargo.toml").is_file()) => Some("buildOutput"),
        "bin" | "obj" if dir.parent().is_some_and(has_dotnet_project) => Some("buildOutput"),
        _ if name.ends_with(".egg-info") => Some("cache"),
        _ => None,
    }
}

fn has_dotnet_project(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok).any(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.ends_with(".csproj") || n.ends_with(".vbproj") || n.ends_with(".fsproj")
            })
        })
        .unwrap_or(false)
}

fn rel_of(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &std::fs::Metadata) -> bool {
    false
}

fn entry_of(root: &Path, e: &DirEntry) -> Option<FileEntry> {
    let meta = e.metadata().ok()?;
    Some(FileEntry {
        rel: rel_of(root, e.path()),
        abs: e.path().to_path_buf(),
        size: meta.len(),
        executable: is_executable(&meta),
    })
}

fn symlink_of(root: &Path, path: &Path) -> Symlink {
    let target = std::fs::read_link(path).map(|t| t.to_string_lossy().into_owned()).unwrap_or_default();
    let resolved = std::fs::canonicalize(path).ok();
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let outside = resolved.is_none_or(|r| !r.starts_with(&root));
    Symlink { rel: rel_of(&root, path).trim_start_matches("../").to_owned(), target, outside }
}

pub fn walk(root: &Path) -> Inventory {
    let mut inv = Inventory::default();

    // 1. Everything except .git, stopping at heavy folders: the left-out list,
    //    ignored files, links and unreadable paths.
    let mut all: Vec<FileEntry> = Vec::new();
    let heavy_found: Arc<Mutex<Vec<(PathBuf, &'static str)>>> = Default::default();
    let mut walker = WalkBuilder::new(root);
    walker.standard_filters(false).hidden(false).follow_links(false);
    let (found, root_buf) = (heavy_found.clone(), root.to_path_buf());
    walker.filter_entry(move |e| {
        if e.path() == root_buf || !e.file_type().is_some_and(|t| t.is_dir()) {
            return true;
        }
        let name = e.file_name().to_string_lossy();
        if name == ".git" {
            return false;
        }
        match heavy_kind(e.path(), &name) {
            Some(kind) => {
                found.lock().unwrap_or_else(|p| p.into_inner()).push((e.path().to_path_buf(), kind));
                false
            }
            None => true,
        }
    });
    for item in walker.build() {
        let e = match item {
            Ok(e) => e,
            Err(err) => {
                inv.unreadable.push(error_path(root, &err));
                continue;
            }
        };
        if e.depth() == 0 {
            continue;
        }
        let ft = e.file_type();
        if ft.is_some_and(|t| t.is_symlink()) {
            inv.symlinks.push(symlink_of(root, e.path()));
            continue;
        }
        // Regular files only: a FIFO or device would block or never end
        if !ft.is_some_and(|t| t.is_file()) {
            continue;
        }
        if let Some(f) = entry_of(root, &e) {
            all.push(f);
        }
    }
    let heavy: Vec<(String, PathBuf, &'static str)> =
        std::mem::take(&mut *heavy_found.lock().unwrap_or_else(|p| p.into_inner()))
            .into_iter()
            .map(|(abs, kind)| (rel_of(root, &abs), abs, kind))
            .collect();

    // 2. The project: .gitignore honoured, heavy folders and .git skipped.
    let mut project: HashSet<String> = HashSet::new();
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .git_global(false)
        .ignore(false)
        .parents(false)
        .require_git(false)
        .follow_links(false);
    let root_buf = root.to_path_buf();
    walker.filter_entry(move |e| {
        let name = e.file_name().to_string_lossy();
        if name == ".git" {
            return false;
        }
        !(e.file_type().is_some_and(|t| t.is_dir()) && heavy_kind(e.path(), &name).is_some() && e.path() != root_buf)
    });
    for e in walker.build().flatten() {
        if e.file_type().is_some_and(|t| t.is_file()) {
            project.insert(rel_of(root, e.path()));
        }
    }

    for f in all {
        let name = f.rel.rsplit('/').next().unwrap_or(&f.rel);
        if project.contains(&f.rel) && !is_os_junk(name) {
            inv.files.push(f);
        } else {
            inv.ignored.push(f);
        }
    }

    inv.heavy = heavy
        .into_par_iter()
        .map(|(rel, abs, kind)| {
            let (bytes, files) = size_of(&abs);
            HeavyDir { rel, abs, kind, bytes, files }
        })
        .collect();
    inv.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    inv.ignored.sort_by(|a, b| a.rel.cmp(&b.rel));
    inv.heavy.sort_by(|a, b| a.rel.cmp(&b.rel));
    inv.symlinks.sort_by(|a, b| a.rel.cmp(&b.rel));
    inv
}

fn error_path(root: &Path, err: &ignore::Error) -> String {
    match err {
        ignore::Error::WithPath { path, .. } => rel_of(root, path),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => error_path(root, err),
        other => other.to_string(),
    }
}

/// Total size and file count under a folder, in parallel, without following links.
pub fn size_of(dir: &Path) -> (u64, u64) {
    let bytes = AtomicU64::new(0);
    let files = AtomicU64::new(0);
    let mut walker = WalkBuilder::new(dir);
    walker.standard_filters(false).hidden(false).follow_links(false);
    walker.build_parallel().run(|| {
        Box::new(|item| {
            if let Ok(e) = item
                && e.file_type().is_some_and(|t| t.is_file())
                && let Ok(m) = e.metadata()
            {
                bytes.fetch_add(m.len(), Ordering::Relaxed);
                files.fetch_add(1, Ordering::Relaxed);
            }
            WalkState::Continue
        })
    });
    (bytes.into_inner(), files.into_inner())
}
