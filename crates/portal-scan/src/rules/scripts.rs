//! Scripts: npm scripts (parsed with the Bash grammar), run scripts without
//! a twin for the other OS, python3 / make on Windows, line endings, exec bits.

use std::collections::HashSet;

use super::{Ctx, Hit};
use crate::parse::{self, Lang, text, walk_nodes};
use crate::report::TargetOs;

/// Commands cmd.exe doesn't have → the cross-platform package that replaces them.
const NPM_UNIX_COMMANDS: &[(&str, &str)] = &[
    ("rm", "rimraf (or `node -e \"fs.rmSync(…)\"`)"),
    ("cp", "cpy-cli or shx cp"),
    ("mv", "shx mv"),
    ("cat", "shx cat"),
    ("ls", "shx ls"),
    ("grep", "a small Node script"),
    ("sed", "a small Node script"),
    ("awk", "a small Node script"),
    ("chmod", "shx chmod (or drop it: Windows has no exec bit)"),
    ("touch", "shx touch"),
    ("ln", "a small Node script"),
    ("find", "a small Node script (or a glob package)"),
    ("which", "the which package"),
    ("sleep", "a small Node script"),
    ("xargs", "a small Node script"),
    ("head", "a small Node script"),
    ("tail", "a small Node script"),
    ("open", "the open package"),
];

/// Finds where `"name":` sits under "scripts" in a package.json, for the line number.
fn script_line(text: &str, name: &str) -> u32 {
    let start = text.find("\"scripts\"").unwrap_or(0);
    let needle = format!("\"{name}\"");
    text[start..].find(&needle).map(|i| text[..start + i].matches('\n').count() as u32 + 1).unwrap_or(1)
}

/// What cmd.exe can't do in one npm script: (problem, alternative).
pub fn npm_script_problems(command: &str) -> Vec<(String, String)> {
    let src = command.as_bytes();
    let Some(tree) = parse::parse(Lang::Bash, src) else { return vec![] };
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |p: String, a: String| {
        if !out.iter().any(|(x, _)| *x == p) {
            out.push((p, a));
        }
    };
    walk_nodes(tree.root_node(), |n| match n.kind() {
        "command" => {
            let Some(name) = n.child_by_field_name("name") else { return };
            let word = text(name, src);
            if let Some((cmd, alt)) = NPM_UNIX_COMMANDS.iter().find(|(c, _)| *c == word) {
                push(format!("has no `{cmd}` command"), format!("Use {alt} instead."));
            }
            if word == "mkdir" && text(n, src).split_whitespace().any(|w| w == "-p") {
                push(
                    "treats `mkdir -p` as a folder called -p".into(),
                    "Use mkdirp or `node -e \"fs.mkdirSync(dir, {recursive: true})\"`.".into(),
                );
            }
            let mut cursor = n.walk();
            if n.children(&mut cursor).any(|c| c.kind() == "variable_assignment") {
                push(
                    "doesn't understand `VAR=value` in front of a command".into(),
                    "Use cross-env: `cross-env NODE_ENV=production node server.js`.".into(),
                );
            }
        }
        "declaration_command" if text(n, src).starts_with("export") => {
            push(
                "has no `export`".into(),
                "Use cross-env to set variables for one command, or a .env file loaded by the app.".into(),
            );
        }
        "raw_string" => {
            push(
                "doesn't treat single quotes as quotes".into(),
                "Use double quotes (escaped as \\\" inside package.json).".into(),
            );
        }
        "simple_expansion" | "expansion" => {
            push(
                "writes variables as %VAR%, not $VAR".into(),
                "Use cross-env (which accepts $VAR on both), or read the variable inside the program.".into(),
            );
        }
        _ => {}
    });
    out
}

pub fn npm_scripts(ctx: &Ctx<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (file, name, command) in &ctx.snap.npm_scripts {
        let json = std::fs::read_to_string(ctx.root.join(file)).unwrap_or_default();
        let at = script_line(&json, name);
        for (problem, alternative) in npm_script_problems(command) {
            hits.push(
                Hit::new("npm-script-unix")
                    .at(file, at)
                    .evidence(format!("\"{name}\": \"{command}\""))
                    .var("problem", problem)
                    .var("alternative", alternative),
            );
        }
    }
    hits
}

fn stem_of(path: &str) -> &str {
    path.rsplit_once('.').map(|(s, _)| s).unwrap_or(path)
}

/// Run commands that call a script file, and whether the other OS has a version.
pub fn twins(ctx: &Ctx<'_>) -> Vec<Hit> {
    let files: HashSet<&str> = ctx.inv.files.iter().map(|f| f.rel.as_str()).collect();
    let mut seen = HashSet::new();
    let mut hits = Vec::new();
    for rc in &ctx.snap.run_commands {
        let base_dir = rc.file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for token in rc.command.split(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | '|' | '"' | '\'')) {
            let token = token.trim_start_matches(".\\").trim_start_matches("./").replace('\\', "/");
            let ext = token.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
            if !matches!(ext.as_str(), "sh" | "bash" | "bat" | "cmd" | "ps1") {
                continue;
            }
            let candidates =
                [token.clone(), if base_dir.is_empty() { token.clone() } else { format!("{base_dir}/{token}") }];
            let Some(script) = candidates.iter().find(|c| files.contains(c.as_str())) else { continue };
            if !seen.insert(script.clone()) {
                continue;
            }
            let stem = stem_of(script);
            let has = |exts: &[&str]| exts.iter().any(|e| files.contains(format!("{stem}.{e}").as_str()));
            let unix = matches!(ext.as_str(), "sh" | "bash");
            if unix && !has(&["ps1", "bat", "cmd"]) {
                hits.push(
                    Hit::new("script-without-windows-twin")
                        .file(script)
                        .evidence(format!("{} (in {})", rc.command, rc.file))
                        .var("script", script.clone())
                        .var("twin", format!("{}.ps1", stem.rsplit('/').next().unwrap_or(stem))),
                );
            }
            if !unix && !has(&["sh", "bash"]) {
                hits.push(
                    Hit::new("script-without-mac-twin")
                        .file(script)
                        .evidence(format!("{} (in {})", rc.command, rc.file))
                        .var("script", script.clone())
                        .var("twin", format!("{}.sh", stem.rsplit('/').next().unwrap_or(stem))),
                );
            }
        }
    }
    hits
}

/// `python3` in commands people run (npm scripts, README, Procfile, VS Code).
pub fn python3_commands(ctx: &Ctx<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();
    for rc in &ctx.snap.run_commands {
        if !matches!(rc.kind.as_str(), "npm-script" | "readme" | "procfile") {
            continue;
        }
        let uses = rc.command.split(|c: char| c.is_whitespace() || matches!(c, '&' | ';' | '|' | '(')).any(|w| {
            w == "python3" || w.strip_prefix("python3.").is_some_and(|v| v.chars().all(|c| c.is_ascii_digit()))
        });
        if uses && seen.insert(rc.command.clone()) {
            hits.push(Hit::new("python3-command").file(&rc.file).evidence(format!("{} (in {})", rc.command, rc.file)));
        }
    }
    hits
}

/// A Makefile that's a way to build or run the project.
pub fn make(ctx: &Ctx<'_>) -> Vec<Hit> {
    let targets: Vec<&crate::report::RunCommand> = ctx.snap.run_commands.iter().filter(|r| r.kind == "make").collect();
    let runnable = targets.iter().find(|t| {
        matches!(t.name.as_str(), "run" | "start" | "dev" | "serve" | "build" | "install" | "setup" | "test" | "all")
    });
    match runnable {
        Some(t) => vec![
            Hit::new("make-on-windows").file(&t.file).evidence(t.command.clone()).var("target_name", t.name.clone()),
        ],
        None => vec![],
    }
}

/// CRLF in shell scripts and in any file that starts with #!.
pub fn line_endings(ctx: &Ctx<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for f in &ctx.inv.files {
        if f.size > parse::MAX_PARSE_BYTES {
            continue;
        }
        let ext = f.rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        let script_ext = matches!(ext.as_str(), "sh" | "bash" | "zsh" | "command" | "ksh");
        if !script_ext && !ext.is_empty() && !matches!(ext.as_str(), "py" | "pl" | "rb") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&f.abs) else { continue };
        if !parse::is_text(&bytes) {
            continue;
        }
        let shebang = bytes.starts_with(b"#!");
        if (script_ext || shebang) && bytes.windows(2).any(|w| w == b"\r\n") {
            let first =
                String::from_utf8_lossy(bytes.split(|b| *b == b'\n').next().unwrap_or_default()).trim().to_owned();
            hits.push(Hit::new("crlf-script").at(&f.rel, 1).evidence(format!("{first}  (lines end in \\r\\n)")));
        }
    }
    hits
}

/// Scripts marked executable (only visible when scanning on a Mac or Linux).
pub fn exec_bits(ctx: &Ctx<'_>) -> Vec<Hit> {
    if ctx.target != TargetOs::Windows {
        return vec![];
    }
    let scripts: Vec<&str> = ctx
        .inv
        .files
        .iter()
        .filter(|f| f.executable)
        .filter(|f| {
            let ext = f.rel.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            matches!(ext, "sh" | "py" | "bash" | "command" | "pl" | "rb") || ext.is_empty()
        })
        .map(|f| f.rel.as_str())
        .collect();
    if scripts.is_empty() {
        return vec![];
    }
    let shown: Vec<&str> = scripts.iter().take(5).copied().collect();
    let more = if scripts.len() > 5 { format!(" and {} more", scripts.len() - 5) } else { String::new() };
    vec![
        Hit::new("executable-bit")
            .file(scripts[0])
            .evidence(format!("{}{more}", shown.join(", ")))
            .var("file", shown.join(" ")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problems(cmd: &str) -> Vec<String> {
        npm_script_problems(cmd).into_iter().map(|(p, _)| p).collect()
    }

    #[test]
    fn npm_scripts_are_read_as_shell_syntax() {
        assert_eq!(problems("rm -rf dist && vite build"), ["has no `rm` command"]);
        assert_eq!(
            problems("NODE_ENV=production node server.js"),
            ["doesn't understand `VAR=value` in front of a command"]
        );
        assert_eq!(problems("export PORT=3000 && node app.js"), ["has no `export`"]);
        assert_eq!(problems("eslint 'src/**/*.ts'"), ["doesn't treat single quotes as quotes"]);
        assert_eq!(problems("node build.js --out $OUT_DIR"), ["writes variables as %VAR%, not $VAR"]);
        assert_eq!(problems("mkdir -p dist/assets"), ["treats `mkdir -p` as a folder called -p"]);
    }

    #[test]
    fn cross_platform_scripts_raise_nothing() {
        for ok in [
            "vite build",
            "rimraf dist && tsc -p .",
            "cross-env NODE_ENV=production node server.js",
            "node \"scripts/x.js\"",
            "shx rm -rf dist",
            "npm run build && npm test",
        ] {
            assert!(problems(ok).is_empty(), "{ok}: {:?}", problems(ok));
        }
    }
}
