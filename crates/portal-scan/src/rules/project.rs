//! Project-wide rules: environments, lockfiles, Python version, Docker,
//! native apps, secrets and services.

use super::{Ctx, Hit};
use crate::report::TargetOs;

/// Docker images known to ship arm64 builds: official library images and a few vendors.
fn known_multi_arch(image: &str) -> bool {
    let name = image.split('@').next().unwrap_or(image);
    let name = name.rsplit_once(':').filter(|(_, tag)| !tag.contains('/')).map(|(n, _)| n).unwrap_or(name);
    let name = name.trim_start_matches("docker.io/").trim_start_matches("library/");
    !name.contains('/')
        || [
            "ollama/ollama",
            "qdrant/qdrant",
            "minio/minio",
            "grafana/grafana",
            "prom/prometheus",
            "chromadb/chroma",
            "bitnami/postgresql",
            "bitnami/redis",
            "mcr.microsoft.com/devcontainers/python",
            "mcr.microsoft.com/devcontainers/javascript-node",
            "ghcr.io/astral-sh/uv",
        ]
        .contains(&name)
}

/// Settings files searched line by line for absolute paths (config isn't code).
const CONFIG_EXTS: &[&str] = &["yaml", "yml", "json", "toml", "ini", "cfg", "conf", "properties", "env"];
const LOCKFILES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "poetry.lock",
    "uv.lock",
    "Pipfile.lock",
    "composer.lock",
];

/// Absolute paths in settings files. Secret files are never opened.
fn config_paths(ctx: &Ctx<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for f in &ctx.inv.files {
        let name = f.rel.rsplit('/').next().unwrap_or(&f.rel);
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        if !CONFIG_EXTS.contains(&ext.as_str())
            || LOCKFILES.contains(&name)
            || crate::snapshot::secret_kind(name).is_some()
            || f.size > crate::parse::MAX_PARSE_BYTES
        {
            continue;
        }
        let Ok(bytes) = std::fs::read(&f.abs) else { continue };
        if !crate::parse::is_text(&bytes) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (i, raw_line) in text.lines().enumerate() {
            let line = raw_line.replace("\\\\", "\\");
            for start in path_starts(&line) {
                let token: String = line[start..]
                    .chars()
                    .take_while(|c| {
                        !matches!(c, '"' | '\'' | ',' | ';' | ']' | '}' | ')' | '<' | '>') && !c.is_whitespace()
                    })
                    .collect();
                if let Some(targets) = super::python::absolute_path_targets(&token) {
                    let mut hit = Hit::new("absolute-path")
                        .at(&f.rel, i as u32 + 1)
                        .evidence(raw_line.trim().to_owned())
                        .var("path", crate::parse::shorten(&token, 60));
                    hit.targets = Some(targets);
                    hits.push(hit);
                    break;
                }
            }
        }
    }
    hits
}

/// Where a path could start in a line of settings.
fn path_starts(line: &str) -> Vec<usize> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    for (i, w) in bytes.windows(3).enumerate() {
        let boundary = i == 0 || matches!(bytes[i - 1], b'"' | b'\'' | b' ' | b'=' | b':' | b'\t' | b'(' | b'[');
        // Bytes, not str slices: a line may hold any UTF-8. Both starts are
        // ASCII, so `i` is always a character boundary.
        let drive = w[0].is_ascii_alphabetic() && w[1] == b':' && (w[2] == b'\\' || w[2] == b'/');
        let unix = bytes[i..].starts_with(b"/Users/") || bytes[i..].starts_with(b"/home/");
        if boundary && (drive || unix) {
            out.push(i);
        }
    }
    out
}

pub fn check(ctx: &Ctx<'_>) -> Vec<Hit> {
    let mut hits = config_paths(ctx);
    let s = ctx.snap;

    // Environments and installed packages that won't be sent.
    for h in ctx.inv.heavy.iter().filter(|h| matches!(h.kind, "pythonEnv" | "nodeModules") && !h.rel.contains('/')) {
        let rebuild = if h.kind == "pythonEnv" { "uv sync, or pip install -r requirements.txt" } else { "npm install" };
        hits.push(
            Hit::new("env-folder")
                .file(&h.rel)
                .evidence(format!("{}: {}, {}", h.rel, count(h.files, "file"), human_bytes(h.bytes)))
                .var("folder", h.rel.clone())
                .var("size", human_bytes(h.bytes))
                .var("rebuild", rebuild),
        );
    }

    // No lockfile: the snapshot becomes the lock.
    for p in &s.packages {
        if p.lockfile.is_none() && p.declared > 0 {
            let (eco, how) = if p.kind == "python" {
                ("Python", "uv lock, or pip freeze > requirements.txt")
            } else {
                ("Node", "npm install writes package-lock.json")
            };
            hits.push(
                Hit::new("no-lockfile")
                    .evidence(format!("{} declared, none locked", count(p.declared as u64, "package")))
                    .var("ecosystem", eco)
                    .var("how", how),
            );
        }
    }

    // A Python request that only allows very old versions.
    if let Some((spec, file)) = &s.requires_python
        && !allows_any(spec, &[(3, 9), (3, 10), (3, 11), (3, 12), (3, 13)])
    {
        hits.push(Hit::new("python-version").file(file).evidence(format!("Python {spec}")).var("spec", spec.clone()));
    }

    // Docker: needed at all; images pinned to Intel; images not known to have arm64.
    let compose_or_dockerfile = ctx.inv.files.iter().any(|f| {
        let n = f.rel.rsplit('/').next().unwrap_or(&f.rel).to_ascii_lowercase();
        n == "dockerfile"
            || n.starts_with("dockerfile.")
            || n.starts_with("docker-compose")
            || n == "compose.yml"
            || n == "compose.yaml"
    });
    if compose_or_dockerfile {
        let wsl = if ctx.target == TargetOs::Windows { " (it runs on WSL 2)" } else { "" };
        hits.push(Hit::new("docker-needed").var("wsl", wsl));
    }
    let mut seen_images = std::collections::HashSet::new();
    for (file, line, image, platform) in &s.docker_images {
        if !seen_images.insert(image.clone()) {
            continue;
        }
        let platform = platform.as_deref().unwrap_or("");
        if platform.contains("amd64") || platform.contains("x86_64") {
            hits.push(
                Hit::new("docker-amd64")
                    .at(file, *line)
                    .evidence(format!("{image} (platform {platform})"))
                    .var("platform", platform.to_owned()),
            );
        } else if !known_multi_arch(image) && !image.contains('$') {
            hits.push(Hit::new("docker-arm64").at(file, *line).evidence(image.clone()).var("image", image.clone()));
        }
    }

    // Native apps.
    let mut apple_done = false;
    for (kind, file) in &s.native {
        match *kind {
            "xcode" if !apple_done => {
                apple_done = true;
                hits.push(Hit::new("native-apple").file(file).evidence(file.clone()));
            }
            "wpf" | "winforms" | "windows-desktop" | "vcxproj" => {
                let label = match *kind {
                    "wpf" => "WPF",
                    "winforms" => "Windows Forms",
                    "vcxproj" => "Visual C++",
                    _ => ".NET Windows desktop",
                };
                hits.push(Hit::new("native-windows").file(file).evidence(file.clone()).var("kind", label));
            }
            _ => {}
        }
    }
    if !apple_done {
        // A Swift package app with Apple UI frameworks and no Xcode project.
        for f in ctx.inv.files.iter().filter(|f| f.rel.ends_with(".swift")) {
            let Ok(text) = std::fs::read_to_string(&f.abs) else { continue };
            if let Some((n, l)) = text.lines().enumerate().find(|(_, l)| {
                let t = l.trim();
                ["import SwiftUI", "import AppKit", "import UIKit", "import Cocoa"]
                    .iter()
                    .any(|i| t == *i || t.starts_with(&format!("{i} ")))
            }) {
                hits.push(Hit::new("native-apple").at(&f.rel, n as u32 + 1).evidence(l.trim().to_owned()));
                break;
            }
        }
    }

    // Secrets: by name, keys only.
    for sf in &s.secrets {
        let what = match sf.kind.as_str() {
            "env" if !sf.keys.is_empty() => format!("these settings: {}", sf.keys.join(", ")),
            "env" => "settings that may be secret".to_owned(),
            "key" => "a private key or certificate".to_owned(),
            _ => "credentials".to_owned(),
        };
        let evidence =
            if sf.keys.is_empty() { sf.path.clone() } else { format!("{}: {}", sf.path, sf.keys.join(", ")) };
        hits.push(Hit::new("secrets").file(&sf.path).evidence(evidence).var("file", sf.path.clone()).var("what", what));
    }

    // Services.
    for svc in &s.services {
        let from_compose = svc.from.ends_with(".yml") || svc.from.ends_with(".yaml");
        if from_compose || svc.kind == "ollama" {
            let from = if from_compose {
                format!("it's a service in {}", svc.from)
            } else {
                format!("it uses the {}", svc.from)
            };
            hits.push(
                Hit::new("service")
                    .evidence(format!("{} ({})", svc.name, svc.from))
                    .var("service", svc.name.clone())
                    .var("from", from),
            );
        } else {
            hits.push(
                Hit::new("service-client")
                    .evidence(format!("{} ({})", svc.name, svc.from))
                    .var("service", svc.name.clone())
                    .var("from", svc.from.clone()),
            );
        }
    }

    // Package builds per OS: only checked online.
    if !s.python_deps.is_empty() {
        let n = unique_count(&s.python_deps);
        let what = if n == 1 { "1 Python package".to_owned() } else { format!("{n} Python packages") };
        hits.push(Hit::new("wheels-unchecked").evidence(what));
    }
    hits
}

fn unique_count(deps: &[(String, String, String)]) -> usize {
    deps.iter().map(|(n, _, _)| n).collect::<std::collections::HashSet<_>>().len()
}

pub fn human_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["bytes", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut u = 0;
    while v >= 1000.0 && u < UNITS.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 {
        format!("{b} bytes")
    } else if v >= 100.0 {
        format!("{v:.0} {}", UNITS[u])
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// Does a Python version spec allow any of these (major, minor) releases?
/// PEP 440 subset plus Poetry's ^ and ~; a bare "3.8" means 3.8.*.
pub fn allows_any(spec: &str, candidates: &[(u32, u32)]) -> bool {
    candidates.iter().any(|c| allows(spec, *c))
}

fn parse_version(v: &str) -> Option<(u32, u32, u32, bool)> {
    let v = v.trim();
    let wildcard = v.ends_with(".*");
    let mut parts = v.trim_end_matches(".*").split('.').map(|p| p.trim().parse::<u32>());
    let major = parts.next()?.ok()?;
    let minor = parts.next().unwrap_or(Ok(0)).ok()?;
    let patch = parts.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch, wildcard))
}

fn allows(spec: &str, (maj, min): (u32, u32)) -> bool {
    let cand = (maj, min, 0u32);
    let cand_hi = (maj, min, u32::MAX);
    spec.split(',').map(str::trim).filter(|c| !c.is_empty()).all(|clause| {
        let (op, rest) = ["~=", "==", "!=", ">=", "<=", ">", "<", "^", "~"]
            .iter()
            .find_map(|op| clause.strip_prefix(op).map(|r| (*op, r)))
            .unwrap_or(("==", clause));
        let Some((a, b, c, wild)) = parse_version(rest) else { return true };
        let v = (a, b, c);
        match op {
            ">=" => cand_hi >= v,
            ">" => cand_hi > v,
            "<=" => cand <= v,
            "<" => cand < v,
            "!=" => !(wild && (maj, min) == (a, b)),
            "==" => (maj, min) == (a, b),
            "~=" => cand_hi >= v && if rest.trim().split('.').count() >= 3 { (maj, min) == (a, b) } else { maj == a },
            "^" => cand_hi >= v && maj == a,
            "~" => (maj, min) == (a, b),
            _ => true,
        }
    })
}

/// "1 file", "3 files"
fn count(n: u64, one: &str) -> String {
    if n == 1 { format!("1 {one}") } else { format!("{n} {one}s") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_specs_are_read_like_pip_reads_them() {
        let modern = [(3, 9), (3, 10), (3, 11), (3, 12), (3, 13)];
        for ok in [">=3.11", ">=3.8,<4", "^3.10", "~=3.9", "3.11.9", "==3.12.*", ">3.8"] {
            assert!(allows_any(ok, &modern), "{ok}");
        }
        for old in ["<3.9", "==3.7.*", ">=3.6,<3.9", "~=3.8.1", "3.8", "~3.7"] {
            assert!(!allows_any(old, &modern), "{old}");
        }
    }

    #[test]
    fn settings_lines_with_any_text_are_searched_safely() {
        assert_eq!(path_starts("naïve: \"/Users/ana/x\" — café"), vec![9]);
        assert_eq!(path_starts("out = C:\\data, ünïcode"), vec![6]);
        assert!(path_starts("日本語のテキスト /tmp/x").is_empty());
    }

    #[test]
    fn official_images_count_as_multi_arch() {
        assert!(known_multi_arch("python:3.12-slim"));
        assert!(known_multi_arch("postgres:16"));
        assert!(known_multi_arch("ollama/ollama:latest"));
        assert!(!known_multi_arch("acme/legacy-tool:1.2"));
        assert!(!known_multi_arch("ghcr.io/acme/app:main"));
    }
}
