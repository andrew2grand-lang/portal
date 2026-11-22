# Copy and trust review: Project Send

## Overview

**What I read.** I went through all 35 states in the lab list for each of the 6 demo scenarios. I read each one in three window modes: the macOS frame, the Windows frame and Auto. I also opened every finding, "Show all" list and Details log one at a time. I ran the lab's "Scan a real folder" tool 6 times on test folders I made: a Python app, the same app with the online check on, a folder with secrets and Docker, a Windows script checked for macOS, an empty folder, and a path that doesn't exist. I ran `portal-scan` on the same folders too (summary, `--online`, `--target macos`, `--progress`, `--json` and error cases).

**Overall.** Most of the writing is good: short, concrete, it names files and line numbers, and it says "Nothing is claimed" when something wasn't checked. The consent stops give a time limit, a cost and a list of what is never sent. The trust problems come from a few confident words that claim more than the screens show:

1. **What leaves the computer.** While it is contacting pypi.org, the scan screen says "nothing leaves this Mac". The secrets promises cover more than Portal actually checks, and the screens never name the AI that gets your files.
2. **What was proved.** "Proved", "passes its checks" and "Runs on Windows" appear next to Failed and Couldn't-check results. The test count goes from "14 of 14 passed" to "1 test fails" on the very next screen.
3. **Headlines leave out what wasn't checked.** "No blockers found" appears even for an app built on MLX, which only runs on Apple computers.
4. **Counts change from screen to screen** (5 → 6 → 7 changes; 2 → 5 → 4).
5. **The upgrade is pitched under every finding**, and also on reports where there's nothing for Pro to do. The "used up" screens offer no option except paying or bringing your own key.

**Severity used below.**
- **Blocks use:** the user can't continue, or would make a wrong decision with real cost (privacy, money, a project that doesn't work).
- **Confuses:** the user gets the wrong idea or can't tell what happened.
- **Annoys:** the user understands, but the wording grates.
- **Polish:** small fixes.

**Lab notes.** These are not product copy, but they could mislead a reviewer:
- In the forced macOS and Windows frames, "this Mac", "this PC" and "Desk PC" follow the frame, while the OS follows the scenario. So a PC → Mac scenario in the Mac frame reads "Converting ledger-tool for macOS … The 5 fixes above happen on Desk PC". I judged machine wording in Auto.
- In the Node scenario, "AI repair, attempt 2 of 3" blanks the whole page (`Cannot read properties of undefined (reading 'lines')`), so I couldn't review that state.
- The edge states keep the previous project's Details log. For example, "This folder is empty" sits above "14 files, 6 parsed".

---

## Screen by screen

### 1. Project tab: Drop a project, Dragging a folder over

| Quote | Problem | Suggested rewrite |
|---|---|---|
| Tab: "Project **Paid**" | The badge marks the whole tab as paid, but the first thing in it is a free scan. It puts people off trying the free part. | Remove the badge, or put "Pro" next to "Pro conversion" only. |
| "Pro conversion: Portal fixes it on Desk PC, checks it runs, and shows every change." | "checks it runs" reads as a promise. Later screens show runs that fail or can't be checked. | "Portal makes the fixes on Desk PC, tries to install and start it there, and shows every change." |
| "Drop to check it" | Polish only: "it" works while dragging, but the folder's name is friendlier. | "Drop to check local-chat" |

### 2. Scanning (live demo, real scan, huge folder)

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "Reading it here on this Mac. Nothing is run or changed, and nothing leaves this Mac." (with the online check on) | False. The Details log on the same screen says "net online: pypi.org". | Online: "Reading it here on this Mac. Nothing is run or changed. Package names are looked up on pypi.org; nothing else is sent." |
| "Finding ecosystems" | Jargon. | "Finding languages and tools" |
| "Checking for Windows-only code" (Mac → Windows) / "Checking for macOS-only code" (PC → Mac) | Backwards. A Mac project checked for Windows needs a search for Mac-only code. | "Looking for code that won't run on Windows" |
| "1 of 0" then "0 files checked" (Xcode, WPF, Documents) | Impossible count. | Hide the count when there is nothing to check: "No code to check". |
| "It's a big one: 31.4 GB. Almost all of it is node_modules and .venv folders, which aren't sent, so only the project's own 6,377 code files are checked." | Chatty opener, a 31-word sentence, and two folder names a non-developer won't know. | "31.4 GB, mostly downloaded packages that aren't sent. Checking the project's own 6,377 code files." |
| Details: "**safe** nothing was run; secret values were not read" | "safe" is a verdict, not an event. | Label it "run" or "privacy": "Nothing was run. Secret values weren't read." |
| Details: "recv", "net", "parsed" | Abbreviations and parser talk. | "received", "internet", "code read" |

### 3. Blocker report (The answer, A finding opened, Hand-off note exported)

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "No blockers found. 5 things will need a fix and 2 need you." | Leaves out the "Couldn't check: 1" group below it. | "No blockers found. 5 things need a fix, 2 need you, 1 couldn't be checked." |
| Category chips "App", "Static", "Native app", "System", "Scripts" | Taxonomy labels with no explanation. | "What it is: a web app that uses a local AI model" |
| "These stop it from running there until something changes." | "there" is vague. | "These stop it running on the Mac until they're changed." |
| "Not tested here, so nothing is claimed." | Good idea. "Tested" clashes with "checked" everywhere else. | "Portal couldn't check these, so it makes no claim about them." |
| "Uses a path that only exists on this computer" / "…/config.json is a folder on the computer this was written on" | The title says "this computer" but the body says "the computer this was written on". Also, a file is called a folder. | "Uses a path from the computer it was written on" / "`…/config.json` won't exist on Windows." |
| "File name Windows can't use (3 places). AUX is a reserved name on Windows." | The reason only covers 1 of 3 (the others are a colon and a trailing dot). | Give a reason per line: "aux.txt: AUX is reserved · report:2024.txt: ':' isn't allowed · 'trailing. ': names can't end in a dot or space". |
| "Runs a command Windows doesn't have (2 places). \`open\` isn't a Windows command" (the 2nd place is `say`) | Same problem: the reason only covers the first place. | "\`open\` and \`say\` aren't Windows commands." |
| "npm script uses Unix shell features (3 places). …cmd.exe, which doesn't understand \`VAR=value\`… Fix: Use cross-env: \`cross-env NODE_ENV=production node server.js\`" | The reason covers 1 of 3 (`rm -rf` and single quotes aren't explained). The example isn't the user's line. | One reason and one fix per line, using the user's own script. |
| "Rename it. Portal renames such files when they arrive and lists each change…" then "A Pro conversion does this for you on Windows." | Contradicts itself: renaming happens on arrival, yet it's also sold as Pro. | "Portal renames these when they arrive and lists each change. Code that still uses the old names needs updating; Pro does that part." |
| "A Pro conversion does this for you on Windows." (under every finding) | Repeated upsell. | Say it once, in the Pro box. |
| "Check packages online" | Doesn't say what gets sent. | "Look up 8 package names on PyPI" + "Only names and versions are sent." |
| "1.85 GB won't be sent … The rest is ignored by the project or clutter." The list includes "data/ 222 MB". | The user's own data folder is called clutter. "Ignored by the project" means .gitignore, which most users won't know. | "data/ (222 MB) is listed in .gitignore, so it isn't sent. Send it with Files if you need it there." |
| "DerivedData is rebuilt on Desk PC…" (Xcode) / "InvoiceDesk/bin and InvoiceDesk/obj are rebuilt…" (WPF) | Not true for an app the same screen says can't build there. | "DerivedData is build output; the other computer doesn't need it." |
| "Secrets stay on this Mac. .env holds 3 keys… Portal only read their names, never the values." | The headline is broader than the check. Keys written into code aren't detected (see Findings). | "Values in .env stay on this Mac. Portal doesn't look for keys written into code." |
| "Secrets aren't sent automatically… Portal never sends secrets without asking." | A third version of the same promise. | Use one sentence everywhere (see Findings). |
| "and 1 more" (Needs Docker) | Says nothing about what the other item is, and it can't be expanded. | List it: "and docker-compose.yml". |
| "AI use for a conversion (estimate): None. Nothing to change, so no AI should be needed." (folder with 3 Needs-you items) | "Nothing to change" contradicts the list right above it. | "None: what's left is for you to do." |
| "Pro converts it for you. The changes happen on Desk PC, then Portal checks that it installs, passes its tests and starts." | Shown even when there's nothing for Pro to fix. It promises tests for projects that have none, and gives no price. | "With Pro ($12 a month), Portal makes these 5 fixes on Desk PC and tries to install, test and start it there. You review every change." Hide this box when there are 0 fixes. |
| "Export hand-off note" / "Hand-off note saved" | Who is it handed to? Where was it saved? | "Save the fix list" / "Saved to Downloads › local-chat fixes.md · Show" |

### 4. Consent stop: Run a command?

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "This project wants to run uvicorn app.main:app --port 8000. Run it?" | The project doesn't want anything; Portal is asking. The command comes before the reason. | "Start the app to check it works? Portal would run: `uvicorn app.main:app --port 8000`" |
| "It runs on this PC as you, in the project folder, for up to 2 minutes, then Portal stops it." | "as you" hides the real risk. | "It can do anything you can do on this PC, so run it only if you trust this project. Portal stops it after 2 minutes." |
| "If you skip it, starting it is marked Couldn't check." | Awkward. The ledger scenario reuses it for an export, which isn't "starting". | "If you skip, the report says 'Couldn't check: [the export]'." |

### 5. Consent stop: Send files to the AI?

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "Here's exactly what's sent. No secrets." | Claims more than the check covers (see Findings). | "These 4 files are sent. Never sent: .env, key files, and anything in .gitignore." |
| (nothing) | The screen never says which company's AI gets the files, whether it keeps them, or whether it trains on them. | "Sent to [provider] to write this fix. [Provider] doesn't keep them or train on them. Details" |
| "About $0.05 of AI usage. You've used $3.40 of $20.00 this month." | Half hidden and faded behind the pinned "Send 4 files" button, and fully hidden when Details is open. | Put the price on the button: "Send 4 files · about $0.05" |
| Node: "Send 0 files to the AI to fix this?" / "Send 0 files" / "About $0.05" | Asks for consent and money for nothing. | Don't show this stop when nothing would be sent. |
| Log under this screen: "run the tests: proved" | The screen above says "1 test fails on Windows". | "run the tests: 13 of 14 passed" |

### 6. Conversion (Planning, Applying changes, Checking it runs, AI repair)

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "Snapshot · Pack · Plan · Apply · Verify · Report · Repair: only if a check fails" | Internal pipeline names. | "Copy · Plan · Change · Check · Report · Fix with AI (only if a check fails)" |
| "Matching 6 findings to fix patterns" (the report said 5); Node: "Matching 5 findings" (the report said 2, and 4 are listed) | The numbers don't match. | "Planning 6 changes for the 5 problems found" |
| "Known fix" / "For the AI" | Jargon tags. | "Standard fix" / "Needs the AI" |
| "Run the tests: 14 of 14 passed, same as on MacBook Pro" → next state: "1 test failed on Windows" | Contradiction. | Show the real count: "13 of 14 passed · 1 failed: test_ingest" |
| "The test passed, but the index on Desk PC uses / names: the old index would not match. Undone." | Hard to follow. | "The test passed, but your existing index would stop matching, so Portal undid this." |
| "Mac line endings" | Imprecise: it means Unix (LF) line endings. | "Unix line endings (LF)" |
| "Running…" | Says nothing about what is running or for how long. | "Starting the app (up to 2 min)" |

### 7. Final report (The answer, Reviewing changes, A change undone, Hand-off plan exported)

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "Runs on Windows. 2 things need you." (above "Answers from the model: Couldn't check") | The headline claims more than was checked. The model's answers are the whole point of this app. | "Installs, passes its tests and starts on Windows. Not checked: answers from the model. 2 things need you." |
| "Runs on the Mac. 1 thing needs you." (above "Downloading exchange rates: Failed") | A failed check doesn't appear in the headline. | "Runs on the Mac, except downloading exchange rates (that fails on Windows too)." |
| "Proved" (status tag) | Too strong for "ran once with sample data". | "Passed", plus what was run ("with 2 sample PDFs") |
| "Tests pass · Proved · 14 of 14, the same as on MacBook Pro." | The scan said nothing was run on the Mac. It also hides that the AI changed code to get there. | "14 of 14 passed on Desk PC (1 needed the AI's fix)." |
| "7 changes, each one undoable." | The report said 5, the planning screen said 6. | "7 changes: 6 standard, 1 by the AI." |
| "Undone. Build the index path with pathlib" (the headline still says "Runs on Windows") | There's no sign the checks ran again, so the headline is out of date. | "Undone. Checking again…" then update the headline. |
| "Portal never copies secrets; .env.example lists what's needed." | A fourth version of the secrets promise. | Use the single sentence from the Findings. |
| "Export hand-off plan" (the report used "hand-off note") | Two names. | "Save the change list" |

### 8. Rebuild verdict, migration plan, "needs a rebuild" stop

| Quote | Problem | Suggested rewrite |
|---|---|---|
| Report button "See the rebuild plan" → opens a verdict whose button is "See the migration plan" | The label doesn't match what opens. | "Why it needs a rebuild" → "See the rebuild plan" |
| "It's an app for one OS, so Portal doesn't pretend to convert it." | Defensive tone. | "Portal can't convert an app built for one OS. The rebuild plan lists what moving it takes." |
| "local-chat doesn't need a rebuild. It isn't an app for one OS." | Double negative, and "rebuild" isn't explained. | "local-chat can be converted; it doesn't need rebuilding." |
| Verdict details ("Export.swift makes PNGs with AppKit (NSBitmapImageRep)") while Details says "7 files, 0 parsed" | It's unclear how Portal knows this. | Count the files it read, or say "from file names and project settings". |
| "Retarget to net8.0" | Jargon. | "Switch the project from the Windows-only .NET target to plain .NET 8" |
| "With Pro, the AI can write this plan out in detail for your code. It doesn't touch the project." | Doesn't say the code is sent to the AI. | "With Pro, Portal can send your code to the AI (it asks first) to write a detailed plan. Nothing in the project changes." |

### 9. Accounts and plans

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "Sign in to convert projects. Scans work without an account…" | Good, but the price is still nowhere on this path. | Add: "Pro is $12 a month; you choose a plan after signing in." |
| "sam@example.com · Pro, renews 1 November" | No price. | "Pro · $12 a month · renews 1 November" |
| "Conversions, with up to $20 of AI use a month included. Past that, you choose whether to allow more." | Doesn't say what "more" costs. | "Past that, Portal asks before adding more, $10 at a time, charged only for what's used." |
| "Choose" (Own key) | Doesn't say what it does. | "Switch to Own key" |
| "Conversions using your own AI key." / "API key" | "AI key" and "API key" are jargon, and the provider isn't named. | "Use your own [provider] API key (a code from your [provider] account)" |
| "You've been given $5 of AI usage" | Given by whom? Does it ever expire? | "Portal added $5 of AI use to your account. It's used first and doesn't expire." |
| "You've used this month's AI allowance" + "Use my own key" / "Allow $10 more" | No way to wait. "Allow" hides that it's a charge. | Add "Wait until 1 November". Rename to "Add up to $10 (charged as used)". |
| "allowance" / "AI budget" / "AI use" / "AI usage" | Four names for one thing. | Pick "AI allowance". |

### 10. Edge states

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "3 files couldn't be read. This computer won't let Portal open them… Check their permissions, then scan again." (headline still "No blockers found…") | Jargon, no how-to, and the headline ignores it. | Headline: "…3 files couldn't be read." Body: "Portal isn't allowed to open these. In Finder, choose File › Get Info › Sharing & Permissions, then scan again." |
| "Offline: couldn't check" (identical to the normal report) | Nothing tells the user they were offline. | Banner: "Offline: package builds weren't checked. Check online" |
| "Finish without it" | "it" is unclear. | "Finish with 1 check failing" |
| "Installs and passes its checks on macOS, but it wasn't started." (with "Downloading exchange rates: Failed") | "passes its checks" while one failed. It also wasn't "started": the skipped step was an export. | "Installs on the Mac. 1 check failed, and the export wasn't run because you skipped it." |
| "Opens the workbook · Proved" (with "Exports a month: Couldn't check") | Impossible: nothing made the workbook. | Mark checks that depend on a skipped one as Couldn't check too. |
| "You chose not to run …, so this wasn't tried." | Slightly blaming. | "Skipped at your request, so this wasn't tried." |
| "AI use for this month is used up… 1 check still fails" (shown in the Xcode, WPF and Documents scenarios) | Lab: makes no sense for a rebuild or a documents folder. | Hide it in those scenarios. |

### 11. `portal-scan` summary (CLI)

| Quote | Problem | Suggested rewrite |
|---|---|---|
| "Tier: Apps: MLX models" | "Tier" clashes with the paid tier. | "Kind: app (MLX models)" |
| "Expected AI use (estimate): Sometimes." | In a free scan, with no context. The app says "for a conversion". | "If converted with Pro, AI use (estimate): sometimes" |
| "No blockers found. 5 things will need a fix." (6 packages not checked; requirements include mlx and pyobjc-framework-Cocoa) | Leaves out what wasn't checked, and both packages are Mac-only. | "No blockers found in the code. 5 things need a fix. 6 packages weren't checked (offline)." |
| "mlx isn't on PyPI. mlx wasn't found on PyPI (a private or renamed package)" | mlx *is* published on PyPI. The guessed reason is stated as fact. | "PyPI didn't return mlx 0.18.0, so its Windows builds weren't checked." |
| "uvloop has no ready-made build… Fix: Use a version that ships Windows wheels" | Two terms for one idea. | Use "ready-made build" in both. |
| "Scanned 8 files (1 parsed) in 0.1 s." | "parsed" | "Read 8 files; checked the code in 1." |
| "How it's run here: \`rm -rf dist && cp -r src dist\`, \`chmod +x bin/cli\`, \`NODE_ENV=production node server.js\`" | Lists the build and postinstall scripts as "how it's run". | "Scripts: build …, postinstall …, start …" |
| "Add --online to check package builds on PyPI (nothing else is sent)." | Doesn't say what *is* sent. | "…sends package names and versions to pypi.org, nothing else." |
| "Needs Docker… (it runs on WSL 2)" / "It uses dependency redis" | Jargon, and an awkward phrase. | "…Docker Desktop (it installs a Linux layer on Windows)" / "It depends on the redis package" |
| "portal-scan: nope isn't a folder that can be read" | Treats "doesn't exist" and "no permission" as one error. | "No folder at nope" / "Can't open nope: no permission" |

### Jargon a non-developer won't follow

| Term | Where | Plain alternative |
|---|---|---|
| ecosystems | Scanning | languages and tools |
| parsed | Details, CLI | code read |
| Static / Native app / Tier | Report chip, CLI | "What it is:" in plain words |
| PyPI, ready-made build, wheels | Couldn't check, CLI | the Python package library; "ready-made version for Windows" |
| lockfile | Info, CLI | "a file that pins exact package versions" |
| .venv, node_modules, __pycache__, DerivedData, bin/obj | Won't be sent | "downloaded packages and build files (rebuilt automatically)" |
| `__main__` guard, worker processes | Finding | "the code that starts helper processes runs again on Windows" |
| UTF-8, code page | Finding | "accented letters and emoji can break" |
| executable flag, chmod +x | Info | "marked as a program" |
| Git Bash, WSL, cmd.exe, PowerShell | Findings | "Windows can't run Mac shell scripts" |
| registry | WinPy and WPF findings | "Windows' built-in settings store" |
| symbolic link, Developer Mode | CLI | "shortcut file" |
| LF/CRLF, .gitattributes | Line-endings finding | "line breaks saved the Windows way" |
| Snapshot, Pack, Known fix | Conversion | Copy, Standard fix |
| API key, keychain, Credential Manager | Own key | "access code from your AI account, stored in your Mac's password store" |
| retarget, net8.0-windows, XAML | WPF verdict | "the Windows-only version of .NET" |
| permissions | Unreadable files | "Portal isn't allowed to open them" |

---

## Findings

**1. The scan says nothing leaves the Mac while it contacts pypi.org.** *Blocks use.* Screen: Scanning (real scan with "Check packages on PyPI" on).

![Online scan](shots/copy-and-trust-02-online-scan-nothing-leaves.png)

The headline reads "nothing leaves this Mac" and the log below it reads "net online: pypi.org". This is the one sentence a privacy-minded user will check, and it's false. In the report, "Check packages online" doesn't say what it sends either.

*Fix:* Make the line depend on the setting. When online, say "Package names are looked up on pypi.org; nothing else is sent". Use the same words on the button.

**2. "Proved" and "passes" sit next to failures.** *Blocks use.* Screens: Checking it runs, Send files to the AI, Final report, Consent declined.

![Proved vs fails](shots/copy-and-trust-04-send-to-ai-proved-vs-fails.png) ![Consent declined](shots/copy-and-trust-06-winpy-consent-declined.png)

Examples:
- "Run the tests: 14 of 14 passed" is followed by "1 test fails on Windows", while the log says "run the tests: proved".
- The final report says "Tests pass · Proved · 14 of 14" and doesn't mention that the AI changed code to get there.
- In the ledger scenario, "Installs and passes its checks" sits above a **Failed** check.
- "Opens the workbook · Proved" appears although the export that makes the workbook was skipped.

*Fix:* Say "Passed" and name what ran. Carry failures into the headline. Mark any check that depends on a skipped one as Couldn't check. Show before/after counts when the AI repaired something.

**3. Headlines leave out what wasn't checked or failed.** *Blocks use.* Screens: Blocker report, Final report, CLI.

![Final report](shots/copy-and-trust-05-final-runs-on-windows.png)

- "Runs on Windows. 2 things need you." sits above "Answers from the model: Couldn't check", and answering is the whole point of a local-AI chat app.
- "Runs on the Mac" sits above a Failed check.
- In the CLI, my test app listed `mlx` and `pyobjc-framework-Cocoa` (both Mac-only). Offline, it got "No blockers found. 5 things will need a fix". Online, it said mlx "isn't on PyPI (a private or renamed package)". mlx is on PyPI.
- The CLI adds the caveat only when it finds no problems ("No problems found for Windows, but 1 thing couldn't be checked"). As soon as there are problems, the caveat disappears.

*Fix:* Always end the headline with "N couldn't be checked" or "N failed". State what happened instead of guessing why.

**4. The secrets promise is broader than the check, and it's worded five ways.** *Confuses.* Screens: Report, Final report, Send to AI, CLI.

The five versions:
- "Secrets aren't sent automatically"
- "Portal never sends secrets without asking"
- "Secrets stay on this Mac"
- "Portal never copies secrets"
- "No secrets."

In my test folder, `app.py` contained a hard-coded AWS-style key. The scan didn't flag it, still showed "Secrets stay on this Mac", and app.py would be sent.

*Fix:* Use one scoped sentence everywhere: "Values in .env and key files aren't sent. Portal doesn't look for keys written into code."

**5. Sending files to the AI doesn't say where they go.** *Confuses.* Screens: Send files to the AI?, Own key, Migration plan.

No screen names the AI provider or says whether files are kept or used for training. The cost line is half hidden behind the pinned "Send 4 files" button, and with Details open both the cost and the "Never sent" list scroll out of view (see the shot in finding 2). The migration plan offers "the AI can write this plan out in detail for your code" without saying the code is sent. The Node scenario asks to "Send 0 files" for about $0.05.

![Send 0 files](shots/copy-and-trust-09-node-send-0-files.png)

*Fix:* Add one line naming the provider and its retention policy, with a link. Put the cost on the button. Skip the stop when there are 0 files.

**6. Counts change between screens.** *Confuses.* Screens: Report → Planning → Checking → Final.

- Python: "5 things will need a fix" / "The 5 fixes above" → "Matching 6 findings" → "6 changes made" → "7 changes".
- Node: "2 things" → "Matching 5 findings" (4 listed) → log "5 fixes", "5 changes in 3 files" → "4 changes".
- Xcode, WPF and Documents show "1 of 0".

*Fix:* Explain each step: "5 problems → 6 changes (the UTF-8 fix touches 2 files) + 1 by the AI." Use one noun ("change") once work starts.

**7. The upgrade is pitched too often and too early, without the price.** *Annoys.* Screens: Project tab, Report.

![Report pitch](shots/copy-and-trust-03-report-couldnt-check-and-pitch.png) ![Paid badge](shots/copy-and-trust-01-drop-paid-badge.png)

- "A Pro conversion does this for you on Windows." repeats under every finding.
- The "Pro converts it for you" box appears even when "AI use: None. Nothing to change".
- That box promises "passes its tests" for projects that have none.
- "Convert with Pro" never shows a price.
- The free scan lives under a "Paid" tab badge.

*Fix:* Pitch once per report, with the price. Hide the pitch when there are 0 fixes. Say "tries to install, test and start". Move the badge onto "Pro conversion".

**8. The "used up" moments only let you pay.** *Annoys.* Screens: Allowance used up, AI budget used up, Plans.

![Allowance](shots/copy-and-trust-07-allowance-used-up.png) ![Plans](shots/copy-and-trust-08-plans.png)

- "Allowance used up" offers only "Use my own key" or "Allow $10 more". There's no "Wait until 1 November" and no close button, and the full red bar adds alarm.
- "Allow" is a softer word for paying.
- Plans says "Past that, you choose whether to allow more", without a price.
- The same thing is called "allowance", "budget", "AI use" and "AI usage".

*Fix:* Add a wait option. Use "Add up to $10 (charged as used)" and state the top-up terms on Plans. Pick one noun.

**9. The scan step is labelled backwards.** *Confuses.* Screen: Scanning.

A Mac project checked for Windows shows "Checking for Windows-only code", and a PC → Mac scan shows "Checking for macOS-only code". Developers will read this as a bug.

*Fix:* "Looking for code that won't run on Windows".

**10. One explanation covers several different problems.** *Confuses (for developers).* Screens: Report findings, CLI.

- "\`open\` isn't a Windows command" also covers `say`.
- "AUX is a reserved name" also covers a colon and a trailing dot.
- The Node `rm -rf` and quoting problems get only the `VAR=value` reason.
- Files are called folders ("config.json is a folder").
- "Mac line endings" means LF.
- "DerivedData is rebuilt on Desk PC" is said of an app that can't build there.

*Fix:* Give one reason per line. Check the wording for files vs folders.

**11. The user's data folder is filed under "clutter".** *Confuses.* Screen: Report, "won't be sent".

`data/ 222 MB` is listed under "The rest is ignored by the project or clutter." A non-developer won't know it is excluded because of .gitignore, and may arrive without their data.

*Fix:* Name it: "data/ is in .gitignore, so it isn't sent. Send it with Files if you need it."

**12. Machine names: "this" vs the computer's name.** *Confuses.* Screens: Conversion, Final report.

The copy uses "this Mac" or "this PC", taken from the window. The user starts the conversion on one computer and it runs on the other. In the Mac frame this produces "Checks on this Mac" for checks on Desk PC, "Install Ollama on this Mac" for an app that now lives in C:\Users\Sam…, and "Show in Finder" for a C:\ path. Auto mode gets it right, but "the same as on MacBook Pro" still implies tests ran on the Mac, which no screen shows.

*Fix:* Always use the machine's name for where something happens or must happen: "Install Ollama on Desk PC".

**13. Rebuild screens use three names for the same idea.** *Confuses.* Screens: Report, Verdict, Migration plan.

- "See the rebuild plan" opens a verdict, whose button then says "See the migration plan".
- The exports are called "hand-off note", "hand-off plan" and "migration plan".
- "Hand-off note saved" doesn't say where.

*Fix:* Use "Why it needs a rebuild" → "Rebuild plan". Call the exports "fix list" and "change list". Show the save location.

**14. The "Run a command?" stop undersells the risk.** *Confuses.* Screen: Run a command?

"This project wants to run…" puts the request on the project. "Runs … as you" hides that the command can touch anything you can. "Starting it is marked Couldn't check" is reused for an export.

*Fix:* See section 4.

**15. Unreadable files and offline state are easy to miss.** *Polish.* Screens: Unreadable files, Offline.

The headline stays "No blockers found" when 3 files couldn't be read. "Check their permissions" doesn't say how. The "Offline: couldn't check" state looks identical to the normal report.

*Fix:* Add both to the headline, give Finder steps, and show an offline banner with "Check online".

**16. "Doesn't need a rebuild" and documents.** *Polish.* Screens: Verdict, Documents report.

The double negative "doesn't need a rebuild / isn't an app for one OS" is hard to parse. For documents, "No problems found for Windows" and "It arrives clean" read as "everything will open". My CLI test left .pages, .numbers and .HEIC files unflagged, and Windows has no built-in app for the first two. An "AI use (estimate): None" line on a documents folder is noise.

*Fix:* "These file names all work on Windows. Portal doesn't check whether Windows has apps to open them."

**17. Tone slips.** *Polish.*

- "It's a big one" is cute.
- "doesn't pretend to convert it" is defensive.
- "clutter" is dismissive.
- "You chose not to…" leans toward blame.
- "Nothing is claimed" appears three times on one screen.

*Fix:* See the rewrites in sections 2, 8 and 10.

**18. CLI summary jargon and missing context.** *Polish.* Screen: `portal-scan`.

"Tier:", "Expected AI use" with no mention of Pro, "parsed", "How it's run here" (which lists build scripts), "(nothing else is sent)" without saying what is sent, and one error message for both "missing" and "no permission".

*Fix:* See section 11.
