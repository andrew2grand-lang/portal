//! The report's promises: never reads secret values, never runs anything,
//! never follows links out, reads packages statically, and keeps its format.
mod common;

use std::path::Path;
use std::time::Instant;

use common::{scan_dir, scan_fixture};
use portal_scan::report::{Report, State, TargetOs};
use portal_scan::{ScanOptions, online, scan};

#[test]
fn secret_values_never_reach_the_report() {
    let (report, _tmp) = scan_fixture("broken", "secrets", TargetOs::Windows).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    for value in ["sk-live-DO-NOT-LEAK-123", "hunter2", "NOT-A-REAL-KEY-BUT-NEVER-READ", "secret-token-value"] {
        assert!(!json.contains(value), "a secret value leaked: {value}");
    }
    let env = report.secrets.iter().find(|s| s.path == ".env").expect(".env found even though it's gitignored");
    assert_eq!(env.keys, ["OPENAI_API_KEY", "DATABASE_URL", "DEBUG"], "key names only");
    assert!(!report.secrets.iter().any(|s| s.path == ".env.example"), "templates aren't secrets");
    let text = portal_scan::text::render(&report);
    assert!(!text.contains("hunter2") && !text.contains("sk-live"), "nor the printed summary");
}

#[test]
fn nothing_in_the_project_is_ever_run() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("trap");
    let marker = tmp.path().join("RAN");
    std::fs::create_dir_all(&root).unwrap();
    let m = marker.display().to_string().replace('\\', "/");
    std::fs::write(root.join("setup.py"), format!("open(r'{m}', 'w').write('ran')\n")).unwrap();
    std::fs::write(root.join("conftest.py"), format!("open(r'{m}', 'w').write('ran')\n")).unwrap();
    std::fs::write(
        root.join("package.json"),
        format!(r#"{{"name":"trap","scripts":{{"postinstall":"node -e \"require('fs').writeFileSync('{m}','x')\"","start":"touch {m}"}}}}"#),
    )
    .unwrap();
    std::fs::write(root.join("Makefile"), format!("run:\n\ttouch {m}\n")).unwrap();
    std::fs::write(root.join("run.sh"), format!("#!/bin/sh\ntouch {m}\n")).unwrap();
    let report = scan_dir(&root, TargetOs::Windows);
    assert!(!marker.exists(), "the scanner ran project code");
    assert!(report.notes.iter().any(|n| n.starts_with("Nothing in the project was run")));
    assert!(report.network.hosts.is_empty() && !report.network.online, "and went nowhere");
}

#[cfg(unix)]
#[test]
fn links_out_of_the_project_are_not_followed() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    for i in 0..5 {
        std::fs::write(outside.join(format!("private-{i}.py")), "import fcntl\n").unwrap();
    }
    let root = tmp.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.py"), "print('hi')\n").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
    std::os::unix::fs::symlink(&root, root.join("loop")).unwrap();
    let report = scan_dir(&root, TargetOs::Windows);
    assert_eq!(report.stats.files, 1, "only main.py");
    assert!(!report.findings.iter().any(|f| f.rule == "unix-module"), "nothing read through the link");
    let links: Vec<_> = report.findings.iter().filter(|f| f.rule == "symlink").collect();
    assert_eq!(links.len(), 2);
    assert!(links.iter().any(|f| f.why.contains("points outside")));
}

#[test]
fn packages_are_read_from_metadata_and_left_out_folders_are_sized() {
    let (report, _tmp) = scan_fixture("broken", "env-folders", TargetOs::Windows).unwrap();
    let py = report.packages.iter().find(|p| p.kind == "python").unwrap();
    assert_eq!(py.source, ".venv");
    assert_eq!(py.installed, 1);
    assert_eq!(py.items[0].name, "requests");
    assert_eq!(py.items[0].version.as_deref(), Some("2.32.3"));
    assert_eq!(py.lockfile, None, "unpinned requirements aren't a lock");
    let node = report.packages.iter().find(|p| p.kind == "node").unwrap();
    assert_eq!(node.items[0].name, "left-pad");
    let venv = report.left_out.items.iter().find(|i| i.path == ".venv").unwrap();
    assert!(venv.rebuildable && venv.files == 3 && venv.bytes > 0);
    assert!(report.left_out.items.iter().any(|i| i.path == "node_modules" && i.kind == "nodeModules"));
    assert_eq!(report.stats.files, 3, "requirements.txt, app.py, package.json and nothing from the environments");
}

#[test]
fn case_clashes_are_caught_where_the_disk_allows_them() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("README.md"), "a").unwrap();
    std::fs::write(root.join("Readme.md"), "b").unwrap();
    let both_exist = std::fs::read_dir(root).unwrap().count() == 2;
    let report = scan_dir(root, TargetOs::Windows);
    if both_exist {
        let f = report.findings.iter().find(|f| f.rule == "case-clash").expect("case clash");
        assert_eq!(f.state, State::Blocker);
    } else {
        // A case-insensitive disk (the usual Mac) can't even hold both.
        assert!(!report.findings.iter().any(|f| f.rule == "case-clash"));
    }
}

#[test]
fn an_empty_folder_gives_an_honest_empty_report() {
    let tmp = tempfile::tempdir().unwrap();
    let report = scan_dir(tmp.path(), TargetOs::Windows);
    assert_eq!(report.stats.files, 0);
    assert!(report.findings.is_empty());
    assert_eq!(report.headline, "This folder is empty, so there was nothing to check.");
    assert_eq!(report.tier.tier, portal_scan::report::Tier::Static);
    assert_eq!(report.tier.reason, "Static: an empty folder");
}

#[cfg(unix)]
#[test]
fn unreadable_files_are_reported_not_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("p");
    std::fs::create_dir_all(root.join("locked")).unwrap();
    std::fs::write(root.join("locked/a.py"), "import fcntl\n").unwrap();
    std::fs::write(root.join("ok.py"), "print(1)\n").unwrap();
    std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o000)).unwrap();
    let report = scan_dir(&root, TargetOs::Windows);
    std::fs::set_permissions(root.join("locked"), std::fs::Permissions::from_mode(0o755)).unwrap();
    if std::fs::read_dir(root.join("locked")).is_ok() && unsafe_is_root() {
        return; // root reads everything; nothing to test
    }
    assert!(report.stats.unreadable.iter().any(|u| u.contains("locked")), "{:?}", report.stats.unreadable);
    assert!(report.notes.iter().any(|n| n.contains("couldn't be read")));
}

#[cfg(unix)]
fn unsafe_is_root() -> bool {
    std::env::var("USER").is_ok_and(|u| u == "root")
}

/// Canned PyPI answers: the online check without the network.
struct FakePypi;
impl online::PypiSource for FakePypi {
    fn project(&self, name: &str, _version: Option<&str>) -> Result<Option<serde_json::Value>, String> {
        let files: &[(&str, &str)] = match name {
            "numpy" => &[
                ("numpy-2.1.0-cp312-cp312-win_amd64.whl", "bdist_wheel"),
                ("numpy-2.1.0-cp312-cp312-macosx_14_0_arm64.whl", "bdist_wheel"),
            ],
            "pyobjc-core" => &[
                ("pyobjc_core-10.3-cp312-cp312-macosx_10_9_universal2.whl", "bdist_wheel"),
                ("pyobjc-core-10.3.tar.gz", "sdist"),
            ],
            "oldlib" => &[("oldlib-0.1.tar.gz", "sdist")],
            "requests" => &[("requests-2.32.3-py3-none-any.whl", "bdist_wheel")],
            _ => return Ok(None),
        };
        Ok(Some(
            serde_json::json!({ "version": "1.0", "urls": files.iter().map(|(f, k)| serde_json::json!({"filename": f, "packagetype": k})).collect::<Vec<_>>() }),
        ))
    }
}

#[test]
fn online_package_checks_name_what_has_to_be_compiled() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("requirements.txt"),
        "numpy==2.1.0\npyobjc-core==10.3\noldlib\nrequests\nour-private-lib\n",
    )
    .unwrap();
    std::fs::write(tmp.path().join("app.py"), "import numpy\n").unwrap();
    let mut opts = ScanOptions::new(TargetOs::Windows);
    opts.host_versions = false;
    opts.online = true;
    opts.pypi = Some(Box::new(FakePypi));
    let report = scan(tmp.path(), &opts, &|_| {}).unwrap();
    let rule_for = |pkg: &str| {
        report
            .findings
            .iter()
            .find(|f| f.evidence.as_deref().is_some_and(|e| e.starts_with(pkg)))
            .map(|f| f.rule.as_str())
    };
    assert_eq!(rule_for("pyobjc-core"), Some("no-wheel"));
    assert_eq!(rule_for("oldlib"), Some("source-only"));
    assert_eq!(rule_for("our-private-lib"), Some("package-not-found"));
    assert_eq!(rule_for("numpy"), None);
    assert_eq!(rule_for("requests"), None);
    assert!(!report.findings.iter().any(|f| f.rule == "wheels-unchecked"), "it was checked");
    assert_eq!(report.network.hosts, ["pypi.org"]);
    let compiled = report.findings.iter().find(|f| f.rule == "no-wheel").unwrap();
    assert!(compiled.why.contains("Microsoft C++ Build Tools"), "{}", compiled.why);
}

#[test]
fn the_schema_file_matches_the_report_types() {
    let schema = serde_json::to_string_pretty(&schemars::schema_for!(Report)).unwrap() + "\n";
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/scan-report.schema.json");
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, &schema).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    assert_eq!(on_disk, schema, "docs/scan-report.schema.json is stale: run with UPDATE_GOLDEN=1");
}

/// Timings, paths and host details vary; everything else must not change unnoticed.
fn normalised(mut r: Report) -> serde_json::Value {
    r.scanned_at = "<time>".into();
    r.duration_ms = 0;
    r.project.path = "<root>".into();
    r.source = portal_scan::report::Os::Other;
    let mut v = serde_json::to_value(&r).unwrap();
    // Folder sizes differ between file systems.
    if let Some(items) = v.pointer_mut("/leftOut/items").and_then(|i| i.as_array_mut()) {
        for i in items {
            i["bytes"] = 0.into();
        }
    }
    v["leftOut"]["bytes"] = 0.into();
    v["stats"]["bytes"] = 0.into();
    // Every release bumps it.
    v["tool"]["version"] = "<version>".into();
    v
}

#[test]
fn golden_reports_keep_their_shape() {
    let cases = [
        ("clean", "python-cli", TargetOs::Windows),
        ("broken", "secrets", TargetOs::Windows),
        ("broken", "native-xcode", TargetOs::Windows),
        ("broken", "npm-scripts", TargetOs::Windows),
    ];
    let golden_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut changed = Vec::new();
    for (kind, name, target) in cases {
        let Some((report, _tmp)) = scan_fixture(kind, name, target) else { continue };
        let got = serde_json::to_string_pretty(&normalised(report)).unwrap() + "\n";
        let path = golden_dir.join(format!("{name}.json"));
        if std::env::var("UPDATE_GOLDEN").is_ok() {
            std::fs::create_dir_all(&golden_dir).unwrap();
            std::fs::write(&path, &got).unwrap();
        }
        let want = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
        if want != got {
            changed.push(name);
        }
    }
    assert!(changed.is_empty(), "golden reports changed: {changed:?} (UPDATE_GOLDEN=1 to accept)");
}

#[test]
fn the_portal_repo_scans_quickly() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let started = Instant::now();
    let report = scan_dir(&repo, TargetOs::Windows);
    let took = started.elapsed();
    eprintln!(
        "portal repo: {} files, {} parsed, in {took:?} (report says {} ms)",
        report.stats.files, report.stats.parsed_files, report.duration_ms
    );
    assert!(report.stats.files > 100);
    assert!(took.as_secs() < 30, "took {took:?}");
}

#[test]
fn five_thousand_files_scan_in_seconds() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    for i in 0..5000 {
        let dir = root.join(format!("pkg{}", i % 50));
        std::fs::create_dir_all(&dir).unwrap();
        let (name, body) = match i % 4 {
            0 => (
                format!("mod_{i}.py"),
                format!("import os\nfrom pathlib import Path\n\n\ndef f_{i}(p):\n    return Path(p) / \"x{i}\"\n"),
            ),
            1 => (
                format!("util_{i}.js"),
                format!("const path = require('path')\nmodule.exports = (d) => path.join(d, 'x{i}')\n"),
            ),
            2 => (format!("comp_{i}.ts"), format!("export const v{i}: number = {i}\n")),
            _ => (format!("notes_{i}.md"), format!("# Note {i}\n")),
        };
        std::fs::write(dir.join(name), body).unwrap();
    }
    let started = Instant::now();
    let report = scan_dir(root, TargetOs::Windows);
    let took = started.elapsed();
    eprintln!("5,000 files: {} parsed in {took:?}", report.stats.parsed_files);
    assert_eq!(report.stats.files, 5000);
    assert_eq!(report.stats.parsed_files, 3750);
    assert!(took.as_secs() < 30, "took {took:?}");
}

#[test]
fn tier_reasons_say_what_is_in_the_folder() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("Handbook");
    for (rel, body) in [
        ("README.md", "# Handbook\n"),
        ("guide.md", "# Guide\n"),
        ("site/index.html", "<h1>Hi</h1>\n"),
        ("site/style.css", "h1 {}\n"),
    ] {
        std::fs::create_dir_all(docs.join(rel).parent().unwrap()).unwrap();
        std::fs::write(docs.join(rel), body).unwrap();
    }
    std::fs::write(docs.join("map.png"), [0u8, 1, 2]).unwrap();
    let r = scan_dir(&docs, TargetOs::Windows);
    assert_eq!(r.tier.reason, "Static: documents, images and a small website; nothing to run");

    let tool = tmp.path().join("tool");
    std::fs::create_dir_all(&tool).unwrap();
    for name in ["tool.py", "start.bat", "fetch.ps1", "fetch.sh"] {
        std::fs::write(tool.join(name), "x = 1\n").unwrap();
    }
    let r = scan_dir(&tool, TargetOs::Macos);
    assert_eq!(r.tier.reason, "Scripts: 1 Python script and 3 command scripts");
}

#[test]
fn env_files_give_names_only_even_with_multi_line_values() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("app");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join(".env"),
        "API_KEY=abc\nPRIVATE_KEY=\"-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC7\n\
         9mxDXDf6AU0cN7RPBjb9qSHDcWZHGzUCIG2Es59z8ugGrDYpxLQnwfotadxdUy==\n-----END PRIVATE KEY-----\"\n\
         DEBUG=1\nOTHER='multi\nline=value'\nexport LAST=\"one line\"\n",
    )
    .unwrap();
    let report = scan_dir(&root, TargetOs::Windows);
    assert_eq!(report.secrets[0].keys, ["API_KEY", "PRIVATE_KEY", "DEBUG", "OTHER", "LAST"]);
    let json = serde_json::to_string(&report).unwrap() + &portal_scan::text::render(&report);
    for piece in ["9mxDXDf6", "MIIEvQ", "line=value", "multi"] {
        assert!(!json.contains(piece), "part of a value leaked: {piece}");
    }
}

#[cfg(unix)]
#[test]
fn installed_packages_are_never_read_through_links_out_of_the_project() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    std::fs::create_dir_all(outside.join("extpkg")).unwrap();
    std::fs::write(outside.join("extpkg/package.json"), r#"{"name":"extpkg","version":"9.9.9-outside"}"#).unwrap();
    std::fs::create_dir_all(outside.join("foo-1.0.dist-info")).unwrap();
    std::fs::write(outside.join("foo-1.0.dist-info/METADATA"), "Name: foo\nVersion: 6.6.6-outside\n").unwrap();
    let root = tmp.path().join("app");
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::fs::create_dir_all(root.join(".venv/lib/python3.12/site-packages")).unwrap();
    std::fs::write(root.join(".venv/pyvenv.cfg"), "version = 3.12.7\n").unwrap();
    std::fs::write(root.join("package.json"), r#"{"name":"app","dependencies":{"extpkg":"^9"}}"#).unwrap();
    std::fs::write(root.join("requirements.txt"), "foo==6.6.6\n").unwrap();
    std::os::unix::fs::symlink(outside.join("extpkg"), root.join("node_modules/extpkg")).unwrap();
    std::os::unix::fs::symlink(
        outside.join("foo-1.0.dist-info"),
        root.join(".venv/lib/python3.12/site-packages/foo-1.0.dist-info"),
    )
    .unwrap();
    let json = serde_json::to_string(&scan_dir(&root, TargetOs::Windows)).unwrap();
    assert!(!json.contains("outside"), "read through a link out of the project: {json}");
}

#[cfg(unix)]
#[test]
fn a_fifo_never_hangs_the_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("app");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.py"), "print(1)\n").unwrap();
    for name in [".env", "server.key", "run"] {
        let made = std::process::Command::new("mkfifo").arg(root.join(name)).status().unwrap();
        assert!(made.success());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let dir = root.clone();
    std::thread::spawn(move || tx.send(scan_dir(&dir, TargetOs::Windows)).unwrap());
    let report = rx.recv_timeout(std::time::Duration::from_secs(10)).expect("the scan finished");
    assert_eq!(report.stats.files, 1, "only the regular file counts");
}
