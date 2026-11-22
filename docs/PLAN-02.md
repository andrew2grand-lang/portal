# Mission 02 plan: Project Send design lab + real scanner

Spec: `MISSION-02-project-send-design-and-scanner.md` (repo root). Decisions: `docs/DECISIONS.md` (79 onwards). Status: `docs/PROGRESS.md`.

**Goal:** a real `portal-scan` crate + CLI that reads a project folder (never runs it) and writes an honest, versioned blocker report; a separate design lab on localhost with every Project Send screen, rendering that report; and two small fixes in the app.

**Architecture:**
- `crates/portal-scan`: library + `portal-scan` binary. No Tauri, no async. Walks with `ignore` (honours `.gitignore`), parses Python / JS / TS / Bash with tree-sitter, runs rules in parallel (rayon), writes a `Report` (serde + schemars). The JSON schema in `docs/scan-report.schema.json` is generated from the Rust types and a test fails if it's stale. Reuses `portal-core::fsafe` for Windows file-name rules.
- `design-lab/`: a second Vite entry in the same package (`npm run lab`, port 5190) that imports the app's tokens, components and window frames, so it can't drift. A dev-server plugin runs the built `portal-scan` CLI and streams its progress (Server-Sent Events) for real scans.
- App: `PortalWindow` gains an optional stage slot (invisible to the app) so the lab renders real Portal windows.

**Execution:** native, step by step, tests first where there's behaviour; one independent review at the end.

## Global constraints (from the spec, verbatim where it matters)
- "The scanner never runs the project's code and never reads secret values." Key names only from `.env`; key files by name only. Network only with `--online`, only PyPI's JSON API, and the report says so. No following symlinks out of the folder.
- Findings carry `id, category, state, severity, file, line, evidence, why, fixPattern, autoFixable, targets`. Scanner states: Blocker / Will need a fix / Needs you / Couldn't check / Info. Never "will work".
- Tiers: Static, Scripts, Apps, Systems, Native, each with a one-line reason. AI usage: None / Rare / Sometimes / Often / Plan only, labelled an estimate.
- "The real app's look stays as it is." Only the two fixes in §7. Project tab keeps "Coming soon".
- Spend nothing; crates and tools free; nothing released in the last two weeks.
- Lab: both OS window styles, light and dark, every §4 screen, 5 demo scenarios, real scan of a typed/picked folder. Motion subtle; reduced motion respected.

## Review focus (inputs the spec implies but no rule test covers)
1. Projects with huge `node_modules` / `.venv` (100k files): left-out sizing must stay fast and never parse inside them.
2. Files that aren't UTF-8, binary files with code extensions, very large source files: skip or report "unreadable", never crash.
3. Guarded platform code (`if sys.platform == "win32":`, `try: import fcntl except ImportError`) must not raise false blockers.
4. Symlink loops and links pointing outside the project.
5. A clean, well-written project of each kind must come back with no blockers and no "will need a fix" (the clean set).

## Steps (each ends green and committed)

### 1. Plan and workspace
- This file; skills at the top of PROGRESS. `crates/portal-scan` (lib + bin, pinned deps), `design-lab/` entry, `npm run lab`, tsconfig/eslint coverage.

### 2. Skeleton: walk, ignore rules, ecosystems, schema, golden tests
- `walk.rs`: project files (gitignore on, hidden on, `.git` and heavy folders skipped), all files (for secrets), left-out folders (`.venv`, `venv`, `node_modules`, `__pycache__`, build output, OS junk) with sizes, gitignored files, symlinks (never followed).
- `report.rs`: `Report { schemaVersion: 1, … }`; `docs/scan-report.schema.json` generated (test: up to date). Golden JSON tests with timings and absolute paths normalised. CLI with `--json`, text summary.

### 3. Snapshot
- Ecosystems (Python, Node, shell, Docker, other languages and native project files by name and build files); requested Python/Node versions; installed versions here (`python --version`, `node --version` only, with a timeout); installed packages read statically; lockfile or not; run commands (pyproject scripts, package.json scripts, Makefile, Procfile, `.vscode/launch.json`, `__main__` guards, README code blocks); "won't be sent" sizes.

### 4. Rules, each with a broken fixture (`tests/fixtures/broken/<rule>/` + `expected.json`)
- Python (tree-sitter): absolute paths, path concatenation, `open`/`read_text`/`write_text` without `encoding`, process pools without a `__main__` guard, Unix-only and Windows-only modules, Unix-only OS APIs and signals, environment variables (`HOME` / `APPDATA`…), shell commands in `subprocess`/`os.system`, CUDA → Mac, CUDA-only libraries, MLX → Windows.
- JS/TS: absolute paths and path concatenation in code; npm scripts parsed with the Bash grammar (`rm -rf`, `cp`, `VAR=x cmd`, `export`, single quotes, `$VAR`).
- Scripts: CRLF in shell scripts / shebang files, missing `.ps1`/`.bat` twin (and the reverse), executable bit, `python3` / `make` in run commands.
- Files: symlinks, Windows-invalid names, case-only clashes, paths over 260 characters.
- Docker (arm64 couldn't check; `platform: linux/amd64`), native projects, secrets (names only), services (databases), Python version, `.venv`/`node_modules`, no lockfile.
- `tests/fixtures/clean/` (≥10 projects): no blockers and no "will need a fix".

### 5. Tier, headline, AI usage, hand-off note
### 6. `--online` PyPI wheel check (cached; canned responses in tests)
### 7. Design lab shell: index sidebar, OS + theme switches, scenario picker, hidden dev panel, shared tokens
### 8. Screens 1–3 on real scans (SSE bridge), then 4–9 with demo data; scenarios are real fixture projects scanned by the real scanner
### 9. App fixes: per-device "Files from <device>" replaces Send/Both/Receive; minimum arrival moment
### 10. CI (scanner tests on macOS and Windows), README, `docs/screens/`, final summary, independent review
