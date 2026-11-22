//! Reads the product name from the app's Tauri config, so it's set in one place.

fn main() {
    let conf = concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri/tauri.conf.json");
    println!("cargo:rerun-if-changed={conf}");
    let text = std::fs::read_to_string(conf).unwrap_or_else(|e| panic!("reading {conf}: {e}"));
    let name = text
        .lines()
        .find_map(|l| l.trim().strip_prefix("\"productName\":"))
        .and_then(|v| v.trim().trim_end_matches(',').trim().strip_prefix('"')?.strip_suffix('"').map(str::to_owned))
        .unwrap_or_else(|| panic!("no \"productName\" in {conf}"));
    println!("cargo:rustc-env=SEAMDROP_APP_NAME={name}");
}
