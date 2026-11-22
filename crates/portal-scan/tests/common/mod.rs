#![allow(dead_code)]
pub mod fixture;

use std::path::Path;

use portal_scan::report::{Report, TargetOs};
use portal_scan::{ScanOptions, scan};

/// Scans a folder the way tests want: no host lookups, offline.
pub fn scan_dir(dir: &Path, target: TargetOs) -> Report {
    let mut opts = ScanOptions::new(target);
    opts.host_versions = false;
    scan(dir, &opts, &|_| {}).expect("scan runs")
}

/// Materialises a fixture and scans it. None if it can't exist on this OS.
pub fn scan_fixture(kind: &str, name: &str, target: TargetOs) -> Option<(Report, tempfile::TempDir)> {
    let src = fixture::fixtures_dir().join(kind).join(name);
    let tmp = tempfile::Builder::new().prefix(&format!("{name}-")).tempdir().unwrap();
    let dst = tmp.path().join(name);
    if !fixture::materialize(&src, &dst).unwrap() {
        return None;
    }
    Some((scan_dir(&dst, target), tmp))
}
