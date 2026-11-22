//! JavaScript / TypeScript rules, on the tree-sitter syntax tree.

use tree_sitter::Node;

use super::Hit;
use super::python::{ENV_VARS, SHELL_COMMANDS, absolute_path_targets, path_piece, system_path};
use crate::parse::{self, evidence, line, string_value, text, walk_nodes};

/// Node advice for the same problems (the shared tables speak Python).
fn node_env_advice(name: &str) -> &'static str {
    match name {
        "HOME" | "USERPROFILE" | "HOMEDRIVE" | "HOMEPATH" => "Use os.homedir(), which works on both.",
        "USER" | "USERNAME" | "LOGNAME" => "Use os.userInfo().username, which works on both.",
        "TMPDIR" | "TEMP" => "Use os.tmpdir(), which works on both.",
        "APPDATA" | "LOCALAPPDATA" | "XDG_CONFIG_HOME" | "XDG_DATA_HOME" | "XDG_CACHE_HOME" => {
            "Use a package such as env-paths for per-OS app folders."
        }
        _ => "Read it behind a process.platform check, or use a Node API that works on both.",
    }
}

fn is_platform_guarded(node: Node<'_>, src: &[u8]) -> bool {
    let mut n = node.parent();
    while let Some(p) = n {
        if matches!(p.kind(), "if_statement" | "ternary_expression" | "switch_statement") {
            let t = p.child_by_field_name("condition").map(|c| text(c, src)).unwrap_or_else(|| text(p, src));
            if t.contains("process.platform")
                || t.contains("os.platform()")
                || t.contains("isWindows")
                || t.contains("IS_WINDOWS")
            {
                return true;
            }
        }
        n = p.parent();
    }
    false
}

pub fn check(rel: &str, src: &[u8], tree: &tree_sitter::Tree) -> Vec<Hit> {
    let mut hits = Vec::new();
    walk_nodes(tree.root_node(), |n| match n.kind() {
        "string" | "template_string" => {
            // Imports and requires name modules, not files on disk.
            if n.parent().is_some_and(|p| matches!(p.kind(), "import_statement" | "export_statement")) {
                return;
            }
            let Some(value) = string_value(n, src) else { return };
            if let Some(targets) = absolute_path_targets(&value) {
                let mut hit = Hit::new("absolute-path")
                    .at(rel, line(n))
                    .evidence(evidence(n, src))
                    .var("path", parse::shorten(&value, 60));
                hit.targets = Some(targets);
                hits.push(hit);
                return;
            }
            if let Some((where_, targets)) = system_path(&value) {
                let mut hit = Hit::new("system-path")
                    .at(rel, line(n))
                    .evidence(evidence(n, src))
                    .var("path", parse::shorten(&value, 60))
                    .var("where", where_);
                hit.targets = Some(targets);
                hits.push(hit);
                return;
            }
            if n.kind() == "template_string"
                && !value.contains("://")
                && !value.contains(char::is_whitespace)
                && !parse::builds_url(n, src)
            {
                if value.contains("}\\\\") || value.contains("}\\") {
                    hits.push(Hit::new("path-concat-backslash").at(rel, line(n)).evidence(evidence(n, src)));
                } else if value.contains("}/") && !value.contains('?') {
                    hits.push(Hit::new("path-concat-slash").at(rel, line(n)).evidence(evidence(n, src)));
                }
            }
        }
        "binary_expression" => {
            let op = n.child_by_field_name("operator").map(|o| text(o, src)).unwrap_or("");
            let (Some(left), Some(right)) = (n.child_by_field_name("left"), n.child_by_field_name("right")) else {
                return;
            };
            if op != "+" || parse::builds_url(n, src) {
                return;
            }
            for (lit, other) in [(left, right), (right, left)] {
                if lit.kind() == "string"
                    && other.kind() != "string"
                    && let Some(sep) = string_value(lit, src).and_then(|v| path_piece(&v))
                {
                    let rule = if sep == '\\' { "path-concat-backslash" } else { "path-concat-slash" };
                    hits.push(Hit::new(rule).at(rel, line(n)).evidence(evidence(n, src)));
                    break;
                }
            }
        }
        "member_expression" | "subscript_expression" => {
            let object = n.child_by_field_name("object").map(|o| text(o, src)).unwrap_or("");
            if object != "process.env" {
                return;
            }
            let name = match n.kind() {
                "member_expression" => n.child_by_field_name("property").map(|p| text(p, src).to_owned()),
                _ => n.child_by_field_name("index").and_then(|i| string_value(i, src)),
            };
            // `process.env.HOME || fallback` has a fallback.
            let fallback = n.parent().is_some_and(|p| {
                p.kind() == "binary_expression"
                    && p.child_by_field_name("operator").is_some_and(|o| matches!(text(o, src), "||" | "??"))
            });
            if let Some(name) = name
                && !fallback
                && !is_platform_guarded(n, src)
                && let Some((var, missing, _)) = ENV_VARS.iter().find(|(v, _, _)| *v == name)
            {
                hits.push(
                    Hit::new("env-var")
                        .at(rel, line(n))
                        .evidence(evidence(n, src))
                        .only(*missing)
                        .var("name", *var)
                        .var("alternative", node_env_advice(var)),
                );
            }
        }
        "call_expression" => {
            let f = n.child_by_field_name("function").map(|f| text(f, src)).unwrap_or("");
            let last = f.rsplit('.').next().unwrap_or(f);
            if !matches!(last, "exec" | "execSync" | "spawn" | "spawnSync" | "execFile" | "execFileSync")
                || is_platform_guarded(n, src)
            {
                return;
            }
            let Some(first) = n.child_by_field_name("arguments").and_then(|a| a.named_child(0)) else { return };
            let Some(cmd) = string_value(first, src) else { return };
            let word = cmd.split_whitespace().next().unwrap_or("").to_ascii_lowercase();
            if let Some((c, missing, _)) = SHELL_COMMANDS.iter().find(|(c, _, _)| *c == word) {
                hits.push(
                    Hit::new("shell-command")
                        .at(rel, line(n))
                        .evidence(evidence(n, src))
                        .only(*missing)
                        .var("command", *c)
                        .var("alternative", "Use Node's fs and path modules (fs.rm, fs.cp, fs.readdir) instead of shell commands, or pick the command with process.platform."),
                );
            }
        }
        _ => {}
    });
    let mut seen = std::collections::HashSet::new();
    hits.retain(|h| seen.insert((h.rule, h.line)));
    hits
}
