//! Source files parsed with tree-sitter: one parser per language per thread.
//! Code rules walk the syntax tree; they never match code with regexes.

use std::cell::RefCell;
use std::collections::HashMap;

use tree_sitter::{Language, Node, Parser, Tree};

/// Larger files are skipped (generated code, minified bundles).
pub const MAX_PARSE_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Bash,
}

impl Lang {
    fn language(self) -> Language {
        match self {
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Bash => tree_sitter_bash::LANGUAGE.into(),
        }
    }

    pub fn is_js(self) -> bool {
        matches!(self, Lang::JavaScript | Lang::TypeScript | Lang::Tsx)
    }
}

/// By extension, or by shebang for extensionless scripts.
pub fn lang_for(rel: &str, head: &[u8]) -> Option<Lang> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("py" | "pyw" | "pyi") => Some(Lang::Python),
        Some("js" | "mjs" | "cjs" | "jsx") => Some(Lang::JavaScript),
        Some("ts" | "mts" | "cts") if !name.ends_with(".d.ts") => Some(Lang::TypeScript),
        Some("tsx") => Some(Lang::Tsx),
        Some("sh" | "bash" | "zsh" | "ksh" | "command") => Some(Lang::Bash),
        None => shebang_lang(head),
        _ => None,
    }
}

fn shebang_lang(head: &[u8]) -> Option<Lang> {
    let first = head.split(|b| *b == b'\n').next()?;
    let line = std::str::from_utf8(first).ok()?.trim_end_matches('\r');
    let rest = line.strip_prefix("#!")?;
    if rest.contains("python") {
        Some(Lang::Python)
    } else if rest.contains("node") {
        Some(Lang::JavaScript)
    } else if ["bash", "/sh", " sh", "zsh", "ksh"].iter().any(|s| rest.contains(s)) {
        Some(Lang::Bash)
    } else {
        None
    }
}

/// Looks like text: no NUL in the first 8 KB.
pub fn is_text(bytes: &[u8]) -> bool {
    !bytes[..bytes.len().min(8192)].contains(&0)
}

thread_local! {
    static PARSERS: RefCell<HashMap<Lang, Parser>> = RefCell::new(HashMap::new());
}

/// Parses source with this thread's parser for `lang`.
pub fn parse(lang: Lang, src: &[u8]) -> Option<Tree> {
    PARSERS.with(|cell| {
        let mut parsers = cell.borrow_mut();
        let parser = match parsers.get_mut(&lang) {
            Some(p) => p,
            None => {
                let mut p = Parser::new();
                p.set_language(&lang.language()).ok()?;
                parsers.entry(lang).or_insert(p)
            }
        };
        parser.parse(src, None)
    })
}

/// A node's text.
pub fn text<'a>(node: Node<'_>, src: &'a [u8]) -> &'a str {
    std::str::from_utf8(&src[node.byte_range()]).unwrap_or("")
}

/// 1-based line of a node.
pub fn line(node: Node<'_>) -> u32 {
    node.start_position().row as u32 + 1
}

/// The source line a node starts on, trimmed and shortened, for evidence.
pub fn evidence(node: Node<'_>, src: &[u8]) -> String {
    let row = node.start_position().row;
    let line = src.split(|b| *b == b'\n').nth(row).unwrap_or_default();
    let line = String::from_utf8_lossy(line);
    shorten(line.trim(), 120)
}

pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let cut: String = s.chars().take(max - 1).collect();
    format!("{cut}…")
}

/// Every node in the tree, depth first.
pub fn walk_nodes<'t>(root: Node<'t>, mut f: impl FnMut(Node<'t>)) {
    let mut cursor = root.walk();
    loop {
        f(cursor.node());
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

/// Calls that take a web address: fetch, requests.get, httpx.post, new URL…
const URL_CALLS: &[&str] = &[
    "fetch",
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "head",
    "options",
    "request",
    "urlopen",
    "urljoin",
    "Request",
    "URL",
    "WebSocket",
    "EventSource",
    "axios",
    "got",
    "ky",
];

/// Value names that hold a web address rather than a folder.
const URL_WORDS: &[&str] = &["url", "uri", "host", "origin", "endpoint", "domain"];

/// A string (or `a + "/b"`) that builds a web address, not a file path: an
/// argument to an HTTP call, or glued to a value named like a URL (API_URL,
/// host, endpoint…). Both use "/" on every OS.
pub fn builds_url(node: Node<'_>, src: &[u8]) -> bool {
    let mut expr = node;
    while let Some(p) = expr.parent() {
        if !matches!(p.kind(), "binary_operator" | "binary_expression" | "parenthesized_expression") {
            break;
        }
        expr = p;
    }
    let mut url_named = false;
    walk_nodes(expr, |n| {
        if matches!(n.kind(), "identifier" | "property_identifier" | "shorthand_property_identifier") {
            let name = text(n, src).to_ascii_lowercase();
            url_named |= URL_WORDS.iter().any(|w| name.contains(w));
        }
    });
    if url_named {
        return true;
    }
    let Some(call) =
        expr.parent().filter(|p| matches!(p.kind(), "argument_list" | "arguments")).and_then(|a| a.parent())
    else {
        return false;
    };
    let callee = call.child_by_field_name("function").or_else(|| call.child_by_field_name("constructor"));
    callee.is_some_and(|c| {
        let name = text(c, src);
        URL_CALLS.contains(&name.rsplit('.').next().unwrap_or(name))
    })
}

/// The value of a Python or JS string literal node (quotes and prefixes
/// removed; escapes and interpolations left as written).
pub fn string_value(node: Node<'_>, src: &[u8]) -> Option<String> {
    match node.kind() {
        // Python: string → string_start, string_content*, string_end;
        // JS: string → string_fragment / escape_sequence.
        "string" => {
            let raw = node
                .child(0)
                .is_some_and(|s| s.kind() == "string_start" && text(s, src).to_ascii_lowercase().contains('r'));
            let mut out = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "string_content" | "string_fragment" | "escape_sequence" => out.push_str(text(child, src)),
                    "interpolation" | "template_substitution" => out.push_str("{…}"),
                    _ => {}
                }
            }
            Some(if raw { out } else { unescape(&out) })
        }
        "template_string" => {
            let raw = text(node, src);
            Some(unescape(raw.trim_matches('`')))
        }
        _ => None,
    }
}

/// The common escapes decoded (`\\` → `\`, `\n`, `\t`, `\r`, quotes), so a
/// real backslash and an escape can be told apart. Others stay as written.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(q @ ('"' | '\'' | '`')) => out.push(q),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_grammar_loads_into_the_parser() {
        for lang in [Lang::Python, Lang::JavaScript, Lang::TypeScript, Lang::Tsx, Lang::Bash] {
            assert!(parse(lang, b"x").is_some(), "{lang:?}");
        }
    }

    #[test]
    fn languages_come_from_extensions_or_shebangs() {
        assert_eq!(lang_for("src/app.py", b""), Some(Lang::Python));
        assert_eq!(lang_for("web/main.tsx", b""), Some(Lang::Tsx));
        assert_eq!(lang_for("types/index.d.ts", b""), None, "declarations only");
        assert_eq!(lang_for("bin/tool", b"#!/usr/bin/env python3\nprint(1)"), Some(Lang::Python));
        assert_eq!(lang_for("bin/run", b"#!/bin/bash\r\necho hi"), Some(Lang::Bash));
        assert_eq!(lang_for("README", b"hello"), None);
    }

    #[test]
    fn python_string_values_drop_quotes_and_prefixes() {
        let src = br#"a = r"C:\data\x.csv"; b = f"{base}/out""#;
        let tree = parse(Lang::Python, src).unwrap();
        let mut values = Vec::new();
        walk_nodes(tree.root_node(), |n| {
            if n.kind() == "string" {
                values.push(string_value(n, src).unwrap());
            }
        });
        assert_eq!(values, [r"C:\data\x.csv", "{…}/out"]);
        let src = br#"u = "\\\\server\\share""#;
        let tree = parse(Lang::Python, src).unwrap();
        let mut got = None;
        walk_nodes(tree.root_node(), |n| {
            if n.kind() == "string" {
                got = string_value(n, src);
            }
        });
        assert_eq!(got.as_deref(), Some(r"\\server\share"), "escapes decoded in normal strings");
    }
}
