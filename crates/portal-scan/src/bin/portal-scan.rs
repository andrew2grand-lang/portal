//! portal-scan <folder> [--target windows|macos] [--json] [--online] [--progress]
//!
//! Reads a project folder (never runs it) and reports what blocks it on the
//! other OS. --progress writes one JSON line per step to stderr.

use std::path::PathBuf;
use std::process::ExitCode;

use portal_scan::report::TargetOs;
use portal_scan::{ScanOptions, scan, text};

const USAGE: &str = "usage: portal-scan <folder> [--target windows|macos] [--json] [--online] [--progress]

  --target   the OS the project should run on next (default: the other one)
  --json     print the versioned JSON report instead of the summary
  --online   check Python packages for ready-made builds on PyPI (nothing else is contacted)
  --progress write progress as JSON lines on stderr";

fn main() -> ExitCode {
    let mut folder: Option<PathBuf> = None;
    let mut target = TargetOs::other_than_here();
    let (mut json, mut online, mut progress) = (false, false, false);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--json" => json = true,
            "--online" => online = true,
            "--progress" => progress = true,
            "--target" => match args.next().as_deref() {
                Some("windows") => target = TargetOs::Windows,
                Some("macos" | "mac") => target = TargetOs::Macos,
                other => {
                    eprintln!("--target needs windows or macos, not {}\n\n{USAGE}", other.unwrap_or("nothing"));
                    return ExitCode::from(2);
                }
            },
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option {other}\n\n{USAGE}");
                return ExitCode::from(2);
            }
            other => folder = Some(PathBuf::from(other)),
        }
    }
    let Some(folder) = folder else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let mut opts = ScanOptions::new(target);
    opts.online = online;
    let report_progress = |p: portal_scan::Progress| {
        if progress && let Ok(line) = serde_json::to_string(&p) {
            eprintln!("{line}");
        }
    };
    match scan(&folder, &opts, &report_progress) {
        Ok(report) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&report).expect("report serialises"));
            } else {
                print!("{}", text::render(&report));
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("portal-scan: {e}");
            ExitCode::from(1)
        }
    }
}
