//! File-level rules: links, names Windows can't use, case clashes, long paths.

use std::collections::HashMap;

use portal_core::fsafe::{self, Target};

use super::{Ctx, Hit};
use crate::report::TargetOs;

/// Roughly `C:\Users\<name>\Downloads\SeamDrop\` in front of every received path.
const RECEIVE_PREFIX: usize = 40;
const WINDOWS_MAX_PATH: usize = 260;

pub fn check(ctx: &Ctx<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();

    for link in &ctx.inv.symlinks {
        let outside =
            if link.outside { " It also points outside the project, so what it points to isn't sent." } else { "" };
        hits.push(
            Hit::new("symlink")
                .file(&link.rel)
                .evidence(format!("{} → {}", link.rel, link.target))
                .var("outside", outside),
        );
    }

    if ctx.target == TargetOs::Windows {
        for f in &ctx.inv.files {
            for part in f.rel.split('/') {
                let s = fsafe::sanitize(part, Target::Windows);
                let nfc = fsafe::sanitize(part, Target::Mac).name;
                if s.name != nfc && !s.reasons.is_empty() {
                    hits.push(
                        Hit::new("invalid-name")
                            .file(&f.rel)
                            .evidence(format!("{part} → {}", s.name))
                            .var("reason", reason_sentence(&s.reasons)),
                    );
                    break;
                }
            }
            if f.rel.chars().count() + RECEIVE_PREFIX > WINDOWS_MAX_PATH {
                hits.push(Hit::new("long-path").file(&f.rel).evidence(format!("{} characters", f.rel.chars().count())));
            }
        }
    }

    // Same folder, names equal apart from case (or Unicode form).
    let mut by_key: HashMap<String, &str> = HashMap::new();
    for f in &ctx.inv.files {
        // How a case-insensitive disk sees the name (NFC, lowercase).
        let key = fsafe::fold(&f.rel);
        if let Some(first) = by_key.get(&key) {
            if *first != f.rel {
                hits.push(
                    Hit::new("case-clash")
                        .file(&f.rel)
                        .evidence(format!("{first} and {}", f.rel))
                        .var("other", (*first).to_owned()),
                );
            }
        } else {
            by_key.insert(key, &f.rel);
        }
    }
    hits
}

fn reason_sentence(reasons: &[String]) -> String {
    let mut s = reasons.join("; ");
    if let Some(first) = s.get(..1) {
        s = first.to_uppercase() + &s[1..];
    }
    if !s.ends_with('.') {
        s.push('.');
    }
    s
}
