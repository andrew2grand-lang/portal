# Third-party material

What SeamDrop uses that someone else made, and under which terms. Everything
here is free to use; nothing was paid for.

## Research

**Cross-OS portability issues in Python projects (MSR 2026).** The scanner's
first seven finding categories (Files & paths, Processes & signals, Library
dependencies, API availability, Environment & display, Permissions & limits,
System information) follow the categories of this study of 2,042 Python
projects. Data: https://github.com/ncsu-swat/portability-issues-py (BSD-2-Clause).

Only the category names and the idea of the grouping are used. No code,
issue text or examples from the dataset are copied: every fixture under
`crates/portal-scan/tests/fixtures/` was written for SeamDrop. The other
categories (Copied environments, Scripts & line endings, Local AI hardware,
Docker, Interpreter on Windows, Native apps, Secrets, Services) are our own.

## Scanner (`crates/portal-scan`)

| Crate | Version | Licence | Used for |
|---|---|---|---|
| tree-sitter | 0.27.1 | MIT | Parsing source code |
| tree-sitter-python | 0.25.0 | MIT | Python grammar |
| tree-sitter-javascript | 0.25.0 | MIT | JavaScript grammar |
| tree-sitter-typescript | 0.23.2 | MIT | TypeScript / TSX grammars |
| tree-sitter-bash | 0.25.1 | MIT | Shell scripts and npm scripts |
| streaming-iterator | 0.1.9 | MIT OR Apache-2.0 | tree-sitter query results |
| ignore | 0.4.33 | Unlicense OR MIT | `.gitignore`-aware walking |
| rayon | 1.12.0 | MIT OR Apache-2.0 | Parsing files in parallel |
| toml | 0.9.8 | MIT OR Apache-2.0 | `pyproject.toml`, `Pipfile` |
| yaml-rust2 | 0.13.0 | MIT OR Apache-2.0 | `environment.yml`, Compose files |
| schemars | 1.2.3 | MIT | The report's JSON schema |
| ureq | 3.4.2 | MIT OR Apache-2.0 | `--online` PyPI lookups only |
| rustls / ring | 0.23 / 0.17 | Apache-2.0 / ISC | TLS for `--online` |
| webpki-roots | 1.0.11 | CDLA-Permissive-2.0 | Root certificates for `--online` |
| dirs | 6.0.0 | MIT OR Apache-2.0 | The PyPI cache folder |

## App

| Package | Version | Licence | Used for |
|---|---|---|---|
| thinking-orbs | 0.3.2 | MIT | The orb beside the transfer percentage |

The app's other dependencies (Tauri, React, Motion, rustls, mdns-sd and the
rest) are pinned in `Cargo.lock` / `package-lock.json`; `cargo metadata` and
`npm ls --json` list their licences.
