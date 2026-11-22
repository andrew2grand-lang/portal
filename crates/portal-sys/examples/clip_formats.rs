//! Prints what's on the clipboard by name only, as one JSON line: every
//! format (Windows) or type (macOS), the two "Can…" DWORDs on Windows, the
//! app that put it there, and what SeamDrop would make of it ("private" with
//! the marker, "files", "contents" or "empty"). Never the content.
//!
//!   cargo run -p portal-sys --example clip_formats
//!
//! `scripts/browser-markers.mjs` runs it after each browser copy (Mission 04 §4.3).

fn main() {
    let f = portal_sys::clipboard::formats();
    let verdict = match portal_sys::clipboard::read() {
        portal_sys::clipboard::Read::Private { marker, .. } => format!("private: {marker}"),
        portal_sys::clipboard::Read::Files(p) => format!("files: {}", p.len()),
        portal_sys::clipboard::Read::Contents { .. } => "contents".into(),
        portal_sys::clipboard::Read::Empty => "empty".into(),
        portal_sys::clipboard::Read::Busy => "busy".into(),
    };
    let names: Vec<String> =
        f.names.iter().map(|n| format!("\"{}\"", n.replace('\\', "\\\\").replace('"', "\\\""))).collect();
    let opt = |v: Option<u32>| v.map_or("null".to_owned(), |v| v.to_string());
    let source =
        f.source.map_or("null".to_owned(), |s| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")));
    println!(
        "{{\"formats\":[{}],\"canIncludeInClipboardHistory\":{},\"canUploadToCloudClipboard\":{},\"source\":{},\"seamdrop\":\"{}\"}}",
        names.join(","),
        opt(f.can_include_in_history),
        opt(f.can_upload_to_cloud),
        source,
        verdict
    );
}
