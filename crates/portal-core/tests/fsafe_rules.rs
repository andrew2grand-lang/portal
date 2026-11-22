//! Brief §6.5: every file-name rule, one test each.

use portal_core::fsafe::{Namer, Target, numbered_fitting, sanitize};

fn win(name: &str) -> String {
    sanitize(name, Target::Windows).name
}

fn mac(name: &str) -> String {
    sanitize(name, Target::Mac).name
}

fn utf16(s: &str) -> usize {
    s.encode_utf16().count()
}

// ---- Going to Windows -------------------------------------------------------

#[test]
fn windows_forbidden_characters_become_underscores() {
    for c in ['<', '>', ':', '"', '|', '?', '*'] {
        assert_eq!(win(&format!("a{c}b.txt")), "a_b.txt", "{c:?}");
    }
}

#[test]
fn control_characters_become_underscores_on_windows() {
    assert_eq!(win("a\u{1}b\u{1f}c.txt"), "a_b_c.txt");
    assert_eq!(win("tab\there"), "tab_here");
}

#[test]
fn reserved_device_names_are_renamed_case_insensitively() {
    for n in [
        "CON", "con", "Prn", "AUX", "NUL", "COM0", "COM1", "com9", "LPT0", "LPT1", "lpt9", "COM¹", "COM²", "COM³",
        "LPT¹", "LPT²", "LPT³",
    ] {
        let out = win(n);
        assert_ne!(out.to_lowercase(), n.to_lowercase(), "{n} must change");
    }
    assert_eq!(win("CON.txt"), "CON_.txt");
    assert_eq!(win("nul.tar.gz"), "nul_.tar.gz", "checked before the first dot");
    assert_eq!(win("AUX .txt"), "AUX _.txt", "a space before the dot doesn't hide the device name");
}

#[test]
fn names_that_only_look_reserved_are_left_alone() {
    for n in ["CONSOLE.txt", "COM10", "LPT", "nullable.rs", "aux-notes.md", "COM.txt"] {
        assert_eq!(win(n), n);
    }
}

#[test]
fn trailing_dots_and_spaces_are_removed_on_windows() {
    assert_eq!(win("report."), "report");
    assert_eq!(win("report "), "report");
    assert_eq!(win("report. . "), "report");
    assert_eq!(win("..."), "_", "nothing left becomes _");
    assert_eq!(win(" leading is fine"), " leading is fine");
}

#[test]
fn names_are_normalised_to_nfc() {
    let nfd = "Re\u{301}sume\u{301}.pdf";
    assert_eq!(win(nfd), "Résumé.pdf");
    assert_eq!(mac(nfd), "Résumé.pdf");
    assert!(sanitize(nfd, Target::Windows).reasons.is_empty(), "NFC alone isn't reported as a rename");
}

#[test]
fn the_brief_example_arrives_with_a_safe_name_and_reasons() {
    let s = sanitize("Re\u{301}sume\u{301}: Q3?.pdf", Target::Windows);
    assert_eq!(s.name, "Résumé_ Q3_.pdf");
    assert_eq!(s.reasons.len(), 1);
    assert!(s.reasons[0].contains(':') && s.reasons[0].contains('?'), "{:?}", s.reasons);
}

// ---- Both directions --------------------------------------------------------

#[test]
fn long_names_fit_ntfs_in_utf16_units_and_keep_the_extension() {
    let long = format!("{}.txt", "a".repeat(300));
    let out = win(&long);
    assert_eq!(utf16(&out), 255);
    assert!(out.ends_with(".txt"));
    let emoji = format!("{}.png", "😀".repeat(200));
    let out = win(&emoji);
    assert!(utf16(&out) <= 255);
    assert!(out.ends_with(".png"));
    assert!(out.chars().all(|c| c == '😀' || ".png".contains(c)), "never splits a character");
}

#[test]
fn long_names_fit_apfs_in_utf8_bytes() {
    // 100 CJK characters = 300 UTF-8 bytes: fine on NTFS, too long for APFS.
    let cjk = format!("{}.docx", "文".repeat(100));
    assert_eq!(win(&cjk), cjk, "fits NTFS");
    let out = mac(&cjk);
    assert!(out.len() <= 255, "{} bytes", out.len());
    assert!(out.ends_with(".docx"));
}

#[test]
fn numbering_still_fits_after_truncation() {
    let long = format!("{}.txt", "b".repeat(300));
    let first = win(&long);
    let second = numbered_fitting(&first, 2, false, Target::Windows);
    assert!(utf16(&second) <= 255);
    assert!(second.ends_with(" (2).txt"), "{second}");
}

#[test]
fn names_that_collide_after_sanitising_get_numbers() {
    let mut n = Namer::new(Target::Windows);
    assert_eq!(n.map_file(&["a:b.txt".into()]), vec!["a_b.txt"]);
    assert_eq!(n.map_file(&["a?b.txt".into()]), vec!["a_b (2).txt"]);
    assert_eq!(n.map_file(&["A_B.TXT".into()]), vec!["A_B (3).TXT"], "case-insensitive");
    let mut n = Namer::new(Target::Mac);
    n.map_file(&["Re\u{301}sume\u{301}.pdf".into()]);
    assert_eq!(n.map_file(&["Résumé.pdf".into()]), vec!["Résumé (2).pdf"], "NFD and NFC are the same name");
}

#[test]
fn renames_are_recorded_with_a_reason() {
    let mut n = Namer::new(Target::Windows);
    n.map_file(&["Folder:1".into(), "CON.txt".into()]);
    let reasons: Vec<_> = n.renames.iter().map(|r| (r.from.join("/"), r.to.join("/"), r.reason.clone())).collect();
    assert_eq!(reasons.len(), 2, "{reasons:?}");
    assert_eq!(reasons[0].1, "Folder_1");
    assert!(reasons[1].2.contains("reserved"), "{reasons:?}");
}

#[test]
fn mac_targets_keep_windows_only_rules_off() {
    assert_eq!(mac("CON.txt"), "CON.txt");
    assert_eq!(mac("report."), "report.");
    assert_eq!(mac("a?b"), "a?b");
}
