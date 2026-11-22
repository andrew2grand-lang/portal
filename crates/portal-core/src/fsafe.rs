//! Cross-OS file safety (brief §6.5): what may arrive on the wire, and what
//! a received name becomes on this machine. One module, a test per rule.

use std::collections::{HashMap, HashSet};

use unicode_normalization::UnicodeNormalization;

/// Whose naming rules a received name must satisfy: the receiver's file system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Target {
    /// NTFS: forbidden characters, reserved names, no trailing dot/space, 255 UTF-16 units.
    Windows,
    /// APFS: 255 UTF-8 bytes.
    Mac,
    Other,
}

impl Target {
    pub fn current() -> Target {
        if cfg!(windows) {
            Target::Windows
        } else if cfg!(target_os = "macos") {
            Target::Mac
        } else {
            Target::Other
        }
    }

    fn label(self) -> &'static str {
        match self {
            Target::Windows => "Windows",
            Target::Mac => "macOS",
            Target::Other => "this system",
        }
    }
}

/// Characters NTFS / Win32 won't take in a name (besides control characters).
const WINDOWS_FORBIDDEN: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const MAX_NAME: usize = 255;
/// Longer than this, an "extension" isn't one (keeps truncation sane).
const MAX_EXTENSION: usize = 32;

/// A name made safe for a target, with plain reasons for every change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sanitized {
    pub name: String,
    pub reasons: Vec<String>,
}

fn is_reserved_windows(base: &str) -> bool {
    let b = base.to_ascii_uppercase();
    if matches!(b.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let (head, tail) = (b.get(..3), b.get(3..));
    match (head, tail) {
        (Some("COM" | "LPT"), Some(t)) => {
            let mut chars = t.chars();
            matches!((chars.next(), chars.next()), (Some('0'..='9' | '¹' | '²' | '³'), None))
        }
        _ => false,
    }
}

fn describe_chars(chars: &[char]) -> String {
    let shown: Vec<String> = chars
        .iter()
        .map(|c| if (*c as u32) < 32 { "control characters".to_owned() } else { format!("‘{c}’") })
        .collect::<Vec<_>>()
        .into_iter()
        .fold(Vec::new(), |mut acc, s| {
            if !acc.contains(&s) {
                acc.push(s);
            }
            acc
        });
    match shown.as_slice() {
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        [] => String::new(),
    }
}

/// Makes one name safe on `target` (brief §6.5). NFC on every target; the
/// Windows rules only when the receiver is Windows.
pub fn sanitize(name: &str, target: Target) -> Sanitized {
    let mut reasons = Vec::new();
    let mut s: String = name.nfc().collect();
    if target == Target::Windows {
        let mut bad = Vec::new();
        let replaced: String = s
            .chars()
            .map(|c| {
                if WINDOWS_FORBIDDEN.contains(&c) || (c as u32) < 32 {
                    bad.push(c);
                    '_'
                } else {
                    c
                }
            })
            .collect();
        if !bad.is_empty() {
            let verb = if bad.len() == 1 && (bad[0] as u32) >= 32 { "isn’t" } else { "aren’t" };
            reasons.push(format!("{} {verb} allowed in Windows names", describe_chars(&bad)));
            s = replaced;
        }
        let trimmed = s.trim_end_matches(['.', ' ']);
        if trimmed.len() != s.len() {
            reasons.push("Windows names can’t end with a dot or a space".into());
            s = trimmed.to_owned();
        }
        if s.is_empty() {
            s = "_".into();
        }
        let base_end = s.find('.').unwrap_or(s.len());
        let base = s[..base_end].trim_end_matches(' ');
        if is_reserved_windows(base) {
            reasons.push(format!("{} is a reserved name on Windows", base.to_ascii_uppercase()));
            s.insert(base_end, '_');
        }
    }
    if s.is_empty() {
        s = "_".into();
    }
    if !fits(&s, target) {
        s = shorten(&s, "", false, target);
        reasons.push(format!("the name was too long for {}", target.label()));
    }
    Sanitized { name: s, reasons }
}

/// Length of a name as the target file system counts it.
fn units(s: &str, target: Target) -> usize {
    match target {
        Target::Windows => s.encode_utf16().count(),
        _ => s.len(),
    }
}

pub fn fits(name: &str, target: Target) -> bool {
    units(name, target) <= MAX_NAME
}

/// Splits off an extension the way `numbered` does (none for folders).
fn split_ext(name: &str, is_dir: bool) -> (&str, &str) {
    if is_dir {
        return (name, "");
    }
    match name.rfind('.').filter(|&i| i > 0 && name.len() - i <= MAX_EXTENSION) {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    }
}

/// `stem + suffix + ext` cut on a character boundary so it fits `target`.
fn shorten(name: &str, suffix: &str, is_dir: bool, target: Target) -> String {
    let (stem, ext) = split_ext(name, is_dir);
    let budget = MAX_NAME.saturating_sub(units(ext, target) + units(suffix, target));
    let mut out = String::new();
    for c in stem.chars() {
        let mut tmp = [0u16; 2];
        let width = if target == Target::Windows { c.encode_utf16(&mut tmp).len() } else { c.len_utf8() };
        if units(&out, target) + width > budget {
            break;
        }
        out.push(c);
    }
    format!("{out}{suffix}{ext}")
}

/// `numbered`, shortened if needed so the result still fits (brief: "re-check after adding (2)").
pub fn numbered_fitting(name: &str, n: u32, is_dir: bool, target: Target) -> String {
    let plain = numbered(name, n, is_dir);
    if fits(&plain, target) { plain } else { shorten(name, &format!(" ({n})"), is_dir, target) }
}

/// Why a path from the wire was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("empty path")]
    Empty,
    #[error("empty name")]
    EmptyComponent,
    #[error("“.” or “..” in a path")]
    DotComponent,
    #[error("forbidden character in a name")]
    Separator,
    #[error("drive letter or network path")]
    Absolute,
    #[error("path nested too deep")]
    TooDeep,
    #[error("name too long")]
    TooLong,
}

/// Deeper than this is refused outright (Windows can't open it anyway).
pub const MAX_DEPTH: usize = 128;
/// Hard cap on a single component from the wire, before local shortening.
pub const MAX_WIRE_COMPONENT: usize = 1024;

/// Relative paths travel as an array of components, never a joined string.
/// Anything that could escape the receive folder is refused.
pub fn validate_components(parts: &[String]) -> Result<(), PathError> {
    if parts.is_empty() {
        return Err(PathError::Empty);
    }
    if parts.len() > MAX_DEPTH {
        return Err(PathError::TooDeep);
    }
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            return Err(PathError::EmptyComponent);
        }
        if p == "." || p == ".." {
            return Err(PathError::DotComponent);
        }
        if p.len() > MAX_WIRE_COMPONENT {
            return Err(PathError::TooLong);
        }
        if p.contains(['/', '\\', '\0']) {
            return Err(PathError::Separator);
        }
        // "C:", "C:foo" (drive-relative), and anything with a drive prefix
        let b = p.as_bytes();
        if i == 0 && b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
            return Err(PathError::Absolute);
        }
    }
    Ok(())
}

/// OS clutter skipped by default (setting "Skip system files").
pub fn is_junk(name: &str) -> bool {
    const JUNK: &[&str] = &[
        ".DS_Store",
        "Icon\r",
        ".Spotlight-V100",
        ".Trashes",
        ".fseventsd",
        ".TemporaryItems",
        "Thumbs.db",
        "desktop.ini",
        "$RECYCLE.BIN",
        "System Volume Information",
    ];
    name.starts_with("._") || JUNK.iter().any(|j| j.eq_ignore_ascii_case(name))
}

/// `name (n).ext` for files, `name (n)` for folders and dot-files.
pub fn numbered(name: &str, n: u32, is_dir: bool) -> String {
    let split = if is_dir { None } else { name.rfind('.').filter(|&i| i > 0) };
    match split {
        Some(i) => format!("{} ({n}){}", &name[..i], &name[i..]),
        None => format!("{name} ({n})"),
    }
}

/// A name the receiver changed, for Details and the done screen.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rename {
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub reason: String,
}

/// Maps wire paths to local paths inside one transfer: makes each name safe
/// and unique within its folder, and remembers folders it already mapped.
/// Saved with a paused transfer, so names stay the same when it resumes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Namer {
    target: Target,
    #[serde(with = "pairs")]
    dirs: HashMap<Vec<String>, Vec<String>>,
    #[serde(with = "pairs")]
    used: HashMap<Vec<String>, HashSet<String>>,
    /// Local top-level names, in arrival order.
    pub top: Vec<(String, bool)>,
    pub renames: Vec<Rename>,
}

impl Default for Namer {
    fn default() -> Self {
        Namer::new(Target::current())
    }
}

impl Namer {
    pub fn new(target: Target) -> Self {
        Namer { target, dirs: HashMap::new(), used: HashMap::new(), top: Vec::new(), renames: Vec::new() }
    }

    pub fn target(&self) -> Target {
        self.target
    }

    pub fn map_dir(&mut self, wire: &[String]) -> Vec<String> {
        if let Some(local) = self.dirs.get(wire) {
            return local.clone();
        }
        let parent = if wire.len() > 1 { self.map_dir(&wire[..wire.len() - 1]) } else { Vec::new() };
        let name = self.name_in(&parent, wire, true);
        let mut local = parent;
        local.push(name);
        self.dirs.insert(wire.to_vec(), local.clone());
        local
    }

    pub fn map_file(&mut self, wire: &[String]) -> Vec<String> {
        let parent = if wire.len() > 1 { self.map_dir(&wire[..wire.len() - 1]) } else { Vec::new() };
        let name = self.name_in(&parent, wire, false);
        let mut local = parent;
        local.push(name);
        local
    }

    fn name_in(&mut self, parent: &[String], wire: &[String], is_dir: bool) -> String {
        let original = wire.last().expect("validated path");
        let safe = sanitize(original, self.target);
        let used = self.used.entry(parent.to_vec()).or_default();
        let mut name = safe.name.clone();
        let mut n = 2;
        while used.contains(&fold(&name)) {
            name = numbered_fitting(&safe.name, n, is_dir, self.target);
            n += 1;
        }
        used.insert(fold(&name));
        let nfc: String = original.nfc().collect();
        if name != nfc {
            let mut reasons = safe.reasons;
            if n > 2 {
                reasons.push("another item in this transfer had the same name".into());
            }
            let mut to = parent.to_vec();
            to.push(name.clone());
            self.renames.push(Rename { from: wire.to_vec(), to, reason: reasons.join("; ") });
        }
        if parent.is_empty() {
            self.top.push((name.clone(), is_dir));
        }
        name
    }
}

/// Comparison key for collisions: Windows and macOS file systems ignore case
/// and (APFS) normalisation.
/// How names compare on case-insensitive file systems (NFC, lowercase).
pub fn fold(name: &str) -> String {
    name.nfc().collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normal_relative_paths_are_fine() {
        assert_eq!(validate_components(&v(&["Photos", "2024", "IMG_0001.jpg"])), Ok(()));
        assert_eq!(validate_components(&v(&["a..b", ".hidden", "…"])), Ok(()));
    }

    #[test]
    fn traversal_and_absolute_paths_are_refused() {
        assert_eq!(validate_components(&[]), Err(PathError::Empty));
        assert_eq!(validate_components(&v(&["a", ""])), Err(PathError::EmptyComponent));
        assert_eq!(validate_components(&v(&["..", "etc"])), Err(PathError::DotComponent));
        assert_eq!(validate_components(&v(&["a", "."])), Err(PathError::DotComponent));
        assert_eq!(validate_components(&v(&["a/../../b"])), Err(PathError::Separator));
        assert_eq!(validate_components(&v(&["a\\..\\b"])), Err(PathError::Separator));
        assert_eq!(validate_components(&v(&["a\0b"])), Err(PathError::Separator));
        assert_eq!(validate_components(&v(&["C:"])), Err(PathError::Absolute));
        assert_eq!(validate_components(&v(&["c:Windows"])), Err(PathError::Absolute));
        assert_eq!(validate_components(&v(&["\\\\server", "share"])), Err(PathError::Separator));
        assert_eq!(validate_components(&vec!["d".to_string(); MAX_DEPTH + 1]), Err(PathError::TooDeep));
    }

    #[test]
    fn os_junk_is_recognised() {
        for j in [
            ".DS_Store",
            "._report.pdf",
            "Icon\r",
            ".Spotlight-V100",
            ".Trashes",
            ".fseventsd",
            ".TemporaryItems",
            "Thumbs.db",
            "thumbs.db",
            "desktop.ini",
            "$RECYCLE.BIN",
            "System Volume Information",
        ] {
            assert!(is_junk(j), "{j:?}");
        }
        for ok in ["report.pdf", "Icon", "DS_Store", "my.desktop.ini.txt"] {
            assert!(!is_junk(ok), "{ok:?}");
        }
    }

    #[test]
    fn numbering_keeps_the_extension() {
        assert_eq!(numbered("notes.txt", 2, false), "notes (2).txt");
        assert_eq!(numbered("archive.tar.gz", 3, false), "archive.tar (3).gz");
        assert_eq!(numbered(".bashrc", 2, false), ".bashrc (2)");
        assert_eq!(numbered("Photos", 2, true), "Photos (2)");
        assert_eq!(numbered("v1.2", 2, true), "v1.2 (2)");
    }

    #[test]
    fn duplicate_names_in_one_transfer_get_numbers_and_folders_are_remembered() {
        let mut n = Namer::default();
        assert_eq!(n.map_file(&v(&["a.txt"])), v(&["a.txt"]));
        assert_eq!(n.map_file(&v(&["A.TXT"])), v(&["A (2).TXT"]), "case-insensitive file systems");
        assert_eq!(n.map_file(&v(&["Docs", "x.md"])), v(&["Docs", "x.md"]));
        assert_eq!(n.map_file(&v(&["Docs", "y.md"])), v(&["Docs", "y.md"]), "same folder reused");
        assert_eq!(n.renames.len(), 1);
        assert_eq!(
            n.top,
            vec![("a.txt".to_string(), false), ("A (2).TXT".to_string(), false), ("Docs".to_string(), true)]
        );
    }
}

/// A map whose keys aren't strings, saved as a list of pairs (JSON objects
/// only take string keys).
mod pairs {
    use std::collections::HashMap;
    use std::hash::Hash;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, K: Serialize, V: Serialize>(map: &HashMap<K, V>, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(map.iter())
    }

    pub fn deserialize<'de, D, K, V>(d: D) -> Result<HashMap<K, V>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}
