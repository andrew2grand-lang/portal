//! SeamDrop's project scanner: reads a project folder, never runs it, and
//! writes an honest report of what blocks it on the other OS.
//!
//! Never executed: the project's code, scripts, notebooks or package managers.
//! Never read: secret values (key names only). Network: only with `online`,
//! only PyPI's JSON API.

/// The product name, from `productName` in `src-tauri/tauri.conf.json` (see `build.rs`).
pub const APP_NAME: &str = env!("SEAMDROP_APP_NAME");

pub mod online;
pub mod parse;
pub mod report;
pub mod rules;
pub mod snapshot;
pub mod summary;
pub mod text;
pub mod walk;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use rayon::prelude::*;
use serde::Serialize;

use crate::parse::Lang;
use crate::report::{Counts, Finding, Network, Os, ProjectInfo, Report, SCHEMA_VERSION, State, Stats, TargetOs, Tool};
use crate::rules::{Ctx, Hit, MAX_PER_RULE};

pub struct ScanOptions {
    pub target: TargetOs,
    /// Read PyPI's JSON API for package builds (off by default).
    pub online: bool,
    /// Ask `python --version` / `node --version` on this machine.
    pub host_versions: bool,
    /// Where PyPI answers come from (tests pass a fake; None = the real API).
    pub pypi: Option<Box<dyn online::PypiSource>>,
}

impl ScanOptions {
    pub fn new(target: TargetOs) -> Self {
        ScanOptions { target, online: false, host_versions: true, pypi: None }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    Reading,
    Ecosystems,
    Code,
    Packages,
    Report,
}

/// Live progress, for "Checking 214 files for Windows-only code (120 of 214)".
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub step: Step,
    pub done: u64,
    pub total: u64,
    pub current: Option<String>,
}

#[derive(Debug)]
pub enum ScanError {
    NotAFolder(String),
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::NotAFolder(p) => write!(f, "{p} isn't a folder that can be read"),
        }
    }
}

impl std::error::Error for ScanError {}

fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Scans `root` for `opts.target`. Calls `progress` as it goes (from any thread).
pub fn scan(root: &Path, opts: &ScanOptions, progress: &(dyn Fn(Progress) + Sync)) -> Result<Report, ScanError> {
    let started = Instant::now();
    if !root.is_dir() {
        return Err(ScanError::NotAFolder(root.display().to_string()));
    }
    let root = &std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    progress(Progress { step: Step::Reading, done: 0, total: 0, current: None });
    let inv = walk::walk(root);
    progress(Progress {
        step: Step::Reading,
        done: inv.files.len() as u64,
        total: inv.files.len() as u64,
        current: None,
    });

    progress(Progress { step: Step::Ecosystems, done: 0, total: 0, current: None });
    let snap = snapshot::snapshot(root, &inv, opts.host_versions);
    let ctx = Ctx { root, target: opts.target, inv: &inv, snap: &snap };

    // Code, in parallel.
    let code: Vec<(&walk::FileEntry, Lang)> = inv
        .files
        .iter()
        .filter(|f| f.size <= parse::MAX_PARSE_BYTES)
        .filter_map(|f| {
            if f.rel.ends_with(".ipynb") {
                return Some((f, Lang::Python));
            }
            let lang = parse::lang_for(&f.rel, &sniff(f))?;
            (lang != Lang::Bash).then_some((f, lang))
        })
        .collect();
    let total = code.len() as u64;
    let done = AtomicU64::new(0);
    let skipped = AtomicU64::new(0);
    let last_report = Mutex::new(Instant::now());
    progress(Progress { step: Step::Code, done: 0, total, current: None });
    let mut hits: Vec<Hit> = code
        .par_iter()
        .flat_map_iter(|(f, lang)| {
            let found = check_file(&ctx, f, *lang).unwrap_or_else(|| {
                skipped.fetch_add(1, Ordering::Relaxed);
                vec![]
            });
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if let Ok(mut last) = last_report.try_lock()
                && (last.elapsed().as_millis() >= 50 || n == total)
            {
                *last = Instant::now();
                progress(Progress { step: Step::Code, done: n, total, current: Some(f.rel.clone()) });
            }
            found
        })
        .collect();
    progress(Progress { step: Step::Code, done: total, total, current: None });

    hits.extend(rules::files::check(&ctx));
    hits.extend(rules::scripts::npm_scripts(&ctx));
    hits.extend(rules::scripts::twins(&ctx));
    hits.extend(rules::scripts::python3_commands(&ctx));
    hits.extend(rules::scripts::make(&ctx));
    hits.extend(rules::scripts::line_endings(&ctx));
    hits.extend(rules::scripts::exec_bits(&ctx));
    hits.extend(rules::project::check(&ctx));

    // Packages: online only when asked.
    let deps = online::unique_deps(&snap.python_deps);
    progress(Progress {
        step: Step::Packages,
        done: 0,
        total: if opts.online { deps.len() as u64 } else { 0 },
        current: None,
    });
    let mut network = Network { online: opts.online, hosts: vec![] };
    let mut notes = vec!["Nothing in the project was run: no code, scripts, notebooks or package managers.".to_owned()];
    if opts.online && !deps.is_empty() {
        hits.retain(|h| h.rule != "wheels-unchecked");
        let default_source = online::HttpPypi::new();
        let source: &dyn online::PypiSource = opts.pypi.as_deref().unwrap_or(&default_source);
        let (online_hits, reached) = online::check(&deps, opts.target, source, &|done, name| {
            progress(Progress { step: Step::Packages, done, total: deps.len() as u64, current: Some(name.to_owned()) })
        });
        hits.extend(online_hits);
        if reached {
            network.hosts.push("pypi.org".into());
            notes.push("Online: read PyPI's public JSON API for package builds. Nothing else was contacted.".into());
        } else {
            notes.push("Online was asked for, but PyPI couldn't be reached; package builds weren't checked.".into());
        }
    } else if !opts.online {
        notes.push("Offline: nothing was looked up on the internet.".into());
    }

    progress(Progress { step: Step::Report, done: 0, total: 0, current: None });
    let (findings, counts) = to_findings(&hits, opts.target);
    let stats = Stats {
        files: inv.files.len() as u64,
        bytes: inv.files.iter().map(|f| f.size).sum(),
        parsed_files: total - skipped.load(Ordering::Relaxed),
        skipped_files: skipped.load(Ordering::Relaxed)
            + inv.files.iter().filter(|f| f.size > parse::MAX_PARSE_BYTES && is_code(&f.rel)).count() as u64,
        unreadable: inv.unreadable.clone(),
    };
    if !stats.unreadable.is_empty() {
        notes.push(format!("{} files or folders couldn't be read and weren't checked.", stats.unreadable.len()));
    }
    if counts.omitted > 0 {
        notes.push(format!("{} more places weren't listed: each rule lists up to {MAX_PER_RULE}.", counts.omitted));
    }
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| root.display().to_string());
    let mut report = Report {
        schema_version: SCHEMA_VERSION,
        tool: Tool { name: "portal-scan".into(), version: env!("CARGO_PKG_VERSION").into() },
        scanned_at: now_rfc3339(),
        project: ProjectInfo { name, path: root.display().to_string() },
        source: Os::current(),
        target: opts.target,
        network,
        duration_ms: 0,
        stats,
        tier: summary::tier(&snap, &inv, &findings),
        headline: String::new(),
        counts,
        ai_usage: summary::ai_usage_placeholder(),
        ecosystems: snap.ecosystems.clone(),
        runtimes: snap.runtimes.clone(),
        packages: snap.packages.clone(),
        run_commands: snap.run_commands.clone(),
        left_out: snap.left_out.clone(),
        secrets: snap.secrets.clone(),
        services: snap.services.clone(),
        findings,
        handoff: None,
        notes,
    };
    report.headline = summary::headline(&report);
    report.ai_usage = summary::ai_usage(&report);
    report.handoff = summary::handoff(&report);
    report.duration_ms = started.elapsed().as_millis() as u64;
    progress(Progress { step: Step::Report, done: 1, total: 1, current: None });
    Ok(report)
}

fn is_code(rel: &str) -> bool {
    parse::lang_for(rel, b"").is_some() || rel.ends_with(".ipynb")
}

/// The first bytes, for a shebang: only files without an extension need
/// one, and secret files (.env, keys) are never opened for it.
fn sniff(f: &walk::FileEntry) -> Vec<u8> {
    use std::io::Read;
    let name = f.rel.rsplit('/').next().unwrap_or(&f.rel);
    if name.contains('.') || snapshot::secret_kind(name).is_some() {
        return Vec::new();
    }
    let mut buf = vec![0u8; 256];
    let n = std::fs::File::open(&f.abs).and_then(|mut file| file.read(&mut buf)).unwrap_or(0);
    buf.truncate(n);
    buf
}

/// Parses one file and runs its language's rules. None: skipped (not text).
fn check_file(ctx: &Ctx<'_>, f: &walk::FileEntry, lang: Lang) -> Option<Vec<Hit>> {
    let bytes = std::fs::read(&f.abs).ok()?;
    if !parse::is_text(&bytes) {
        return None;
    }
    if f.rel.ends_with(".ipynb") {
        let json = String::from_utf8_lossy(&bytes);
        let mut out = Vec::new();
        for (cell, source) in rules::python::notebook_cells(&json) {
            let src = source.as_bytes();
            let Some(tree) = parse::parse(Lang::Python, src) else { continue };
            for mut h in rules::python::check(ctx, &f.rel, src, &tree) {
                h.evidence = h.evidence.map(|e| format!("[cell {cell}] {e}"));
                out.push(h);
            }
        }
        return Some(out);
    }
    let tree = parse::parse(lang, &bytes)?;
    Some(match lang {
        Lang::Python => rules::python::check(ctx, &f.rel, &bytes, &tree),
        Lang::JavaScript | Lang::TypeScript | Lang::Tsx => rules::js::check(&f.rel, &bytes, &tree),
        Lang::Bash => vec![],
    })
}

/// Hits → findings for the target: sorted, capped per rule, counted.
fn to_findings(hits: &[Hit], target: TargetOs) -> (Vec<Finding>, Counts) {
    let mut all: Vec<Finding> = hits.iter().filter_map(|h| rules::finding(h, target)).collect();
    all.sort_by(|a, b| {
        (a.state, a.severity, &a.rule, &a.file, a.line).cmp(&(b.state, b.severity, &b.rule, &b.file, b.line))
    });
    all.dedup_by(|a, b| a.id == b.id);
    let mut counts = Counts { occurrences: all.len() as u32, ..Default::default() };
    let mut groups: BTreeMap<(String, State), ()> = BTreeMap::new();
    let mut per_rule: HashMap<String, usize> = HashMap::new();
    let mut kept = Vec::new();
    for f in all {
        groups.insert((f.rule.clone(), f.state), ());
        let n = per_rule.entry(f.rule.clone()).or_default();
        *n += 1;
        if *n <= MAX_PER_RULE {
            kept.push(f);
        } else {
            counts.omitted += 1;
        }
    }
    for (_, state) in groups.keys() {
        match state {
            State::Blocker => counts.blocker += 1,
            State::WillNeedAFix => counts.will_need_a_fix += 1,
            State::NeedsYou => counts.needs_you += 1,
            State::CouldntCheck => counts.couldnt_check += 1,
            State::Info => counts.info += 1,
        }
    }
    (kept, counts)
}
