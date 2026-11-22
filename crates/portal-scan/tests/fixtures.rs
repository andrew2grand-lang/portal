//! The Phase 1 gate: every planted problem is found; clean projects raise
//! no blockers and nothing to fix, in either direction.
mod common;

use common::{fixture, scan_fixture};
use portal_scan::report::{Finding, State, TargetOs};

fn target_of(v: &serde_json::Value) -> TargetOs {
    match v.get("target").and_then(|t| t.as_str()) {
        Some("macos") => TargetOs::Macos,
        _ => TargetOs::Windows,
    }
}

fn state_name(s: State) -> &'static str {
    match s {
        State::Blocker => "blocker",
        State::WillNeedAFix => "willNeedAFix",
        State::NeedsYou => "needsYou",
        State::CouldntCheck => "couldntCheck",
        State::Info => "info",
    }
}

fn matches(f: &Finding, want: &serde_json::Value) -> bool {
    let s = |k: &str| want.get(k).and_then(|v| v.as_str());
    s("rule") == Some(f.rule.as_str())
        && s("file").is_none_or(|file| f.file.as_deref() == Some(file))
        && want.get("line").and_then(|v| v.as_u64()).is_none_or(|l| f.line == Some(l as u32))
        && s("state").is_none_or(|st| st == state_name(f.state))
}

#[test]
fn every_planted_problem_is_found() {
    let dir = fixture::fixtures_dir().join("broken");
    let mut names: Vec<String> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let mut missing = Vec::new();
    let mut checked = 0;
    for name in &names {
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(name).join("expected.json")).unwrap()).unwrap();
        let target = target_of(&expected);
        let Some((report, _tmp)) = scan_fixture("broken", name, target) else { continue };
        checked += 1;
        for want in expected["expect"].as_array().unwrap() {
            if cfg!(windows) && want.get("unix").and_then(|v| v.as_bool()) == Some(true) {
                continue;
            }
            if !report.findings.iter().any(|f| matches(f, want)) {
                let got: Vec<String> = report
                    .findings
                    .iter()
                    .map(|f| {
                        format!(
                            "{} {}:{} {}",
                            f.rule,
                            f.file.as_deref().unwrap_or("-"),
                            f.line.unwrap_or(0),
                            state_name(f.state)
                        )
                    })
                    .collect();
                missing.push(format!("{name}: expected {want}\n    got: {got:?}"));
            }
        }
    }
    assert!(checked >= 20, "only {checked} fixtures ran");
    assert!(missing.is_empty(), "planted problems not found:\n{}", missing.join("\n"));
}

#[test]
fn clean_projects_raise_no_blockers_and_nothing_to_fix() {
    let dir = fixture::fixtures_dir().join("clean");
    let mut names: Vec<String> =
        std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    assert!(names.len() >= 10, "need at least 10 clean samples");
    let mut wrong = Vec::new();
    for name in &names {
        for target in [TargetOs::Windows, TargetOs::Macos] {
            let Some((report, _tmp)) = scan_fixture("clean", name, target) else { continue };
            for f in report.findings.iter().filter(|f| matches!(f.state, State::Blocker | State::WillNeedAFix)) {
                wrong.push(format!(
                    "{name} → {target:?}: {} {}:{} {:?}",
                    f.rule,
                    f.file.as_deref().unwrap_or("-"),
                    f.line.unwrap_or(0),
                    f.evidence
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "false alarms on clean samples:\n{}", wrong.join("\n"));
}

/// Every stored fixture file must survive a Windows checkout: no reserved
/// names, no characters Windows forbids, and short enough paths. Names that
/// need those go under `renames` in the fixture's `_manifest.json`.
#[test]
fn fixture_files_can_be_checked_out_on_windows() {
    const RESERVED: &[&str] = &[
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1",
        "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    fn visit(dir: &std::path::Path, root: &std::path::Path, bad: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let stem = name.split('.').next().unwrap_or("").to_ascii_lowercase();
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let illegal = name.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\') || c < ' ')
                || name.ends_with(['.', ' '])
                || RESERVED.contains(&stem.as_str());
            // The repo sits at D:\a\portal\portal\ on CI runners; keep well under 260.
            if illegal || rel.len() > 180 {
                bad.push(rel);
            }
            if entry.file_type().unwrap().is_dir() {
                visit(&path, root, bad);
            }
        }
    }
    let fixtures = fixture::fixtures_dir().canonicalize().unwrap();
    let repo = fixtures.join("../../../..").canonicalize().unwrap();
    let mut bad = Vec::new();
    visit(&fixtures, &repo, &mut bad);
    assert!(bad.is_empty(), "Windows can't check these out: {bad:?}");
}
