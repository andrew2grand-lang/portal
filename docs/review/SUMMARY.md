# Fresh-eyes review: summary

Seven reviewers looked at the design lab and the scanner. None of them was given the mission, plan, decisions or progress docs. Each got only this sentence: "Portal moves files and code projects between a Mac and a Windows PC. Project Send scans a project and tells you what would break on the other computer, and the paid tier fixes it." They also got the lab URL and the `portal-scan` CLI, and were told not to open anything in the repo.

| Reviewer | Report |
|---|---|
| Beginner on a Windows gaming PC (friend's Python project from a Mac) | [persona-beginner.md](persona-beginner.md) |
| Mac developer sending a local-AI app to a client's PC | [persona-mac-dev.md](persona-mac-dev.md) |
| Non-technical person sending documents and a small script | [persona-nontechnical.md](persona-nontechnical.md) |
| Skeptical power user (git + an AI assistant) | [persona-power-user.md](persona-power-user.md) |
| Scanner quality on 20 real open-source projects | [scanner-quality.md](scanner-quality.md) |
| Copy and trust | [copy-and-trust.md](copy-and-trust.md) |
| Accessibility and polish | [accessibility-polish.md](accessibility-polish.md) |

Screenshots are in [shots/](shots/). This summary merges and de-duplicates the findings in those reports. I didn't re-run each finding myself. I did confirm the two lab bugs (B10, A9) directly. Nothing has been fixed.

## The picture in five lines

1. **The scanner can't yet be trusted with its all-clear.** It's accurate on Python and Node patterns it knows: POSIX modules, signals, npm shell syntax, make, symlinks and Xcode scored 100%. But on 20 real projects, overall precision was **47%** (51 correct, 58 false alarms, 21 debatable). Projects that can't run on the other OS often got "No problems found": C with termios, a WPF app sent to a Mac, Windows registry PowerShell, a shell-script tool, Rust/Go probes, and a Mac shell script.
2. **Privacy and proof claims are broader than what was checked.** Examples: "nothing leaves this Mac" while PyPI is contacted, "Secrets stay on this Mac" while keys in code are sent, "Proved" with nothing to inspect, and "14 of 14 passed" followed by "1 test fails".
3. **The paid moment is unclear and a little pushy.** The free scan sits under a "Paid" tab. The price never appears where you decide. "Convert with Pro" skips sign-in, plan and price. "The AI" is never named.
4. **Real data breaks the layout and focus.** Long paths are cut off with no way to see them, focus is lost on every screen change, and a few colors fail contrast.
5. **What worked:** the reviewers singled out the change review (diffs, per-change Undo, "By the AI"), the AI consent screen, the Rebuild verdicts for Xcode/WPF, the honest "Couldn't check / nothing is claimed" pattern, file:line evidence, and scan speed (median 0.11 s).

---

## Blocks use

These stop the person, or would lead them to a wrong decision with real cost: a project that won't run, money, or privacy.

### B1. The scanner gives clean reports for projects that won't run on the other OS
- **Where:** Blocker report (real scans) and CLI.
- **Screenshots:** [kilo → Windows](shots/scanner-quality-kilo-win.png), [EarTrumpet → Mac](shots/scanner-quality-eartrumpet-mac.png), [Scoop → Mac](shots/scanner-quality-scoop-mac.png), [nanoGPT → Mac](shots/scanner-quality-nanogpt-mac.png), [ripgrep](shots/persona-power-user-04-ripgrep-report.png), [Mac shell script](shots/persona-nontechnical-03-script-finding.png), [start path missed](shots/persona-mac-dev-01-real-report.png).
- **What happens:**
  - "No problems found" or "No blockers found" for:
    - a C editor built on termios (kilo, labelled "Static: documents");
    - an old-style .NET Framework WPF app with P/Invoke, sent to a Mac (EarTrumpet);
    - a PowerShell tool that edits the Windows registry (Scoop);
    - nvm, which is entirely a POSIX shell script;
    - Unix-only Rust and Go probes (languages it only summarises);
    - nanoGPT, which defaults to `cuda` on a Mac (`torch.compile` on Windows wasn't flagged either);
    - a Mac `.sh` invoice script, whose only note was "Nothing breaks on Windows".
  - The start scripts behind `make dev` weren't checked. A run script was only flagged when it was named `run.sh` and the README mentioned it.
  - An indented-code README was ignored where a fenced one wasn't. `source .venv/bin/activate` was never flagged.
  - Ollama called over HTTP wasn't detected, and neither were `/tmp`, `pbcopy` or GNU `sed -i`.
  - No shell-script finding came up in 20 repos.
- **Suggested fix:**
  - List every ecosystem without deep support (Rust, Go, C/C++, C#, PowerShell…) under "Couldn't check", with file counts. Never print "No problems found" when that code wasn't read.
  - Add native-Windows detection for old-style `.csproj`/`.vbproj` (`WinExe`, `TargetFrameworkVersion v4.*`, `[DllImport]` of Windows DLLs).
  - Flag C/C++ includes (`termios.h`, `sys/ioctl.h`, `unistd.h` vs `windows.h`), PowerShell registry use (`HKLM:`, `HKCU:`, `Microsoft.Win32`), and "the product is a shell script" for Windows.
  - Treat every `.sh` as needing a Windows equivalent, and follow Makefile/npm targets into the scripts they call.
  - Read indented README code blocks.
  - Add shell rules: GNU vs BSD flags for macOS, and "needs bash" for Windows.
  - Add GPU rules: `cuda` defaults without `is_available()`, `nccl`, and `torch.compile` on Windows.
  - Detect Ollama by port, `OLLAMA_HOST` or `ollama serve`.
- **Found by:** Scanner, Power user, Mac dev, Non-technical, Beginner.

### B2. Secrets: the promise covers more than the check, and keys in code get sent
- **Where:** Blocker report ("Secrets stay on this Mac"), Final report, Send files to the AI, CLI.
- **Screenshot:** [secrets and models](shots/persona-mac-dev-02-secrets-models.png).
- **What happens:**
  - These weren't flagged and would be sent: API keys typed into `app/keys.py`, an extensionless SSH private key (`config/deploy_key`), and an AWS-style key in `app.py`. The screen still says "Secrets stay on this Mac", and the log says "secret values were not read".
  - `.env.local` and `secrets.json` are listed as secrets but missing from "won't be sent". One key is listed twice.
  - The promise is worded five ways: "aren't sent automatically", "never sends secrets without asking", "Secrets stay on this Mac", "never copies secrets", "No secrets.".
- **Suggested fix:**
  - Look inside text files for key patterns (`sk-…`, `hf_…`, AWS, PEM headers) and hold those files back.
  - Use one scoped sentence everywhere, e.g. "Values in .env and key files aren't sent. Portal doesn't read keys written into code" (until it does).
  - Show one table of secret files: keys, "sent: no".
- **Found by:** Mac dev, Copy.

### B3. "Nothing leaves this Mac" while the scan contacts PyPI
- **Where:** Scanning (online check on). "Check packages online" button.
- **Screenshot:** [online scan](shots/copy-and-trust-02-online-scan-nothing-leaves.png).
- **What happens:** The screen says "nothing leaves this Mac", and the log right below it says "net online: pypi.org". The button doesn't say what it sends.
- **Suggested fix:** Make the line depend on the setting: "Package names are looked up on pypi.org; nothing else is sent". Use the same words on the button.
- **Found by:** Copy. Related: Non-technical (the privacy line only appears on a screen that flashes by).

### B4. "Proved" and the conversion headlines claim more than the screens show
- **Where:** Checking it runs, Send files to the AI, Final report, Consent declined, A change undone, Details log.
- **Screenshots:** [Proved vs fails](shots/copy-and-trust-04-send-to-ai-proved-vs-fails.png), [Runs on Windows](shots/copy-and-trust-05-final-runs-on-windows.png), [declined](shots/copy-and-trust-06-winpy-consent-declined.png), [Details log](shots/persona-power-user-08-details-log.png), [undone](shots/persona-mac-dev-07-undone.png).
- **What happens:**
  - "14 of 14 passed" is followed by "1 test fails on Windows". The log shows the AI's edit with no verify after it.
  - "Runs on Windows" sits above "Answers from the model: Couldn't check", and for a local-AI app the answers are the point.
  - "Installs and passes its checks" sits above a Failed check.
  - "Opens the workbook: Proved" appears after the export that makes the workbook was skipped.
  - No "Proved" row can be opened to see the command or output.
  - After Undo, the headline and counts don't change and nothing re-checks.
- **Suggested fix:**
  - Make the demo data consistent.
  - Put failed and couldn't-check items in the headline.
  - Say "Passed" and name what ran. Each row opens its command, exit code and output.
  - Re-verify after the AI's change and after Undo, and show "Checking again…".
  - Mark checks that depend on a skipped step as Couldn't check.
- **Found by:** Copy, Power user, Mac dev, Beginner.

### B5. The scan headline leaves out what couldn't be checked, including packages that won't install
- **Where:** Blocker report headline, CLI summary.
- **Screenshots:** [online](shots/persona-beginner-03-real-scan-online.png), [uvloop](shots/persona-beginner-04-uvloop-opened.png), [Couldn't check under the pitch](shots/copy-and-trust-03-report-couldnt-check-and-pitch.png).
- **What happens:**
  - "No blockers found. 5 things will need a fix" leaves out the Couldn't-check group. That group can hide `uvloop` ("may not build there at all"), `mlx` and `pyobjc`, which are Mac-only.
  - Package builds are only checked online, which is off by default.
  - The CLI adds the caveat only when it found no problems.
  - "mlx isn't on PyPI (a private or renamed package)" was wrong: mlx is on PyPI; that version wasn't.
- **Suggested fix:**
  - Always end the headline with "N couldn't be checked" (and "N failed").
  - Treat "no build for this OS" as a likely blocker.
  - Offer the package check before showing the verdict.
  - State what happened ("PyPI didn't return mlx 0.18.0") instead of guessing why.
- **Found by:** Beginner, Copy, Mac dev.

### B6. Pro is offered for projects it can't convert, and its promise is silent about blockers
- **Where:** Blocker report ("Pro converts it for you", "Convert with Pro").
- **Screenshots:** [gunicorn](shots/persona-power-user-01-gunicorn-report.png), [fork "fix"](shots/persona-power-user-02-gunicorn-fork-fix.png), [kilo](shots/scanner-quality-kilo-win.png).
- **What happens:**
  - gunicorn, which is Unix-only by design, has 26 `os.fork`/`setuid` calls filed as "Known problems with known fixes", with fix text like "Use getpass.getuser()". Then: "Pro converts it… checks that it installs, passes its tests and starts."
  - kilo (C/termios) and EarTrumpet (WPF) are offered Pro too.
  - For the Mac dev's app, the Pro box says "the 8 fixes above" and nothing about its 2 blockers. It promises tests for projects that have none.
- **Suggested fix:**
  - Treat fork/setuid/process-signal clusters as a rebuild-style verdict, like Xcode: "Won't run on Windows; use waitress/uvicorn there."
  - Never pitch Pro on a tier the scanner only summarised.
  - Say per blocker whether Pro handles it, and say "tries to install, test and start".
- **Found by:** Power user, Scanner, Mac dev.

### B7. Files the app needs are dropped as "rebuildable" or "clutter"
- **Where:** Blocker report ("… won't be sent").
- **Screenshots:** [Scoop](shots/scanner-quality-scoop-mac.png), [models/ as clutter](shots/persona-mac-dev-02-secrets-models.png).
- **What happens:**
  - Scoop's committed `supporting/validator/bin` DLLs, which its code loads, are labelled build output, "rebuilt from the project's own files", and left out. The tool would break.
  - A local-AI app's 2.15 GB `models/` folder and a user's `data/` (222 MB) are "ignored by the project or clutter", with no way to send them.
  - There's no list of what *will* be sent.
- **Suggested fix:**
  - Treat git-tracked files as source unless a build step that makes them is found.
  - Name why each folder is left out ("in .gitignore"), and offer "send it anyway" or add it to the hand-off.
  - Add an expandable "will be sent" list with sizes.
- **Found by:** Scanner, Mac dev, Copy.

### B8. On the PC there's no way to check a project you've received
- **Where:** Project tab (Windows window), report wording.
- **Screenshots:** [drop on Windows](shots/persona-beginner-01-drop-windows.png), [report says fix on MacBook Pro](shots/persona-beginner-02-real-scan-offline-details-open.png).
- **What happens:** The PC's drop zone only checks "what would break on macOS". When the beginner forced a Windows check, the report said to add keys and apply fixes "on MacBook Pro", and that a path "only exists on this computer".
- **Suggested fix:** Add a "Check for: this PC / the other computer" choice. Word "fixes happen on", "add them on" and "this computer" from the point of view of the computer the project will run on.
- **Found by:** Beginner.

### B9. Dead ends in the prototype: Files, the gear, "Send as files", and the Mode switch
- **Where:** Project tab, Blocker report, Mode switch.
- **Screenshots:** [entry](shots/persona-nontechnical-01-entry.png), [arrow key discards report](shots/accessibility-polish-11-mode-arrow-discards-report.png).
- **What happens:**
  - Clicking Files, the settings gear or "Send as files" does nothing in the lab, so the documents person could never send anything.
  - Pressing ← in the Mode switch (or clicking Files) throws the report away and shows the drop screen. → doesn't bring it back.
- **Suggested fix:**
  - Prototype the Files hand-off ("Send as files" → sending → arrived), or mark these "not in this demo".
  - Make the Mode switch a tab list that keeps each tab's state, and ask before discarding a report or a running conversion.
- **Found by:** Non-technical, A11y, Beginner.

### B10. Lab crash: Node scenario, "AI repair, attempt 2 of 3"
- **Where:** Conversion → AI repair, shop-web scenario.
- **What happens:** The page goes blank (`Cannot read properties of undefined (reading 'lines')`). I confirmed it: the scenario has no AI change to show. The same scenario's AI consent asks to "Send 0 files" for "about $0.05" ([shot](shots/copy-and-trust-09-node-send-0-files.png)).
- **Suggested fix:** Handle a conversion where the AI changed nothing: skip the AI stops, and show "No AI needed".
- **Found by:** Copy (confirmed).

### B11. Long paths are cut off and most affected files can't be listed
- **Where:** Blocker report (real scan with many findings), opened finding.
- **Screenshot:** [clipped paths](shots/accessibility-polish-01-long-paths-clipped.png).
- **What happens:** Every row is cut at a fixed width with no ellipsis, tooltip or wrap, so the file name never shows. "and 24 more" is plain text, not a control.
- **Suggested fix:** Truncate in the middle and keep the file name (`src/…/module_10.py:10`), with the full path on hover or focus and a copy action. Make "Show all 30" expand in place, and consider grouping by folder.
- **Found by:** A11y.

---

## Confuses

The person gets the wrong idea or can't tell what happened.

### C1. Path and string rules are mostly false alarms, and dominate the report
- **Where:** Blocker report findings.
- **Screenshots:** [express URL routes](shots/persona-power-user-03-express-url-paths.png), [cookiecutter](shots/scanner-quality-cookiecutter-win.png).
- **What happens:**
  - Precision by rule:

    | Rule | Correct |
    |---|---|
    | `absolute-path` | 0/12 |
    | `path-concat-slash` | 1/14 |
    | `path-concat-backslash` | 0/6 |
    | `system-path` | 0/4 |

  - These three are 54% of all listed findings: `open-encoding`, `path-concat-slash` and `absolute-path`.
  - What gets flagged: URL routes and API URLs, module names, display strings, `'\0'`, comments, CI files and vendored minified JS.
  - Applying the `path.join` fix mechanically to URL routes would break them.
  - "Only exists on this computer" is wrong for paths that exist on neither computer.
- **Suggested fix:**
  - Only flag strings that reach a file API (`open`, `Path`, `fs.*`, `os.path.*`). Skip URLs, comments, vendored and CI files.
  - Downgrade `/`-joins to Info: `/` works on Windows.
  - Reword to "a path from a specific machine".
  - Don't cut evidence at spaces (`C:\Program`). Match the fix text to the file's language.
- **Found by:** Scanner, Power user, Mac dev.

### C2. Tests, examples, docs and CI are reported like the app itself
- **Where:** Blocker report.
- **What happens:** 69% of findings that cite a file are in tests, examples, docs, benchmarks, CI, deployment configs or vendored files. All 50 of cookiecutter's encoding hits are in tests that already pass on Windows CI.
- **Suggested fix:** Tag each finding with a role (app, test, example, docs, CI, vendored). Collapse non-app ones into "Only affects tests and examples", outside the headline count.
- **Found by:** Scanner.

### C3. Platform guards are handled backwards
- **Where:** Blocker report.
- **What happens:**
  - Code that *is* guarded gets flagged: pywinauto's `linux/` and `macos/` backends, pexpect's guarded import, and a project's own package treated as a Windows-only third-party module.
  - Code that *isn't* guarded gets suppressed: `os.fork()`/`os.chown()` inside `try/except OSError` don't catch the Windows `AttributeError`.
- **Suggested fix:**
  - Resolve local packages first.
  - Honor platform folder names and import-site guards.
  - Count a `try` as a guard only if it catches `AttributeError`, `ImportError` or `Exception`.
  - Add `strftime('%-d')`.
- **Found by:** Scanner.

### C4. Large notebooks are skipped silently
- **Where:** Blocker report, CLI.
- **What happens:** Notebooks over 1 MB (usually because of embedded plots) aren't read. 15 of 34 in one book were skipped, and the report still says "No problems found". `!unzip` cells went unchecked.
- **Suggested fix:**
  - Strip outputs and parse the code cells whatever the file size.
  - List anything skipped under Couldn't check.
  - Add notebook shell-magic rules (`!unzip`, `!wget`).
- **Found by:** Scanner.

### C5. Wrong project type from incidental files
- **Where:** Tier chip and reason, "Needs Docker".
- **What happens:**
  - gunicorn is "a FastAPI server" (from a test fixture).
  - A notebook book is "a Python web server" (from a `python_version < '3.0'` marker).
  - fzf and nvm are "an app that runs in Docker" and "Needs Docker" (their Dockerfiles are for tests and development).
  - kilo is "Static: documents". EarTrumpet and Scoop are "Scripts".
  - The voting app's reason repeats "Redis, PostgreSQL" three times.
- **Suggested fix:**
  - Let Docker drive the tier only when it's the documented way to run.
  - Honor environment markers.
  - Treat C/C++ with a Makefile as native code.
  - Remove duplicate services from the tier reason.
- **Found by:** Scanner, Power user.

### C6. The secrets rule goes by file name
- **Where:** Blocker report ("holds credentials").
- **What happens:**
  - `.npmrc` files that contain only `save-exact=true` or `package-lock=false` are withheld as "credentials".
  - A public `cert.pem` is called a secret.
  - Test-fixture keys are held back, so the project's SSL tests fail on the other side.
- **Suggested fix:**
  - Look inside `.npmrc` (`_authToken`, `_auth`, `password`).
  - Tell certificates from private keys.
  - Offer "these are test keys, send them".
- **Found by:** Scanner, Power user.

### C7. A Mac app sent to a Mac is told it "needs a rebuild for macOS"
- **Where:** Rebuild verdict, hand-off note.
- **Screenshot:** [Rectangle](shots/scanner-quality-rectangle-mac.png).
- **What happens:** Rectangle scanned with `--target macos` gets "This needs a rebuild for macOS" and a hand-off suggesting .NET MAUI or Avalonia.
- **Suggested fix:** Give the native verdict only when the app's platform differs from the target. Otherwise say "This is already a macOS app".
- **Found by:** Scanner.

### C8. Price, plan and sign-in never appear where you decide
- **Where:** Project tab, Blocker report, "Convert with Pro", Plans, Account.
- **Screenshots:** [Paid badge](shots/copy-and-trust-01-drop-paid-badge.png), [Convert jumps](shots/persona-nontechnical-05-convert-with-pro.png), [machine switch](shots/persona-mac-dev-04-convert-jump.png), [Plans](shots/copy-and-trust-08-plans.png), [Account](shots/persona-beginner-10-account.png).
- **What happens:**
  - A "Paid" badge sits on the tab whose first action is the free scan.
  - "Convert with Pro" goes straight to a "Run it?" stop on the other computer: no price, sign-in, plan or summary. Details already reads "apply 6 changes".
  - Prices are only on Plans, which nothing in the window leads to.
  - All plans are monthly, though some people convert once.
  - "$20 of AI use" inside a $12 plan has no unit people can relate to.
  - The account is Sam's and "covers both computers", so a friend receiving a project can't tell who pays.
- **Suggested fix:**
  - Move "Paid" or "Pro" onto the Pro box only.
  - Put a confirm step before converting: what changes, on which computer, which plan, the price.
  - Show the estimated cost of *this* conversion on the button.
  - Consider pay-per-conversion.
  - Translate "AI use" into "about N conversions like this".
  - Say whose plan pays on each computer.
- **Found by:** Beginner, Non-technical, Mac dev, Power user, Copy.

### C9. "The AI" is never named, and where your code goes isn't said
- **Where:** Send files to the AI, Plans, Own key, Migration plan.
- **Screenshots:** [AI consent](shots/persona-nontechnical-07-ai-consent.png), [Send to AI](shots/persona-power-user-05-send-to-ai.png), [Own key](shots/persona-power-user-11-own-key.png).
- **What happens:**
  - No screen names the AI provider or model, or says whether files are kept or trained on.
  - The own key is kept "in the Mac's keychain", but conversions run on the PC.
  - The migration-plan offer doesn't say the code is sent.
  - Nothing says how files travel to the PC (directly or through the internet).
- **Suggested fix:**
  - Add one line, with a link, naming the provider, model and retention, on the AI consent screen and on Plans.
  - Say which machine calls the AI and whether the key crosses the link.
  - Say "goes straight to Desk PC over your network".
- **Found by:** Power user, Mac dev, Non-technical, Copy.

### C10. Counts and names drift between screens
- **Where:** Report → Planning → Checking → Final. Exports. Account.
- **What happens:**
  - Change counts drift: "5 fixes" → "Matching 6 findings" → "6 changes" → "7 changes" (Node: 2 → 5 → 4).
  - Some scans show "1 of 0".
  - The exports are called "hand-off note", "hand-off plan" and "migration plan"; "See the rebuild plan" opens a verdict whose button says "migration plan".
  - The AI spending is called "allowance", "budget", "AI use" and "AI usage".
  - "Hand-off note saved" doesn't say where it went.
- **Suggested fix:**
  - Explain each step: "5 problems → 6 changes (1 fix touches 2 files) + 1 by the AI".
  - Use one name for each thing.
  - Hide counts when there's nothing to count.
  - Show the save location with a Reveal button.
- **Found by:** Copy, Beginner, Mac dev, Power user.

### C11. Which computer? Names and point of view switch
- **Where:** Window header, Scanning, Conversion, Final report, consent stops.
- **Screenshots:** [header](shots/persona-beginner-01-drop-windows.png), [machine switch](shots/persona-mac-dev-04-convert-jump.png).
- **What happens:**
  - The big header shows the *other* computer's name ("MacBook Pro" on the PC), and people thought Portal had their machine wrong.
  - "Checking for Windows-only code" is backwards for a Mac project.
  - The conversion moves to the other computer's window without saying so, and the person can't tell who approves there.
  - "Same as on MacBook Pro" implies tests ran on the Mac.
- **Suggested fix:**
  - Label the header "Connected to MacBook Pro".
  - Say "Looking for code that won't run on Windows".
  - Add a one-line "Now on Desk PC: approve here" before PC-side stops.
  - Always name the machine where something happens.
- **Found by:** Beginner, Copy, Mac dev, Power user.

### C12. Fixes and words are for programmers only
- **Where:** Opened findings, Needs you, Scanning, Details, Plans.
- **Screenshots:** [uvloop](shots/persona-beginner-04-uvloop-opened.png), [Python fix](shots/persona-nontechnical-04-python-fix.png).
- **What happens:**
  - Fixes are code instructions (`__main__` guard, `Path(__file__)`). A Python file even got Node advice.
  - Jargon is everywhere: ecosystems, parsed, wheels, lockfile, `.venv`, Static/Tier, UTF-8, chmod, Snapshot/Pack, API key, keychain.
  - The full list with plain alternatives is in [copy-and-trust.md](copy-and-trust.md#jargon-a-non-developer-wont-follow).
- **Suggested fix:**
  - Lead each finding with one plain line ("What this means for you") and tuck the code under "For whoever maintains it".
  - Replace the listed terms.
  - Match the fix text to the file's language.
- **Found by:** Beginner, Non-technical, Copy.

### C13. "Needs you" items say what, but not how
- **Where:** Needs you, Final report.
- **Screenshots:** [needs you](shots/persona-beginner-05-needs-you.png), [final](shots/persona-beginner-08-final-answer.png).
- **What happens:** There's no link to Ollama, no hint where API keys come from or what they cost, "run ollama pull" with no place to run it, and no "Start the app" on the final report.
- **Suggested fix:**
  - Add a "Get Ollama" link and "Ask the person who sent this for these keys".
  - Show copyable commands that say where to type them.
  - Add a Start the app button.
- **Found by:** Beginner, Mac dev.

### C14. Documents and mixed folders: the report ignores what people care about
- **Where:** Blocker report (documents and a script).
- **Screenshots:** [shell scan](shots/persona-nontechnical-02-scan-shell.png), [documents](shots/persona-nontechnical-09-documents-report.png).
- **What happens:**
  - "No blockers found. 2 things will need a fix" is a mixed verdict.
  - Documents and photos aren't mentioned.
  - "0 B won't be sent" reads like something was left out.
  - HEIC, .pages and .numbers files (no built-in Windows app for the latter two) are unflagged, and "It arrives clean" reads as "everything will open".
  - "AI use: None" shows on a documents folder.
- **Suggested fix:**
  - Give the files a separate verdict ("11 documents and 3 photos; .HEIC needs an extension on Windows").
  - Hide zero-byte and AI lines when they're empty.
  - Say what isn't checked (whether Windows has apps for these files).
- **Found by:** Non-technical, Copy.

### C15. The change review doesn't lead anywhere for developers
- **Where:** Final report, Reviewing changes.
- **Screenshots:** [review](shots/persona-power-user-09-review-changes.png), [undone](shots/persona-mac-dev-07-undone.png).
- **What happens:**
  - Changes stay in a copy on the PC: no git branch, commit or patch, so the Mac repo and the PC copy diverge.
  - The CLI can't write a hand-off note (`handoff: null` for most projects).
  - Added lines are tinted orange, and were read as deletions.
- **Suggested fix:**
  - Offer "Commit on branch portal/windows" or "Export as patch".
  - Add `portal-scan --handoff out.md` and always fill the note.
  - Color additions so they can't be read as deletions.
- **Found by:** Power user, Mac dev.

### C16. Keyboard and screen-reader users lose their place
- **Where:** Every screen change; Scanning; Conversion; consent stops.
- **Screenshots:** [consent stop](shots/accessibility-polish-03-consent-stop-no-focus.png), [scan steps](shots/accessibility-polish-07-scan-steps-state.png).
- **What happens:**
  - Focus falls to the page on all 14 screen changes tested, including the consent stops that wait for an answer. Nothing announces the new screen.
  - The scan list announces 30 count updates in about 3 s but never which step is done. Conversion progress, "saved" and Undo are silent.
- **Suggested fix:**
  - Move focus to each new screen's heading (or the question, at consent stops).
  - Use one throttled status region per screen with meaningful messages.
  - Give scan steps hidden state text, as the stepper already has.
- **Found by:** A11y.

### C17. Other things that break with real data or keyboard use
- **Where:** Blocker report, Conversion, Final report.
- **Screenshots:** [long file name](shots/accessibility-polish-02-long-filename-overflow.png), [no focus ring](shots/accessibility-polish-04-codeblock-no-focus-ring.png), [undone row](shots/accessibility-polish-06-undone-row-contrast.png).
- **What happens:**
  - A 90-character file name squeezes the finding title to one word per line and pushes it past the window edge.
  - Diff blocks and the Planning pane take Tab focus with no visible ring.
  - An undone change fades to 1.8–2.6:1 contrast while it's still being reviewed.
- **Suggested fix:**
  - Cap the meta column (`max-width:40%`, `min-width:0`) and middle-truncate.
  - Use an inset outline on scroll containers.
  - Show "Undone" with a badge at full contrast.
- **Found by:** A11y.

### C18. Package knowledge is wheel-only, and Docker image checks guess
- **Where:** Couldn't check / Will need a fix (packages, Docker).
- **What happens:**
  - POSIX-only pure-Python packages (`ptyprocess`, `gunicorn`) pass even online.
  - Pure-Python source packages get "couldn't tell if it needs a compiler".
  - Environment markers are ignored.
  - All 6 Docker images flagged for arm64 actually ship arm64, and one Dockerfile already uses `BUILDPLATFORM`.
- **Suggested fix:**
  - Keep a small list of OS-only packages, and use PyPI's `Operating System ::` classifiers when online.
  - Treat pure-Python sdists as fine and evaluate markers.
  - Check image manifests when online, and recognise `--platform=$BUILDPLATFORM`.
- **Found by:** Scanner.

### C19. The "Run a command?" stop undersells the risk
- **Where:** Consent: Run a command?
- **Screenshot:** [run a command](shots/persona-beginner-06-run-a-command.png).
- **What happens:** "This project wants to run…" puts the request on the project. "As you" hides what that means. The risky choice is the primary button. The skip text is reused for an export, which isn't "starting".
- **Suggested fix:**
  - "Start the app to check it works? Portal would run: …"
  - "It can do anything you can do on this PC; run it only if you trust this project."
  - Give both buttons equal weight.
- **Found by:** Beginner, Copy.

### C20. One explanation covers several different problems
- **Where:** Opened findings with several places.
- **What happens:**
  - "\`open\` isn't a Windows command" also covers `say`.
  - "AUX is reserved" also covers a colon and a trailing dot.
  - `rm -rf` and quoting only get the `VAR=value` reason.
  - A file is called a folder.
  - "Mac line endings" means LF.
- **Suggested fix:** Give one reason per place, and check the file/folder wording.
- **Found by:** Copy.

---

## Annoys

The person understands, but it grates or costs time.

- **A1. The upgrade is pitched too often.** "A Pro conversion does this for you" repeats under every finding, and the Pro box shows even when there's nothing to fix. *Fix:* pitch once, with the price, and only when there are fixes. [shot](shots/copy-and-trust-03-report-couldnt-check-and-pitch.png). *Copy.*
- **A2. "Used up" offers only paying.** "Allowance used up" shows "Use my own key" or "Allow $10 more", with no wait option and no close. "Allow" is a soft word for a charge. *Fix:* add "Wait until 1 November", say "Add up to $10 (charged as used)", and state the top-up terms on Plans. [shot](shots/copy-and-trust-07-allowance-used-up.png). *Copy.*
- **A3. Contrast below AA.** Measured: tertiary 11–12 px text 2.58–2.95:1 (light), the Details peek line 4.1:1, pending step circles 1.33:1, the light-theme focus ring 2.76:1. *Fix:* use the muted token for these labels, give the peek a solid backing, put a ≥3:1 border on pending circles, and draw the focus ring at full opacity. [shot](shots/accessibility-polish-05-tertiary-text-contrast.png), [shot](shots/accessibility-polish-14-details-peek-contrast.png). *A11y.*
- **A4. The Details drawer and log don't follow the screen.** Once opened, the drawer stays open across screens and squeezes the report to one row. Edge states show the previous project's log ("This folder is empty" above "14 files, 6 parsed"). *Fix:* collapse it on screen change, and use each screen's own log. [shot](shots/persona-beginner-02-real-scan-offline-details-open.png). *Beginner, Copy.*
- **A5. Short windows hide the action bar.** At 760×600 or 1440×560, Convert, Run it and Send are below the fold. *Fix:* set a minimum window height and pin the action bar. [shot](shots/accessibility-polish-10-short-window-760x600.png). *A11y.*
- **A6. The account sheet:** the close button is named "Clear", Esc doesn't close it, focus is lost on close, and there's no h2. *Fix:* name it Close, close on Esc, return focus to Settings. [shot](shots/accessibility-polish-12-account-sheet.png). *A11y.*
- **A7. The AI cost line is hidden.** On "Send files to the AI?" the cost line is clipped under the buttons, and fully hidden when Details is open. *Fix:* put the price on the button: "Send 4 files · about $0.05". [shot](shots/persona-beginner-07-send-to-ai.png). *Beginner, Copy.*
- **A8. Unreadable files and offline state are easy to miss.** The headline ignores both, and "Check their permissions" doesn't say how. *Fix:* put both in the headline, give Finder steps, and show an offline banner with "Check online". *Copy.*
- **A9. The narrow stepper.** At 380 px step names are cut ("Sna…", "Rep…"). Step summaries are mouse-only tooltips. One tooltip shows an unfilled placeholder: "…stays on {there}" (confirmed). *Fix:* show "Step 6 of 7: Repair" at narrow widths, make the summaries visible text, and fill the placeholder. [shot](shots/accessibility-polish-08-stepper-truncated-380.png). *A11y.*
- **A10. Wheels-unchecked noise.** "Package builds weren't checked" is appended to every offline Python headline. *Fix:* move it to the footer, or ask once before the scan. *Scanner.*

---

## Polish

- **P1.** The elapsed time wraps ("1.1 / s") next to a full-width Stop at 520 px. [shot](shots/accessibility-polish-09-elapsed-wraps-520.png). *A11y.*
- **P2.** A focused finding row near the bottom is hidden under the action bar and its fade. [shot](shots/accessibility-polish-13-row-focus-under-action-bar.png). *A11y.*
- **P3.** Semantics:
  - Eight visual subheadings are `<p>`.
  - Seven buttons are named only "Undo".
  - "Show all" has no expanded state.
  - "Email me a link" and "Save key" are disabled without a reason.
  
  *A11y.*
- **P4.** Tone: "It's a big one", "doesn't pretend to convert it", "clutter", "You chose not to…", and "Nothing is claimed" three times on one screen. *Copy.*
- **P5.** The CLI summary:
  - "Tier:" clashes with the paid tier.
  - "Expected AI use" with no mention of Pro.
  - "parsed".
  - "How it's run here" lists build scripts.
  - One error message for both "missing" and "no permission".
  
  *Copy.*
- **P6.** Small copy:
  - The double negative "doesn't need a rebuild / isn't an app for one OS".
  - "DerivedData is rebuilt on Desk PC" for an app that can't build there.
  - "Work through the items above" when there are none.
  - `service` (Needs you) vs `service-client` (Info), and "Needs Redis" when compose runs it.
  
  *Copy, Scanner.*
- **P7.** Long diff lines: it isn't obvious that they scroll sideways. *Power user.*
- **P8.** The lab panel (prototype tool, not product):
  - The reduced-motion switch and hidden-panel state reset on reload.
  - The panel stacks above the window below 760 px.
  - "Real scan" entries are disabled with no hint why.
  - Forcing the other window style mixes machine wording.
  
  *A11y, Copy.*

---

## What the reviewers said works (keep)

- The change review: real diffs, Undo on every change, "By the AI" marks, and the repair log that shows attempt 1 undone with a reason (Mac dev, Power user, Beginner).
- The AI consent screen: named files, size, what's never sent, and a cost (Power user).
- The Rebuild verdicts for Xcode and WPF: "doesn't pretend to convert", what carries over, a concrete toolkit (Power user, Scanner).
- "Couldn't check / nothing is claimed" as a state (Power user, Copy).
- File:line evidence on every finding (Power user, Mac dev).
- The specific rules: POSIX modules, signals, npm shell syntax, make, symlinks, Xcode, all 100% in the sample (Scanner).
- Scan speed: median 0.11 s, max 0.61 s on 20 projects. `--online` adds up to 6.6 s on the first lookup (Scanner).
- The final report's shape, "Runs on Windows. 2 things need you." with Proved / Needs you / Couldn't check, once its claims match the evidence (Beginner).
- Accessibility basics: native named controls, Tab order matches the visual order in both window styles, `aria-expanded` on expandable items, reduced motion respected, no sideways scroll down to 320 px (A11y).
- The power user's verdict: wouldn't pay today. Own key at $5 if forced; $3–5 per conversion with inspectable proof, a git branch or patch, and a named AI provider.
