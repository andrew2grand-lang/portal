//! The readable summary the CLI prints without --json.

use std::fmt::Write;

use crate::report::{Finding, Report, State};
use crate::rules::project::human_bytes;

fn state_title(s: State) -> &'static str {
    match s {
        State::Blocker => "Blockers",
        State::WillNeedAFix => "Will need a fix",
        State::NeedsYou => "Needs you",
        State::CouldntCheck => "Couldn't check",
        State::Info => "Info",
    }
}

fn place(f: &Finding) -> String {
    match (&f.file, f.line) {
        (Some(file), Some(line)) => format!("{file}:{line}"),
        (Some(file), None) => file.clone(),
        _ => String::new(),
    }
}

pub fn render(r: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} scan · {} → {}", crate::APP_NAME, r.project.name, r.target.name());
    let _ = writeln!(out, "{}", r.headline);
    let _ = writeln!(out, "Tier: {}", r.tier.reason);
    let level = format!("{:?}", r.ai_usage.level).replace("PlanOnly", "Plan only");
    let _ = writeln!(out, "Expected AI use (estimate): {level}. {}", r.ai_usage.reason);

    for state in [State::Blocker, State::WillNeedAFix, State::NeedsYou, State::CouldntCheck, State::Info] {
        let items: Vec<&Finding> = r.findings.iter().filter(|f| f.state == state).collect();
        if items.is_empty() {
            continue;
        }
        let _ = writeln!(out, "\n{}", state_title(state));
        let mut rule_seen: Vec<&str> = Vec::new();
        for f in &items {
            let first_of_rule = !rule_seen.contains(&f.rule.as_str());
            if first_of_rule {
                rule_seen.push(&f.rule);
                let count = items.iter().filter(|x| x.rule == f.rule).count();
                let times = if count > 1 { format!(" ({count} places)") } else { String::new() };
                let _ = writeln!(out, "  • {}{times}", f.title);
                let _ = writeln!(out, "      {}", f.why);
                if matches!(state, State::Blocker | State::WillNeedAFix | State::NeedsYou) {
                    let _ = writeln!(out, "      Fix: {}", f.fix_pattern);
                }
            }
            let at = place(f);
            if !at.is_empty() || f.evidence.is_some() {
                let ev = f.evidence.as_deref().map(|e| format!("  {e}")).unwrap_or_default();
                let _ = writeln!(out, "      - {at}{ev}");
            }
        }
    }

    if !r.left_out.items.is_empty() {
        let parts: Vec<String> =
            r.left_out.items.iter().take(6).map(|i| format!("{} {}", i.path, human_bytes(i.bytes))).collect();
        let _ = writeln!(out, "\nWon't be sent: {} ({})", human_bytes(r.left_out.bytes), parts.join(", "));
    }
    if !r.run_commands.is_empty() {
        let cmds: Vec<String> = r.run_commands.iter().take(6).map(|c| format!("`{}`", c.command)).collect();
        let _ = writeln!(out, "How it's run here: {}", cmds.join(", "));
    }
    let _ = writeln!(
        out,
        "\nScanned {} files ({} parsed) in {:.1} s.",
        r.stats.files,
        r.stats.parsed_files,
        r.duration_ms as f64 / 1000.0
    );
    for note in &r.notes {
        let _ = writeln!(out, "{note}");
    }
    if !r.network.online && r.findings.iter().any(|f| f.rule == "wheels-unchecked") {
        let _ = writeln!(out, "Add --online to check package builds on PyPI (nothing else is sent).");
    }
    out
}
