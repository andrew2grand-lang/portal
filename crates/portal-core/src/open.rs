//! Which received files the done screen may **Open** (Mission 05 §1.6).
//!
//! SeamDrop doesn't mark received files the way browsers mark downloads (the
//! macOS quarantine flag, Windows' Mark of the Web), so opening a received
//! app or script would run it with no Gatekeeper or SmartScreen check.
//! Rather than mark every file (Office then opens them all in Protected
//! View, and a sender's own documents would look like strangers'), Open is
//! offered only for documents, images and media, by an allowlist of
//! extensions: never apps, installers, scripts, shortcuts, archives or web
//! pages. Anything else is one click away with Show in Finder / File
//! Explorer, where the system's own checks apply as usual.

use std::path::Path;

/// Documents, images, audio and video that open in a viewer, not run.
const OPENABLE: &[&str] = &[
    // documents
    "pdf", "txt", "md", "rtf", "csv", "tsv", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "pages",
    "numbers", "key", "epub", // images (not svg: a browser runs its scripts)
    "jpg", "jpeg", "png", "gif", "heic", "heif", "webp", "tif", "tiff", "bmp", "avif", // audio
    "mp3", "m4a", "aac", "wav", "flac", "aiff", "aif", "ogg", "opus", // video
    "mp4", "m4v", "mov", "mkv", "avi", "webm", "wmv",
];

/// True if a received file with this name may be opened from the window.
/// Only the last extension counts ("invoice.pdf.app" is an app), and a
/// name without one never opens.
pub fn safe_to_open(name: &str) -> bool {
    let Some(ext) = Path::new(name).extension().and_then(|e| e.to_str()) else { return false };
    let ext = ext.to_ascii_lowercase();
    OPENABLE.contains(&ext.as_str())
}

#[cfg(test)]
mod tests {
    use super::safe_to_open;

    #[test]
    fn documents_images_and_media_open() {
        for n in ["report.PDF", "notes.txt", "photo.heic", "clip.mov", "song.m4a", "Budget.xlsx", "slides.key"] {
            assert!(safe_to_open(n), "{n}");
        }
    }

    #[test]
    fn apps_installers_scripts_shortcuts_archives_and_pages_never_open() {
        for n in [
            "Tool.app",
            "setup.exe",
            "installer.msi",
            "pkg.pkg",
            "disk.dmg",
            "run.sh",
            "run.command",
            "x.bat",
            "x.cmd",
            "x.ps1",
            "x.vbs",
            "x.js",
            "x.jar",
            "x.py",
            "link.lnk",
            "link.url",
            "site.webloc",
            "x.scr",
            "a.zip",
            "a.7z",
            "a.tar.gz",
            "a.rar",
            "page.html",
            "page.htm",
            "image.svg",
            "invoice.pdf.app",
            "invoice.pdf.exe",
            "README",
            ".hidden",
            "x.reg",
            "x.workflow",
            "x.applescript",
            "x.scpt",
            "x.terminal",
            "x.iso",
        ] {
            assert!(!safe_to_open(n), "{n}");
        }
    }
}
