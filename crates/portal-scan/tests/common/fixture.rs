//! Builds a fixture project in a temp folder. Fixtures store special names
//! as `dot_x` (→ `.x`) and `x_name` (→ `name`), and symlinks / exec bits in
//! `_manifest.json`, so git and every OS keep them exactly. Names Windows
//! can't check out are stored plainly and listed under `renames`.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Not part of the project: the fixture's own bookkeeping.
const BOOKKEEPING: &[&str] = &["expected.json", "_manifest.json"];

fn real_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("dot_") {
        format!(".{rest}")
    } else if let Some(rest) = name.strip_prefix("x_") {
        rest.to_owned()
    } else {
        name.to_owned()
    }
}

fn copy_tree(src: &Path, dst: &Path, top: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if top && BOOKKEEPING.contains(&name.as_str()) {
            continue;
        }
        let to = dst.join(real_name(&name));
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &to, false)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// The fixture's manifest (or an empty one).
pub fn manifest(src: &Path) -> serde_json::Value {
    std::fs::read_to_string(src.join("_manifest.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// Copies `src` to `dst` with real names, links and exec bits. False if the
/// fixture can't exist on this OS (e.g. names Windows can't create).
pub fn materialize(src: &Path, dst: &Path) -> std::io::Result<bool> {
    let m = manifest(src);
    if cfg!(windows) && m.get("unixOnly").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(false);
    }
    copy_tree(src, dst, true)?;
    // Names a Windows checkout can't hold (aux.py, "a: b?.md", > 260 chars)
    // are stored under plain names and renamed here.
    if let Some(renames) = m.get("renames").and_then(|v| v.as_object()) {
        for (stored, real) in renames {
            let to = dst.join(real.as_str().unwrap_or_default());
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::rename(dst.join(stored), to)?;
        }
    }
    // Big files (a demo project's .venv) as sparse files: the real size, no disk.
    if let Some(sparse) = m.get("sparse").and_then(|v| v.as_object()) {
        for (rel, bytes) in sparse {
            let to = dst.join(rel);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(&to)?.set_len(bytes.as_u64().unwrap_or(0))?;
        }
    }
    if let Some(links) = m.get("symlinks").and_then(|v| v.as_object()) {
        for (link, target) in links {
            let target = target.as_str().unwrap_or_default();
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, dst.join(link))?;
            #[cfg(windows)]
            {
                let made = if dst.join(link).parent().map(|p| p.join(target)).is_some_and(|t| t.is_dir()) {
                    std::os::windows::fs::symlink_dir(target, dst.join(link))
                } else {
                    std::os::windows::fs::symlink_file(target, dst.join(link))
                };
                if made.is_err() {
                    return Ok(false); // no symlink privilege here
                }
            }
        }
    }
    #[cfg(unix)]
    if let Some(files) = m.get("executable").and_then(|v| v.as_array()) {
        use std::os::unix::fs::PermissionsExt;
        for f in files.iter().filter_map(|f| f.as_str()) {
            let p = dst.join(f);
            let mut perm = std::fs::metadata(&p)?.permissions();
            perm.set_mode(perm.mode() | 0o755);
            std::fs::set_permissions(&p, perm)?;
        }
    }
    Ok(true)
}

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
