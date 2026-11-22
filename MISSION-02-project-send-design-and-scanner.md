# Mission 02: Project Send design lab + real scanner

**Product:** "Portal" (working name)
**Owner:** the owner
**Goal:** design every Project Send screen in a separate design lab, and build the first real piece of Project Send: a **scanner** that reads a project folder and produces an honest **blocker report**. Also fold two small fixes into the existing app.

Read this whole file before doing anything. Work in the existing `portal` repo.

---

## 0. How to work (same rules as Mission 01)

1. **Skills first.** List every skill you have. Load and use all that are relevant (Tauri, Rust, frontend/UI design, design systems, motion, accessibility, planning, test-driven development, debugging, verification, code review). List the ones you loaded at the top of `docs/PROGRESS.md`.
2. **Work autonomously. Don't stop to ask me questions.** Make the best decision, record it in `docs/DECISIONS.md` with a one-line reason, and keep going. If something truly needs me, skip it, continue, and list it under "Needs the owner" in `docs/PROGRESS.md`.
3. **Plan, then build.** Write the plan to `docs/PLAN-02.md`, then start.
4. **Small, verified steps** in the order of section 9. Tests pass at every step. Commit after each step. CI stays green.
5. **Be honest in reports.** Separate "tested and works" from "written but untested on real hardware".

### Decisions already made
- **The real app's look stays as it is.** The only app changes in this mission are the two fixes in section 7. The app's Project tab keeps its "Coming soon" state.
- **New screens live in a separate design lab** on localhost (section 5), not in the app.
- **Accounts:** design the screens only, with demo data. No real sign-up, backend or email collection yet.
- **Money:** spend nothing. Free tools and crates only.
- **The scanner never runs the project's code** and never reads secret values (section 3.6).

---

## 1. What Project Send is (context)

Free tier: send files between Mac and Windows (done in Mission 01).
**Project Send** (paid, later): send a code project and it arrives converted and running on the other OS, or with an honest report of what blocks it.

The pipeline: **1 Snapshot → 2 Pack → 3 Plan → 4 Apply → 5 Verify → 6 Report**, with **7 Repair agent** (AI) only when a check fails, within a usage budget. A knowledge base of error signatures and fix patterns feeds the plan and the repair agent.

**This mission builds the scanner** (the core of Snapshot and Plan) **and the free blocker report.** Conversion, verification and the AI are design-only for now.

Tiers decided:
- **Free:** scan and blocker report only. Nothing is changed or run.
- **Pro:** conversions with AI usage included on our key, up to a measured cap shown to the user; usage beyond the cap is charged; occasional credit gifts (e.g. $5).
- **Own key:** a cheaper monthly plan where the user plugs in their own AI API key.

### Product principle: honest by design
People trust Claude Code because they watch it work. Portal never shows a bare spinner.
- Always show the step list, what it's doing right now in plain words ("Checking 214 files for Windows-only code (120 of 214)"), and why when it matters.
- Anything ambiguous or failed is raised, never hidden.
- Every result has one of four states:

| State | Meaning |
|---|---|
| **Proved** | A check ran and passed |
| **Failed** | A check ran and failed, with the reason |
| **Couldn't check** | No way to test it here, so no claim is made |
| **Needs you** | Only the user can provide it (a secret, a licence, hardware, a decision) |

For the scanner, which runs nothing, findings use: **Blocker**, **Will need a fix** (fixable in a paid conversion), **Needs you**, **Couldn't check**, **Info**. Never say "will work"; the scanner can only say what it found.

---

## 2. Who uses it and what they move

Projects can contain anything. Language order for deep support: **Python first, then JavaScript/TypeScript, then shell scripts.** Everything else (Java, C#, C++, Swift, Go, Rust…) is detected and gets an honest summary and a hand-off note, never silence.

Every project gets a **tier** before anything else:

| Tier | Example contents | Meaning for the user |
|---|---|---|
| Static | Documents, data, images, HTML/CSS sites | Arrives clean; nothing to run |
| Scripts | Single scripts, notebooks, small tools | Environment rebuild + simple fixes |
| Apps | Web apps, APIs, local AI pipelines | Full pipeline; tests compared with the sender |
| Systems | Several services: databases, Docker, GPU, background workers | Several parts checked; some may need the user |
| Native | Xcode, WPF/.NET desktop, OS-specific C++ builds | Honest "Rebuild" verdict and a migration plan |

The report shows the tier and **expected AI usage** for a conversion (None / Rare / Sometimes / Often / Plan only). Label it as an estimate.

---

## 3. The scanner (real code)

### 3.1 Shape
- A new Rust crate, e.g. `portal-scan`, in the workspace. No Tauri dependency, so the app can use it later.
- A CLI: `portal-scan <folder> --target windows|macos [--json] [--online]`. Default target = the other OS from the machine it runs on.
- Output: a **versioned JSON report** (`schemaVersion`) plus a readable text summary. Write the JSON schema to `docs/scan-report.schema.json`. The design lab renders this exact JSON.
- Code parsing with **tree-sitter** grammars (Python, JavaScript/TypeScript, Bash). Don't use regex for code where syntax matters.
- Fast: a typical project (a few thousand files, excluding `node_modules`/`.venv`) scans in a few seconds. Report the timing.
- Respect `.gitignore` for what counts as the project, but still detect excluded heavy folders so the report can say what will be left out.

### 3.2 Snapshot (what works on this machine, read statically)
- **Ecosystems found:** Python (pyproject, requirements*.txt, uv.lock, poetry.lock, Pipfile, setup.py, environment.yml, notebooks), Node (package.json, package-lock/pnpm-lock/yarn.lock, tsconfig), shell scripts, Dockerfiles/compose, and other languages by file type and build files (`*.xcodeproj`, `*.sln`/`*.csproj`, `CMakeLists.txt`, `pom.xml`, `build.gradle`, `go.mod`, `Cargo.toml`…).
- **Versions:** the Python and Node versions the project asks for (`requires-python`, `.python-version`, `engines`, `.nvmrc`). Optionally the versions installed on this machine by running `python --version` / `node --version` only. **Never run project code.**
- **Installed packages,** read statically from `.venv`/`venv` metadata (`*.dist-info`) and `node_modules/*/package.json` if present. Note when there's no lockfile: the snapshot then becomes the lock.
- **How it's run:** entry points from `pyproject` scripts, `package.json` scripts, `Makefile`, `Procfile`, `.vscode/launch.json`, `if __name__ == "__main__"` files, and `README` code blocks. Record them; don't run them.
- **What would be left out of a send:** `.venv`, `node_modules`, `__pycache__`, build outputs, OS junk. Show their size: "1.8 GB of rebuildable files won't be sent."

### 3.3 Rules (findings). Python first, then JS/TS and scripts.
Each finding has: `id`, `category`, `state`, `severity`, `file`, `line`, `evidence` (short snippet), `why` (one plain sentence), `fixPattern`, `autoFixable` (bool, for later), and `targets` (which direction it affects).

Categories follow an MSR 2026 study of cross-OS issues in 2,042 Python projects (data under BSD-2: https://github.com/ncsu-swat/portability-issues-py): Files & paths, Processes & signals, Library dependencies, API availability, Environment & display, Permissions & limits, System information. Add our own: Copied environments, Scripts & line endings, Local AI hardware, Docker, Interpreter on Windows.

Minimum rule set for this mission:

| Rule | Detect | State when target is Windows / Mac |
|---|---|---|
| Hard-coded absolute paths (`C:\...`, `/Users/...`, `/home/...`) | String literals in code and config | Will need a fix |
| Backslash/forward-slash path building by string concatenation | Code | Will need a fix |
| `open()` / `read_text()` / `write_text()` without `encoding=` | Code (Windows uses the locale code page before Python 3.15) | Will need a fix (Windows) |
| `multiprocessing` / `ProcessPoolExecutor` without a `__main__` guard | Code (spawn is the default on Windows and macOS) | Will need a fix |
| Unix-only modules: `fcntl`, `termios`, `tty`, `pty`, `resource`, `pwd`, `grp`, `curses` | Imports | Blocker or Will need a fix (Windows) |
| Windows-only modules: `winreg`, `msvcrt`, `winsound`, `win32*` (pywin32) | Imports | Blocker or Will need a fix (Mac) |
| Unix-only OS APIs: `os.geteuid`, `os.getuid`, `os.uname`, `os.fork`, `signal.SIGKILL`/`SIGHUP`… | Code | Will need a fix (Windows) |
| `.venv`/`node_modules` present | Folders | Info: "won't be sent; rebuilt on the other side" |
| No lockfile | Files | Info: snapshot becomes the lock |
| Shell scripts with CRLF line endings | `.sh` files | Will need a fix (Mac) |
| `.sh` only, no `.ps1`/`.bat` twin (or the reverse) | Scripts referenced as run commands | Will need a fix |
| Missing executable bit risk | `+x` files going to Windows and back | Info |
| npm scripts using `rm -rf`, `cp`, single quotes, `VAR=x cmd`, `export` | `package.json` scripts | Will need a fix (Windows) |
| Symlinks | Files | Will need a fix (Windows needs admin or Developer Mode) |
| CUDA in code (`.cuda()`, `device="cuda"`, `torch.cuda`) going to a Mac | Code | Will need a fix: CUDA → MPS → CPU device picker |
| MLX imports going to Windows | Imports (MLX lists macOS and Linux) | Blocker, with a suggested alternative (Ollama or llama.cpp) |
| Docker images without arm64 support going to Apple Silicon | Dockerfile/compose `FROM` and `platform:` | Couldn't check unless online; flag |
| Native project files (`*.xcodeproj`, SwiftUI imports, WPF `*.csproj`) | Files | Blocker: tier Native, "Rebuild" |
| Secrets present (`.env`, `*.pem`, `credentials*`) | File names and `.env` **key names only** | Needs you: "Secrets aren't sent automatically" |
| Databases/services needed (Postgres, Redis, etc. in compose or config) | Config | Needs you or Info |
| Python version requested not installable on target | `requires-python` | Couldn't check offline; check online |

**Online package check (`--online`, off by default in tests):** for each Python dependency, read PyPI's JSON API to see whether wheels exist for the target (`win_amd64`, `macosx_*_arm64`) or a source build would be needed. Cache results. Offline, report "Couldn't check" for this part; never pretend.

### 3.4 Tier and summary
- Assign the tier (section 2) with a one-line reason ("Apps: a FastAPI server plus a local model via Ollama").
- Headline sentence, honest: e.g. "3 things block this on Windows, 7 need a fix, 1 needs you." or "No blockers found. 4 things will need a fix."
- Expected AI usage estimate, labelled as an estimate.

### 3.5 Hand-off note
For anything not deeply supported (other languages, Native tier), produce a short Markdown note the user can give their own coding AI: what the project is, what blocks it on the target OS, and suggested steps. Keep it factual: no promises.

### 3.6 Safety
- Never execute project code, scripts, notebooks or package managers on the project.
- Never read values from `.env` or key files; key names only.
- Network only with `--online`, only to PyPI's JSON API, and say so in the report.
- Don't follow symlinks out of the project folder.

### 3.7 Tests (the Phase 1 gate)
- A fixture folder of **small broken sample projects**, one or more per rule, each with the expected findings. The scanner must find **every planted problem**.
- At least **10 clean sample projects** (Python, Node, static, scripts) with **zero false blockers**.
- You may use examples from the MSR 2026 dataset (BSD-2) as fixtures, with attribution in `docs/THIRD_PARTY.md`.
- Golden JSON snapshots for the report format.
- Run the scanner on the `portal` repo itself as a smoke test.

---

## 4. Screens to design (design lab, demo data + real scan)

Design every screen for **both the macOS and Windows window**, **light and dark**, in the existing Portal style: same tokens, type, keycap buttons and edge glow. Reuse the app's components and tokens from the repo (import them; don't copy-paste diverging versions).

1. **Project tab entry:** drop a project folder; short explanation of Free scan vs Pro conversion.
2. **Scanning in progress:** live step list (Reading files → Finding ecosystems → Checking for {target}-only code → Checking packages → Writing the report), counts, what it's checking right now, elapsed time. No bare spinner.
3. **Blocker report (free):** headline answer first; tier badge with its reason; findings grouped by state (Blocker / Will need a fix / Needs you / Couldn't check / Info); each finding expands to why, file and line, evidence and the fix pattern; "1.8 GB won't be sent"; secrets notice; hand-off note export; a calm "Convert with Pro" call to action.
4. **Consent stops:** "This project wants to run `python main.py`. Run it?" and "Send 12 files to the AI to fix this? Here's exactly what's sent (no secrets)."
5. **Conversion progress (demo):** the 7-step pipeline; plain-words status; changes appearing as readable diffs; the AI activity panel (what it's trying, attempt 2 of 3, usage spent against the cap).
6. **Final report (demo):** leads with the answer ("Runs on Windows. 2 things need you."); every result in one of the four honest states; patch review with per-change undo; export a hand-off plan.
7. **Native "Rebuild" verdict:** for an Xcode or WPF project; blockers and the migration plan; no fake progress.
8. **Accounts and plans (demo only):** sign in; account page; plans (Free / Pro / Own key); usage meter against the cap; credit gift notice ("You've been given $5 of AI usage"); own-key entry with a clear explanation; what happens when the cap is reached.
9. **Edge states:** scan of an empty folder; huge folder; unreadable files; offline "Couldn't check"; AI budget used up; user declines consent.

**Demo scenarios** (switchable in a hidden dev panel):
- A Python local-AI app going Mac → Windows: converts, with 2 "needs you" items.
- A Node web app with npm script problems going Mac → Windows.
- A Windows Python tool going to Mac with CRLF scripts and `winreg`.
- An Xcode app going to Windows: Rebuild.
- A static folder of documents: nothing to fix.

**The scan screens must also work with the real scanner:** the lab can scan a real folder on this machine (pick or type a path) and render the real JSON. Choose the simplest local bridge (for example a dev-server endpoint that runs the `portal-scan` CLI) and record the choice.

Motion: subtle and purposeful. Steps tick off, findings slide in as found, diffs appear line by line. Respect reduced motion.

---

## 5. The design lab

- A separate app in the repo (e.g. `design-lab/`), run with one command on localhost. Document it in the README.
- It imports the real app's design tokens and components, so the lab and the app never drift apart.
- A simple screen index (sidebar) listing every screen and state, plus the macOS/Windows and light/dark switches, and the demo scenario picker.
- Nothing from the lab ships in the app yet.

---

## 6. Copy (words on screen)

Plain, calm and specific, like the existing app. Say what happened and what to do next. No jargon in main text; technical detail goes in an expandable "Details". Never imply something works if it wasn't proved.

---

## 7. Two fixes in the real app

1. **Remove the Send / Both / Receive toggle** from the main window. Replace it with a per-device setting in **Settings → Devices**: "Files from <device>: **Ask every time** · **Always accept** · **Never**" (default: Ask every time). Migrate existing settings sensibly (old "Always accept" → Always accept). Keep enforcement in the core, with a clear sender-side message when the receiver is set to Never.
2. **Arrival moment minimum duration:** when a transfer finishes almost instantly, still play the edge glow and "landed" moment for a short minimum (around 0.8–1 s), skipping the progress bar. Respect reduced motion.

Update tests and `docs/MANUAL-TESTS.md` for both. Nothing else in the app changes.

---

## 8. Definition of done

1. `portal-scan` CLI works on Mac and Windows (CI builds and tests on both), produces schema-valid JSON and a readable summary.
2. Every rule in section 3.3 has fixtures; every planted problem is found; zero false blockers on the clean samples.
3. Scanning the `portal` repo and a 5,000-file sample finishes in a few seconds (report the numbers).
4. The design lab runs on localhost with every screen in section 4, both OS window styles, both themes and all demo scenarios.
5. The lab scans a real folder and renders the real report.
6. Both app fixes work and are tested; the app looks otherwise unchanged.
7. `docs/PROGRESS.md` has a final summary: what's done, screenshots of key screens (saved under `docs/screens/`), what's untested, and "Needs the owner".

---

## 9. Build order
1. Plan; workspace setup for `portal-scan` and `design-lab`.
2. Scanner skeleton: walk, ignore rules, ecosystem detection, JSON schema, golden tests.
3. Snapshot: versions, packages (static), run commands, left-out folders.
4. Rules, Python first, each with fixtures; then JS/TS, scripts, Docker, native, secrets.
5. Tiering, headline, AI-usage estimate, hand-off note.
6. Optional `--online` PyPI wheel check with cache.
7. Design lab shell: index, OS/theme switches, scenarios, shared tokens.
8. Screens 1–3 wired to real scans; then 4–9 with demo data.
9. The two app fixes.
10. CI, README, screenshots, final summary.

---

## 10. Future notes (don't build now)
- Conversion (Apply), Verify ladder, Repair agent, real accounts and billing, usage metering, opt-in fix receipts (error signature + fix pattern + outcome; never code), knowledge base, Node/TS deep support, privacy mode with a local model.
- Choosing a device on the network (more than one paired device).
- Mission 03: resume interrupted transfers, plus any polish from testing.
