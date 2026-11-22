//! The answer first: tier, headline, expected AI usage and the hand-off note.

use crate::report::{AiLevel, AiUsage, Finding, Report, State, Support, Tier, TierInfo};
use crate::rules::project::human_bytes;
use crate::snapshot::Snapshot;
use crate::walk::Inventory;

fn has_dep(s: &Snapshot, names: &[&str]) -> Option<&'static str> {
    let deps: Vec<String> =
        s.python_deps.iter().chain(s.node_deps.iter()).map(|(n, _, _)| n.to_ascii_lowercase()).collect();
    names.iter().find(|n| deps.iter().any(|d| d == *n)).map(|n| match *n {
        "fastapi" => "a FastAPI server",
        "flask" => "a Flask app",
        "django" => "a Django site",
        "streamlit" => "a Streamlit app",
        "gradio" => "a Gradio app",
        "starlette" | "uvicorn" | "aiohttp" | "tornado" => "a Python web server",
        "express" => "an Express server",
        "fastify" | "koa" | "@nestjs/core" | "hono" => "a Node server",
        "next" => "a Next.js site",
        "nuxt" => "a Nuxt site",
        "@remix-run/react" => "a Remix site",
        "react" => "a React app",
        "vue" => "a Vue app",
        "svelte" => "a Svelte app",
        "electron" => "an Electron app",
        "ollama" => "a local model via Ollama",
        "torch" => "PyTorch models",
        "transformers" => "Hugging Face models",
        "mlx" | "mlx-lm" => "MLX models",
        "llama-cpp-python" => "llama.cpp models",
        "langchain" | "llama-index" => "an AI pipeline",
        "celery" | "rq" | "dramatiq" | "bullmq" | "bull" => "background workers",
        _ => "an app",
    })
}

const WEB: &[&str] = &[
    "fastapi",
    "flask",
    "django",
    "streamlit",
    "gradio",
    "starlette",
    "uvicorn",
    "aiohttp",
    "tornado",
    "express",
    "fastify",
    "koa",
    "@nestjs/core",
    "hono",
    "next",
    "nuxt",
    "@remix-run/react",
    "react",
    "vue",
    "svelte",
    "electron",
];
const AI: &[&str] =
    &["ollama", "mlx", "mlx-lm", "llama-cpp-python", "torch", "transformers", "langchain", "llama-index"];
const WORKERS: &[&str] = &["celery", "rq", "dramatiq", "bullmq", "bull"];

fn join(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

pub fn tier(s: &Snapshot, inv: &Inventory, findings: &[Finding]) -> TierInfo {
    let native = s
        .native
        .first()
        .map(|(kind, _)| *kind)
        .or_else(|| findings.iter().any(|f| f.rule == "native-apple").then_some("xcode"));
    if let Some(kind) = native {
        let what = match kind {
            "xcode" => "an Xcode app for Apple systems",
            "wpf" => "a WPF desktop app for Windows",
            "winforms" => "a Windows Forms app",
            "vcxproj" => "a Visual C++ project for Windows",
            _ => "a Windows desktop app",
        };
        return TierInfo { tier: Tier::Native, reason: format!("Native: {what}, built for one OS") };
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(web) = has_dep(s, WEB) {
        parts.push(web.into());
    }
    if let Some(ai) = has_dep(s, AI) {
        parts.push(ai.into());
    }
    let compose_services: Vec<&str> = s
        .services
        .iter()
        .filter(|x| x.from.ends_with(".yml") || x.from.ends_with(".yaml"))
        .map(|x| x.name.as_str())
        .collect();
    let workers = has_dep(s, WORKERS).is_some();
    let docker = !s.docker_images.is_empty();
    if compose_services.len() >= 2
        || (!compose_services.is_empty() && (docker || workers))
        || (workers && !s.services.is_empty())
    {
        let mut sys = parts.clone();
        let names: Vec<String> = compose_services.iter().map(|n| n.to_string()).collect();
        if !names.is_empty() {
            sys.push(join(&names));
        }
        if workers && !sys.iter().any(|p| p == "background workers") {
            sys.push("background workers".into());
        }
        let where_ = if docker { " in Docker" } else { "" };
        return TierInfo { tier: Tier::Systems, reason: format!("Systems: {}{where_}", join(&sys)) };
    }
    if !parts.is_empty() || docker {
        if parts.is_empty() {
            parts.push("an app that runs in Docker".into());
        }
        return TierInfo { tier: Tier::Apps, reason: format!("Apps: {}", join(&parts)) };
    }
    let count = |exts: &[&str]| {
        inv.files.iter().filter(|f| exts.iter().any(|e| f.rel.to_ascii_lowercase().ends_with(e))).count() as u64
    };
    let py = count(&[".py"]);
    let nb = count(&[".ipynb"]);
    let js = count(&[".js", ".mjs", ".cjs", ".ts", ".tsx", ".jsx"]);
    let sh = count(&[".sh", ".bash", ".ps1", ".bat", ".cmd"]);
    let mut code = Vec::new();
    if py > 0 {
        code.push(plural(py, "Python script", "Python scripts"));
    }
    if nb > 0 {
        code.push(plural(nb, "notebook", "notebooks"));
    }
    if js > 0
        && s.npm_scripts.is_empty()
        && !s.ecosystems.iter().any(|e| e.kind == "node" && !e.evidence.is_empty())
        && count(&[".html"]) > 0
    {
        return TierInfo {
            tier: Tier::Static,
            reason: "Static: a website of HTML, CSS and JavaScript; nothing to install".into(),
        };
    }
    if js > 0 {
        code.push(plural(js, "JavaScript/TypeScript file", "JavaScript/TypeScript files"));
    }
    if sh > 0 {
        let unix = count(&[".sh", ".bash"]);
        let (one, many) = match () {
            _ if unix == sh => ("shell script", "shell scripts"),
            _ if unix == 0 => ("Windows script", "Windows scripts"),
            _ => ("command script", "command scripts"),
        };
        code.push(plural(sh, one, many));
    }
    if !s.npm_scripts.is_empty() {
        code.push(plural(s.npm_scripts.len() as u64, "npm script", "npm scripts"));
    }
    if !code.is_empty() {
        return TierInfo { tier: Tier::Scripts, reason: format!("Scripts: {}", join(&code)) };
    }
    let html = count(&[".html", ".htm"]);
    let docs = count(&[".md", ".txt", ".pdf", ".doc", ".docx", ".rtf", ".pages", ".odt", ".key", ".pptx"]);
    let manifests = inv
        .files
        .iter()
        .filter(|f| {
            let name = f.rel.rsplit('/').next().unwrap_or(&f.rel);
            matches!(name, "package.json" | "package-lock.json" | "tsconfig.json" | "composer.json" | "deno.json")
        })
        .count() as u64;
    let data = count(&[".csv", ".tsv", ".json", ".xlsx", ".xls", ".numbers", ".parquet", ".sqlite", ".db"]) - manifests;
    let images = count(&[".png", ".jpg", ".jpeg", ".gif", ".heic", ".svg", ".webp", ".tif", ".tiff", ".psd"]);
    let mut kinds: Vec<String> = Vec::new();
    for (n, word) in [(docs, "documents"), (data, "data"), (images, "images")] {
        if n > 0 {
            kinds.push(word.into());
        }
    }
    if html > 0 {
        kinds.push(if kinds.is_empty() { "a website of HTML and CSS" } else { "a small website" }.into());
    }
    if inv.files.is_empty() {
        return TierInfo { tier: Tier::Static, reason: "Static: an empty folder".into() };
    }
    let what = if kinds.is_empty() { "files".to_owned() } else { join(&kinds) };
    TierInfo { tier: Tier::Static, reason: format!("Static: {what}; nothing to run") }
}

pub fn headline(r: &Report) -> String {
    let c = &r.counts;
    let target = r.target.name();
    if r.stats.files == 0 && r.findings.is_empty() {
        return "This folder is empty, so there was nothing to check.".into();
    }
    if r.tier.tier == Tier::Native {
        return format!(
            "This needs a rebuild for {target}: it's {}.",
            r.tier.reason.trim_start_matches("Native: ").split(',').next().unwrap_or("a native app")
        );
    }
    let things = |n: u32, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut parts = Vec::new();
    if c.blocker > 0 {
        parts.push(format!("{} on {target}", things(c.blocker, "thing blocks this", "things block this")));
    }
    if c.will_need_a_fix > 0 {
        parts.push(if c.blocker > 0 {
            things(c.will_need_a_fix, "needs a fix", "need a fix")
        } else {
            things(c.will_need_a_fix, "thing will need a fix", "things will need a fix")
        });
    }
    if c.needs_you > 0 {
        parts.push(things(c.needs_you, "needs you", "need you"));
    }
    let lead = if c.blocker == 0 { "No blockers found. " } else { "" };
    if parts.is_empty() {
        return if c.couldnt_check > 0 {
            format!(
                "No problems found for {target}, but {} couldn't be checked.",
                things(c.couldnt_check, "thing", "things")
            )
        } else {
            format!("No problems found for {target}.")
        };
    }
    let mut sentence = join(&parts);
    if c.blocker == 0 {
        sentence = sentence[..1].to_uppercase() + &sentence[1..];
    }
    format!("{lead}{sentence}.")
}

pub fn ai_usage_placeholder() -> AiUsage {
    AiUsage { level: AiLevel::None, estimate: true, reason: String::new() }
}

pub fn ai_usage(r: &Report) -> AiUsage {
    let issues: Vec<&Finding> =
        r.findings.iter().filter(|f| matches!(f.state, State::Blocker | State::WillNeedAFix)).collect();
    let manual = issues.iter().filter(|f| !f.auto_fixable).count();
    let distinct = r.counts.blocker + r.counts.will_need_a_fix;
    let (level, reason) = if r.tier.tier == Tier::Native {
        (AiLevel::PlanOnly, "A rebuild isn't a conversion: the AI would draft the migration plan, not change the code.")
    } else if issues.is_empty() {
        (AiLevel::None, "Nothing to change, so no AI should be needed.")
    } else if manual == 0 && r.counts.blocker == 0 {
        (AiLevel::Rare, "Every fix found follows a known pattern; the AI would step in only if a check failed.")
    } else if r.tier.tier == Tier::Systems || distinct >= 10 {
        (
            AiLevel::Often,
            "Several parts need changes that don't follow a fixed pattern, so expect the AI to be used often.",
        )
    } else {
        (AiLevel::Sometimes, "Some fixes need judgement (a library swap, a different API), so expect some AI use.")
    };
    AiUsage { level, estimate: true, reason: reason.into() }
}

/// For projects the app can't take further itself: a note for the user's own coding AI.
pub fn handoff(r: &Report) -> Option<String> {
    let summary_only: Vec<&str> = r
        .ecosystems
        .iter()
        .filter(|e| e.support == Support::Summary && (e.files > 0 || !e.evidence.is_empty()))
        .map(|e| e.name.as_str())
        .collect();
    if r.tier.tier != Tier::Native && summary_only.is_empty() {
        return None;
    }
    let target = r.target.name();
    let mut md = String::new();
    md.push_str(&format!("# Moving {} to {target}\n\n", r.project.name));
    md.push_str(concat!("Written by ", env!("SEAMDROP_APP_NAME"), "'s scanner, which read the project and ran nothing. Everything below is what it found, not a promise that anything works.\n\n"));
    md.push_str("## What this project is\n\n");
    md.push_str(&format!("- {}\n", r.tier.reason));
    let langs: Vec<String> = r
        .ecosystems
        .iter()
        .map(|e| if e.files > 0 { format!("{} ({} files)", e.name, e.files) } else { e.name.clone() })
        .collect();
    if !langs.is_empty() {
        md.push_str(&format!("- Languages and tools: {}\n", langs.join(", ")));
    }
    for rc in r.run_commands.iter().take(5) {
        md.push_str(&format!("- Run here with `{}` ({})\n", rc.command, rc.file));
    }
    let section = |md: &mut String, title: &str, state: State| {
        let items: Vec<&Finding> = r.findings.iter().filter(|f| f.state == state).collect();
        if items.is_empty() {
            return;
        }
        md.push_str(&format!("\n## {title}\n\n"));
        let mut seen = std::collections::HashSet::new();
        for f in items {
            if !seen.insert(&f.rule) {
                continue;
            }
            let at = match (&f.file, f.line) {
                (Some(file), Some(line)) => format!(" (`{file}:{line}`)"),
                (Some(file), None) => format!(" (`{file}`)"),
                _ => String::new(),
            };
            md.push_str(&format!("- **{}**{at}: {} Suggested fix: {}\n", f.title, f.why, f.fix_pattern));
        }
    };
    section(&mut md, &format!("What blocks it on {target}"), State::Blocker);
    section(&mut md, "What will need a fix", State::WillNeedAFix);
    section(&mut md, "What needs the person moving it", State::NeedsYou);
    section(&mut md, "What couldn't be checked", State::CouldntCheck);
    md.push_str("\n## Suggested steps\n\n");
    if r.tier.tier == Tier::Native {
        md.push_str(&format!(
            "1. Separate what doesn't depend on the OS (data models, networking, business logic) from the interface.\n2. Choose an interface toolkit that runs on {target} (for example .NET MAUI, Avalonia, Flutter or a web UI) and rebuild the screens one by one.\n3. Replace OS-only services (keychain, notifications, file pickers, menus) with {target} equivalents.\n4. Port the tests first, so each step can be checked.\n"
        ));
    } else {
        md.push_str(&format!(
            "1. Install the language runtimes and build tools these languages need on {target}.\n2. Work through the items above, starting with anything that blocks it.\n3. Build and run the tests on {target}; fix what fails before running the app.\n"
        ));
    }
    if r.left_out.bytes > 0 {
        md.push_str(&format!(
            "\n{} of rebuildable or ignored files weren't included.\n",
            human_bytes(r.left_out.bytes)
        ));
    }
    if !r.secrets.is_empty() {
        let files: Vec<&str> = r.secrets.iter().map(|s| s.path.as_str()).collect();
        md.push_str(&format!("Secrets weren't included ({}); set them up again by hand.\n", files.join(", ")));
    }
    Some(md)
}
