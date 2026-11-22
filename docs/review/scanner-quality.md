# Scanner quality review: portal-scan on 20 real projects

Reviewer: Claude (agent), 2026-11-01. Binary: `target/release/portal-scan` 0.1.0 (schemaVersion 1). Scanner source and docs were not opened. Judgments come from CLI output and the scanned projects' own code.

**Method.** I shallow-cloned 20 public repos into the scratch folder and scanned each with `--json` for `--target windows` and `--target macos` (40 offline scans). I then ran 10 more scans with `--online` on 5 Python projects. I sampled 130 findings across 23 rules, opened each cited line and judged it. I grepped and read entry points for misses, and loaded 6 reports in the lab (`localhost:5190`, "Scan a real folder").

Note on the macOS target: the CLI runs on a Mac, so `--target macos` reports `"source": "macos"`. The rules behave as if the project came from a Windows PC, and the lab presents it that way ("read on this PC"). I judged macOS findings as "project arriving from Windows".

## Projects

Scan time is the median wall-clock time of 3 runs per target (`--json`, warm cache). Size is the scanner's own `stats.bytes`/`stats.files`, which excludes `.git`.

| # | Project (URL) | Commit | Type | Files | Size | Win | Mac |
|---|---|---|---|---|---|---|---|
| 1 | https://github.com/cookiecutter/cookiecutter | c88fbe921c | Python CLI | 327 | 1.0 MB | 0.21 s | 0.34 s |
| 2 | https://github.com/ranger/ranger | 46e957af97 | Python curses TUI (POSIX) | 158 | 1.4 MB | 0.26 s | 0.28 s |
| 3 | https://github.com/miguelgrinberg/microblog | a975ef6486 | Python Flask web app + Docker | 73 | 0.1 MB | 0.12 s | 0.11 s |
| 4 | https://github.com/karpathy/nanoGPT | 3adf61e154 | Python ML (PyTorch) + 2 notebooks | 26 | 0.6 MB | 0.10 s | 0.08 s |
| 5 | https://github.com/karpathy/nn-zero-to-hero | 73c3fcc741 | Jupyter notebooks | 9 | 1.2 MB | 0.10 s | 0.08 s |
| 6 | https://github.com/CamDavidsonPilon/Probabilistic-Programming-and-Bayesian-Methods-for-Hackers | 5b33f77a80 | Jupyter notebooks (book, 34 nb) | 387 | 41.1 MB | 0.45 s | 0.20 s |
| 7 | https://github.com/pexpect/pexpect | fc8f062518 | Python lib (pty/fork/termios) | 144 | 0.7 MB | 0.16 s | 0.13 s |
| 8 | https://github.com/pywinauto/pywinauto | a428d9617a | Python, Windows GUI automation | 944 | 653.7 MB | 0.61 s | 0.44 s |
| 9 | https://github.com/taniarascia/takenote | e0eddbb9a2 | TS/React web app + Express | 178 | 4.4 MB | 0.13 s | 0.13 s |
| 10 | https://github.com/vercel/serve | 37b3a743fa | TS Node CLI | 57 | 0.6 MB | 0.09 s | 0.09 s |
| 11 | https://github.com/isaacs/rimraf | 051e6c72a7 | TS library | 71 | 0.4 MB | 0.37 s | 0.16 s |
| 12 | https://github.com/StartBootstrap/startbootstrap-agency | b2d5d5c6b1 | Static website (Pug/SCSS) | 86 | 2.6 MB | 0.08 s | 0.08 s |
| 13 | https://github.com/dockersamples/example-voting-app | 63e9150ca1 | docker-compose (Py/Node/C#) | 51 | 0.5 MB | 0.26 s | 0.29 s |
| 14 | https://github.com/rxhanson/Rectangle | 39466aea1a | Swift/Xcode macOS app | 487 | 5.2 MB | 0.13 s | 0.06 s |
| 15 | https://github.com/File-New-Project/EarTrumpet | 5bd32a68f7 | C# WPF Windows app | 481 | 9.5 MB | 0.06 s | 0.05 s |
| 16 | https://github.com/nvm-sh/nvm | 5e38bc0cae | POSIX shell | 507 | 1.7 MB | 0.29 s | 0.16 s |
| 17 | https://github.com/antirez/kilo | 323d93b29b | C, termios editor | 6 | 0.04 MB | 0.02 s | 0.01 s |
| 18 | https://github.com/ScoopInstaller/Scoop | e6aa3b366b | PowerShell, Windows-only | 137 | 1.4 MB | 0.04 s | 0.03 s |
| 19 | https://github.com/junegunn/fzf | 26ff936df0 | Go CLI + shell integration | 163 | 1.9 MB | 0.08 s | 0.03 s |
| 20 | https://github.com/typicode/json-server | 89a34a44b7 | TS Node CLI | 33 | 0.1 MB | 0.09 s | 0.09 s |

## Precision per rule

I sampled 130 findings, roughly in proportion to volume, with every rule covered. A finding is **correct** if it would break, or clearly needs action, on the other computer. It is a **false alarm** if it would not. It is **debatable** if it's technically true but harmless or irrelevant (for example, ASCII-only files, code that only runs on a third OS, or test-only fixtures). Precision = correct / (correct + false alarms).

| Rule | State it reports | Sampled | Correct | False alarm | Debatable | Precision |
|---|---|---|---|---|---|---|
| unix-module | blocker | 8 | 7 | 0 | 1 | 100% |
| unix-api | willNeedAFix | 5 | 5 | 0 | 0 | 100% |
| unix-signal | willNeedAFix | 5 | 4 | 0 | 1 | 100% |
| npm-script-unix | willNeedAFix | 6 | 6 | 0 | 0 | 100% |
| make-on-windows | willNeedAFix | 5 | 5 | 0 | 0 | 100% |
| symlink | willNeedAFix | 4 | 4 | 0 | 0 | 100% |
| native-apple | blocker | 1 | 1 | 0 | 0 | 100% |
| executable-bit | info | 2 | 2 | 0 | 0 | 100% |
| no-lockfile | info | 2 | 2 | 0 | 0 | 100% |
| wheels-unchecked | couldntCheck | 2 | 2 | 0 | 0 | 100% (but noise) |
| shell-command | willNeedAFix | 3 | 2 | 1 | 0 | 67% |
| windows-module | blocker | 8 | 3 | 3 | 2 | 50% |
| open-encoding | willNeedAFix | 13 | 3 | 4 | 6 | 43% |
| env-var | willNeedAFix | 6 | 2 | 3 | 1 | 40% |
| docker-needed | needsYou | 5 | 1 | 2 | 2 | 33% |
| secrets | needsYou | 6 | 1 | 3 | 2 | 25% |
| path-concat-slash | willNeedAFix | 15 | 1 | 13 | 1 | 7% |
| absolute-path | willNeedAFix | 14 | 0 | 12 | 2 | 0% |
| path-concat-backslash | willNeedAFix | 6 | 0 | 6 | 0 | 0% |
| system-path | willNeedAFix | 4 | 0 | 4 | 0 | 0% |
| docker-arm64 | couldntCheck | 6 | 0 | 6 | 0 | 0% |
| source-only (online) | couldntCheck | 2 | 0 | 1 | 1 | 0% |
| service / service-client | needsYou / info | 2 | 0 | 0 | 2 | n/a |
| **Total** | | **130** | **51** | **58** | **21** | **47%** (39% counting debatable as wrong) |

The rules that look for something specific (POSIX modules, signals, npm shell syntax, make, symlinks, Xcode) are reliable. The rules built on string patterns (paths, slashes, encodings, Docker images, secrets) carry the volume and are mostly wrong. `open-encoding`, `path-concat-slash` and `absolute-path` alone are 324 of the 598 listed findings (54%).

Representative false alarms:
- `absolute-path`: test data in cookiecutter `tests/test_get_config.py:58` (`'/home/example/some-path-to-templates'`, compared as a string). Also rimraf `test/default-tmp.ts:18` (mocked `'d:\\temp'`), CI YAML (pywinauto `appveyor.yml:15`, EarTrumpet `.github/workflows/main.yml:317`), a **comment** in nvm `.github/workflows/windows-nvm.yml:14`, and the Ubuntu deployment config in microblog `deployment/supervisor/microblog.conf:2`.
- `path-concat-slash`: GitHub API URLs in takenote `src/server/handlers/sync.ts:21`, a module specifier at `src/client/utils/helpers.ts:255`, the "3/10" counter in ranger `gui/widgets/statusbar.py:309`, `print('/' + '=' * ...)` in pexpect `examples/hive.py:268`, and vendored `result/views/angular.min.js:6`.
- `path-concat-backslash`: the null-separator `'\0'.join(...)` in ranger `core/fm.py:554`, shell escaping in `ext/shell_escape.py:14`, and box-drawing `print('\\' + ...)`.
- `system-path`: ranger `container/file.py:90` only *compares* a path to `'/proc/kcore'` and never reads it. pywinauto `linux/application.py:109` is the Linux backend, selected by `sys.platform` in `pywinauto/application.py:78-81`.
- `windows-module`: pywinauto's own package flagged as a Windows-only third-party module. This includes `examples/macos/os_update.py:39` (`from pywinauto.macos.application import Application`, "on a Mac the import fails") and `pywinauto/__init__.py:110`.
- `docker-arm64`: all 6 flagged images are multi-arch with arm64. I checked them in the Docker Hub registry: `dockersamples/examplevotingapp_*`, `rubylang/ruby:3.4.1-noble`, `mcr.microsoft.com/dotnet/*`. The voting-app `worker/Dockerfile:11` even builds with `--platform=${BUILDPLATFORM}` and `TARGETARCH`.
- `secrets`: serve `.npmrc` (`save-exact=true`) and nvm `.npmrc` (`package-lock=false`) are reported as "holds credentials". serve's `cert.pem` is a public certificate.
- `env-var`: ranger `core/main.py:63` *sets* `SHELL` when it is missing. That's the fix, reported as a problem. `ext/rifle.py:548-549` is already guarded by `'XDG_CONFIG_HOME' in os.environ`.
- `open-encoding`: 50 findings, all in cookiecutter `tests/`. That suite runs on `windows-latest` in the project's own CI (`.github/workflows/tests.yml:84-86`).

## Misses (10 projects)

| Project → target | File:line | What was missed | Why it matters |
|---|---|---|---|
| kilo → Windows | `kilo.c:41`, `:50`, `:207`, `:334` | `#include <termios.h>`, `<sys/ioctl.h>`, `tcsetattr`, `ioctl(TIOCGWINSZ)` | Won't compile on Windows at all. The report says "Static: documents; nothing to run" with only "Uses make", and the lab offers "Pro converts it for you". |
| EarTrumpet → macOS | `EarTrumpet/EarTrumpet.csproj:9,13`; `EarTrumpet/Interop/Ole32.cs:7` (72 `[DllImport]`s) | WPF `WinExe` on .NET Framework v4.6.2, P/Invoke into Windows DLLs | Can't run on a Mac. The report says "No blockers found", tier "Scripts: 4 Windows scripts", and offers "Convert with Pro". It's asymmetric: Rectangle gets a `native-apple` blocker, but there's no Windows equivalent. |
| Scoop → macOS | `lib/system.ps1:37,42`; `lib/diagnostic.ps1:45` | Registry (`HKLM:`, `HKCU:`, `Microsoft.Win32`), `.exe` shims; a Windows-only package manager | "No problems found for macOS." The "Windows scripts" ecosystem claims deep support but found nothing. |
| Scoop → Windows | `lib/json.ps1:96` | `Add-Type ...\supporting\validator\bin\Newtonsoft.Json.dll`. That folder is git-tracked, but the scanner labels it `buildOutput`, `rebuildable: true` and won't send it. | Portal's own transfer would break `json_path` (checkver/autoupdate). The lab says it "is rebuilt on MacBook Pro from the project's own files". No build of it exists in the repo flow. |
| nvm → Windows | `nvm.sh`, `install.sh` (whole product) | POSIX shell function sourced into bash/zsh. No shell finding of any kind. | "No blockers found." The tool cannot be used on Windows without WSL. The tier says "Apps: an app that runs in Docker", although `Dockerfile:3` says "development environment only". |
| nanoGPT → macOS | `train.py:72`, `sample.py:20`, `bench.py:17`; `train.py:70` | `device = 'cuda'` default; `backend = 'nccl'` | The first run fails on a Mac. The project's README:105 says to pass `--device=mps`. The report says "No problems found for macOS." |
| nanoGPT → Windows | `train.py:74`, `:208` | `compile = True` → `torch.compile` | The project's README:224 says it isn't available on Windows. Not reported. |
| pexpect → Windows | `setup.py:76`; `pexpect/pty_spawn.py:10` | `install_requires=['ptyprocess']`, a POSIX-only package; not flagged even with `--online` | The online check only asks "is there a wheel?", and pure-Python POSIX-only packages pass. |
| ranger → Windows | `ext/popen_forked.py:17`, `ext/rifle.py:139`, `container/bookmarks.py:198`; `ext/human_readable.py:66` | `os.fork()`/`os.chown()` inside `try: ... except OSError`; `strftime("%-d %b %Y")` | On Windows these raise `AttributeError`/`ValueError`, which `except OSError` does not catch. The scanner seems to treat any `try` as a guard. |
| ranger → macOS | `Makefile:81` | `sed -i 's\|...\|g'` (GNU form) | BSD sed on macOS rejects it. No shell rule fired in any of the 20 repos. |
| Probabilistic… → both | 15 of 34 `.ipynb` (every notebook > 1 MiB), e.g. `Chapter5_LossFunctions/Ch5_LossFunctions_TFP.ipynb` cell 38 `!unzip -q data.zip -d data` | Large notebooks are silently skipped (`stats.skippedFiles: 15`). The CLI just says "36 parsed". | 44% of the book went unchecked. Smaller siblings with the same code got findings, the big ones didn't. Notebooks are big because of embedded plots, which is normal. `unzip` is not on Windows. |
| microblog → Windows | `requirements.txt:25`; `boot.sh:11` | `gunicorn` (POSIX-only server) | Low priority: dev uses `flask run`. Same gap as ptyprocess: no per-package OS knowledge. |

## Noisy rules

- **path-concat-slash** (110 listed): URLs, `require()` specifiers, display strings, dict keys, vendored minified JS. `/` works in Windows file APIs, yet every hit is "Will need a fix".
- **absolute-path** (102 listed, 0% in sample): test fixtures, CI files, deployment configs, comments, and Windows-only examples that are already blocked. The wording "only exists on this computer" is wrong for every one of them: `/home/ubuntu` and `C:\Python37` do not exist on this computer either.
- **open-encoding** (112 listed): mostly tests and ASCII-only files. One "add `encoding=`" item per file swamps the one or two that matter (nanoGPT `data/shakespeare_char/prepare.py:16` writes downloaded text; the sibling script already uses `utf-8`).
- **path-concat-backslash** (12 listed, 0%): escape sequences and string art.
- **wheels-unchecked**: appears in all 14 offline Python reports and turns "No problems found" into "...but 1 thing couldn't be checked". Fine once; noisy as a headline modifier.
- **docker-arm64**: 6 of 6 wrong. **secrets** on `.npmrc` without tokens.
- Cross-cutting: no distinction between `tests/`, `examples/`, `docs/`, `.github/`, vendored `*.min.js` and the code that actually runs. Of the 558 listed findings that cite a file, 386 (69%) are in tests, examples, docs, notes, benchmarks, CI, deployment configs or vendored files.

## Scan time

- Median per-scan wall time (median of 3 runs): **0.11 s**; p90 0.37 s; max **0.61 s** (pywinauto → Windows).
- Scanner-reported `durationMs`: median 94 ms, max 599 ms.
- Slowest: **pywinauto**. It has 944 files and 654 MB, mostly checked-in test-app binaries, and 166 Python files are parsed. Second slowest is rimraf → Windows (0.37 s: 49 TS files, 193 findings, almost all `path-concat-slash`). Size on disk matters little: the 41 MB notebook book scans in 0.2-0.45 s, partly because it skips its big notebooks (see Misses).
- One timing pass ran while the machine was heavily loaded (load average about 340 from other processes). Single scans then took 20-100 s (pywinauto → macOS 102 s, nvm 30 s, fzf 30 s), and `durationMs` matched. That's environmental, not a scanner defect, but there is no timeout or partial result. `--progress` does stream JSON lines.
- `--online` added up to 6.6 s on the first lookup (microblog → Windows, 54 packages). Repeat lookups were near-instant (apparently cached).

## Would scare or mislead a beginner

Screenshots of the lab are in `docs/review/shots/scanner-quality-*.png`.

1. **kilo → Windows** (`scanner-quality-kilo-win.png`): "No blockers found. 1 thing will need a fix." Tier "Static: documents; nothing to run". Then "Pro converts it for you… checks that it installs, passes its tests and starts." That's a paid promise for a program that cannot compile on Windows.
2. **Scoop → macOS** (`scanner-quality-scoop-mac.png`): "No problems found for macOS." plus "Send as files" for a Windows package manager. It also claims a committed DLL folder "is rebuilt on MacBook Pro from the project's own files".
3. **EarTrumpet → macOS** (`scanner-quality-eartrumpet-mac.png`): "No blockers found" and "Convert with Pro" for a WPF app. The only fix listed is a CI workflow path, described as "C:\Program is a folder on the computer this was written on". The path is cut at the space, and the suggested fix is Python/Node code for a YAML file.
4. **Rectangle → macOS** (`scanner-quality-rectangle-mac.png`): "This needs a rebuild for macOS: it's an Xcode app for Apple systems." The handoff suggests moving the macOS app to ".NET MAUI, Avalonia, Flutter" to run on macOS.
5. **nanoGPT → macOS** (`scanner-quality-nanogpt-mac.png`): "No problems found", but `python train.py` fails on a Mac because the default device is CUDA.
6. **cookiecutter → Windows** (`scanner-quality-cookiecutter-win.png`): "5 things will need a fix", with 14 + 50 places, for a project whose test suite already passes on Windows CI. The scariest-looking item, "Uses a path that only exists on this computer", points at test fixtures.
7. **fzf / nvm**: "Needs Docker" (Needs you) and tier "Apps: an app that runs in Docker". In both repos, Docker is only a test/dev environment. A beginner would install Docker Desktop for nothing.
8. **Probabilistic Programming book**: tier "Apps: a Python web server" for a folder of notebooks. The cause is probably `wsgiref>=0.1.2; python_version < '3.0'`; the marker is ignored, and online mode then says wsgiref "has no ready-made build".
9. **pywinauto → macOS**: macOS example scripts are told "pywinauto only exists on Windows; on a Mac the import fails".
10. **serve / nvm**: ".npmrc holds credentials. Portal never sends secrets without asking", for files containing `save-exact=true` and `package-lock=false`.
11. **example-voting-app**: tier reason "a Flask app and Redis, PostgreSQL, Redis, PostgreSQL, Redis and PostgreSQL in Docker". It also says "Needs Redis running" (Needs you) when compose already runs Redis.

## Findings

**1. Whole classes of non-portable projects get a clean or near-clean report.** Severity: **blocks use**.
Evidence: kilo → Windows (termios C), EarTrumpet → macOS (WPF/.NET Framework/P/Invoke), Scoop → macOS (registry PowerShell), nvm → Windows (POSIX shell). All say "No blockers"/"No problems", and two offer "Convert with Pro". Rectangle gets `native-apple`, so the asymmetry is visible.
Suggest: a `native-windows` rule from `.csproj`/`.vbproj` (`UseWPF`, `UseWindowsForms`, `OutputType WinExe`, `TargetFrameworkVersion v4.*`, `net*-windows`, `DllImport` of Windows DLLs). Scan C/C++ headers for POSIX-only (`termios.h`, `sys/ioctl.h`, `unistd.h`) and Windows-only (`windows.h`) includes. PowerShell rules for `HKLM:`/`HKCU:`, `Microsoft.Win32`, `$env:USERPROFILE`, `.exe` shims. A "shell-script project" blocker for Windows when the product itself is a `.sh` (sourced or installed). Never show the Pro conversion pitch for a tier the scanner only summarized.

**2. Committed files the app needs are classified as rebuildable and left out.** Severity: **blocks use**.
Evidence: Scoop `supporting/validator/bin` (git-tracked DLLs loaded at `lib/json.ps1:96`) is `buildOutput, rebuildable: true`, and the lab says it will be rebuilt. nvm's tracked `test/fast/.../node_modules` fixtures are also dropped.
Suggest: treat git-tracked files as source unless a build step that produces them is found. Only claim "rebuilt from the project's own files" when that recipe exists.

**3. No GPU/ML portability rule.** Severity: **blocks use** (for the ML/notebook audience).
Evidence: nanoGPT `train.py:72` `device='cuda'`, `:70` `nccl`, `:74/208` `torch.compile` (Windows). The macOS report says "No problems found".
Suggest: flag `'cuda'` defaults, `.cuda()`, and `torch.cuda.*` without an `is_available()` guard for macOS (suggest `mps`/`cpu`). Flag `torch.compile`/triton for Windows, `nccl` for non-Linux, and `bitsandbytes`/`flash-attn`/`xformers`-style dependencies.

**4. Same-OS / native report contradicts itself.** Severity: **confuses**.
Evidence: Rectangle → macOS has zero findings, but the headline is "This needs a rebuild for macOS" and the handoff suggests MAUI/Avalonia to run on macOS.
Suggest: only fire the native verdict when the app's platform ≠ the target. Otherwise say "This is already a macOS app".

**5. Path and string rules have ~0-7% precision and dominate the report.** Severity: **annoys** (and erodes trust in the real items).
Evidence: `absolute-path` 0/12, `path-concat-slash` 1/14, `path-concat-backslash` 0/6, `system-path` 0/4 (see "Precision per rule" and "Noisy rules" above).
Suggest: only flag strings that reach a file API (`open`, `Path`, `fs.*`, `os.path.*`). Skip URLs, comments, `*.min.js`/vendored files, CI configs (`.github/`, `appveyor.yml`) and deployment configs. Treat `\0`/`\\` escape sequences as non-paths. Downgrade `/`-joins to info (they work on Windows). Reword to "a path from a specific machine". Don't cut evidence at spaces (`C:\Program`). Make the fix text match the file's language.

**6. Tests, examples, docs and CI are reported like the running app.** Severity: **confuses**.
Evidence: 50/50 cookiecutter `open-encoding` hits are in `tests/` (which pass on Windows CI). pexpect's `tests/platform_checks/*` are Python 2 notes. pywinauto's Windows-only `examples/` are reported as blockers.
Suggest: tag each finding with a file role (app, test, example, docs, CI, vendored). Collapse non-app roles into one "Only affects tests/examples" group, and keep them out of the headline count.

**7. Platform guards are handled backwards.** Severity: **confuses**.
Evidence: these are flagged although they're guarded: platform-specific backends (pywinauto `linux/`, `macos/` via `application.py:78-81`; pexpect `pty_spawn.py`, imported only under `if sys.platform != 'win32'` in `__init__.py:79`), and the project's own package treated as a third-party Windows module. These are suppressed although they aren't guarded: `os.fork()`/`os.chown()` inside `try/except OSError` (ranger `ext/popen_forked.py:17`, `container/bookmarks.py:198`).
Suggest: resolve local packages before the known-module list. Honor platform folder/file names and import-site guards. Count a `try` as a guard only if it catches `AttributeError`/`ImportError`/`Exception`. Add `strftime('%-d')`.

**8. Large notebooks are skipped silently.** Severity: **confuses**.
Evidence: 15 of 34 notebooks over 1 MiB in the Probabilistic book have `skippedFiles: 15`. The CLI shows only "36 parsed" and "No problems found for macOS". Their code cells (e.g. `!unzip`) were never read.
Suggest: strip outputs and parse code cells regardless of file size. If anything is skipped, list it under "Couldn't check". Also add a rule for notebook shell magics (`!unzip`, `!wget`, `!ls`).

**9. Tier and Docker classification from incidental files.** Severity: **confuses**.
Evidence: fzf (test Dockerfile) and nvm ("development environment only") are "Apps: an app that runs in Docker" with "Needs Docker". kilo is "Static: documents". Probabilistic is "Apps: a Python web server". EarTrumpet and Scoop are "Scripts".
Suggest: let a Dockerfile drive the tier only when compose files, README run instructions or the run commands make it the primary way to run. Honor PEP 508 markers. A C/C++ project with a Makefile is native code.

**10. Secrets rule judges by filename.** Severity: **confuses**.
Evidence: token-free `.npmrc` (serve, nvm) is reported as "holds credentials" and withheld. A public `cert.pem` is reported as a secret. serve's test-fixture keys are withheld, so its SSL tests will fail on the other side.
Suggest: look inside `.npmrc` for `_authToken`/`_auth`/`password`. Distinguish certificates from private keys. Offer to send test fixtures with a one-click "these are test keys" choice.

**11. Docker image arm64 check guesses wrong.** Severity: **annoys**.
Evidence: 6/6 flagged images ship arm64. One Dockerfile already uses `BUILDPLATFORM`/`TARGETARCH`.
Suggest: check the registry manifest in `--online` mode, as PyPI is checked. Recognize `--platform=$BUILDPLATFORM`. When offline, emit one combined "couldn't check images" line.

**12. Package OS knowledge is wheel-only.** Severity: **annoys**.
Evidence: `ptyprocess` (pexpect `setup.py:76`) and `gunicorn` (microblog `requirements.txt:25`) are not flagged even with `--online`. `flask-mail` (pure-Python sdist) is "couldn't tell if it needs a compiler". `wsgiref` is flagged despite `python_version < '3.0'`.
Suggest: a small list of POSIX-only/Windows-only packages, plus PyPI `Operating System ::` classifiers when online. Treat pure-Python sdists as fine. Evaluate environment markers.

**13. Shell ecosystem shows "deep" support but has no rules.** Severity: **annoys**.
Evidence: 0 shell findings across 20 repos, including ranger `Makefile:81` `sed -i` (GNU-only) for macOS and nvm/fzf bash scripts for Windows.
Suggest: rules for GNU-vs-BSD flags (`sed -i`, `readlink -f`, `date -d`, `grep -P`, `stat -c`) for macOS, and "needs bash (Git Bash/WSL)" for Windows.

**14. Copy and polish.** Severity: **polish**.
Evidence: the tier reason repeats "Redis, PostgreSQL" three times. The handoff says "Work through the items above" when there are none (Scoop). `service` (needsYou) vs `service-client` (info) is inconsistent, and Redis is required although compose provides it. `wheels-unchecked` is appended to every offline Python headline. The CLI accepts `--target` equal to the source OS with no note.
Suggest: dedupe services, drop empty-step text, mark compose-provided services as handled, and move the offline note to the footer.
