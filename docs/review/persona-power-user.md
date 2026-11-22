# Persona review: the skeptical senior developer

## Who I am and what I tried

Senior developer. I move projects between Mac and Windows with `git push` / `git pull` and paste errors into an AI coding assistant. I don't trust tools that say they "fix" code, and I want to know what I get for money that I can't get free.

I ran `portal-scan` on gunicorn, express and ripgrep (shallow clones) and on three 10-line Unix-only probes (Rust, Go, Node). I scanned the same repos through the lab's "Scan a real folder" box, then went through every Pro screen in the Python scenario, plus the Xcode, WPF and Node ones.

## Task 1: Is the free scan accurate on repos I know?

**Finished.** About 12 commands. Each scan took 0.1 to 1.3 s.

| Repo | What Portal said | What I know |
|---|---|---|
| gunicorn | "1 blocks, 9 need a fix", "Tier: Apps: a FastAPI server" | It's a pre-fork WSGI server that is Unix-only by design. "FastAPI" comes from a Docker test fixture. The real answer is "use waitress/uvicorn on Windows". |
| express | "1 needs a fix, 1 needs you" | 11 of the 13 "Glues paths with '/'" hits are URL routes (`res.redirect('/user/' + id)`). `.npmrc` gets flagged as "holds credentials", but it contains `package-lock=false`. Nothing real was found. |
| ripgrep | "No blockers found… Scripts: 2 shell scripts" | It's a Rust project with 102 `.rs` files. The headline is correct, but only by luck. |

My probe projects:

```
unixrs (std::os::unix, UnixListener, libc::fork)  → "No problems found for Windows."
unixgo (syscall.Kill, SIGUSR1, /proc/self/status) → "No problems found for Windows."
unixnode → caught rm -rf, ls, $HOME; missed process.getuid(), SIGUSR2,
           and "start": "NODE_ENV=production node index.js"
```

- **Confused by:** `--json` says Rust support is `"summary"`, but the human summary never says "Rust wasn't checked"; the only hint is "225 files (2 parsed)".
- **Good:** every finding has file:line and the source line. It states plainly that nothing was run. `--online` found `gunicorn-h1c` has no Windows wheel, which I wouldn't have checked.
- **Jargon:** "Tier: Apps / Scripts / Static", "Glues paths", "Needs you".
- **Expected vs happened:** for Python I got roughly a platform linter (high recall, noisy in tests and benchmarks). For other languages I expected "not checked" and got "No problems found": a false all-clear, the one outcome I can't catch myself.

## Task 2: What exactly does Pro do, what goes to the AI, and what does it cost?

**Partly finished.** 9 screens. I found the prices but not the AI provider.

- **Plans:** Free $0 (scans), Pro $12/month "with up to $20 of AI use", Own key $5/month plus my AI bill ([shot](shots/persona-power-user-10-plans.png)).
- **Sent to the AI:** the consent screen is the best in the product: 4 named files, 4 KB, what's never sent, and the cost, "About $0.05" ([shot](shots/persona-power-user-05-send-to-ai.png)).
- **Not stated anywhere:** the AI provider or model, whether my code is retained or trained on, and the rate behind "$20 of AI use". I grep'd every screen; none. The `sk-…` placeholder hints at a vendor without naming it.
- **The pricing confuses me.** Pro gives $20 of AI for $7 more than Own key. Either the AI is marked up, or the cap never matters: at $0.18 a conversion I'd need 100+ a month to hit it. I'd be paying $12 for the harness, not the AI.
- **The key's location is unclear.** The Own key screen says the key is "Kept in the Mac's keychain on MacBook Pro", but conversions run on Desk PC. Does the key travel to the PC? ([shot](shots/persona-power-user-11-own-key.png))
- **Marketing words:** "Pro converts it for you", "Portal fixes it".

## Task 3: Is "Proved" actually proved?

**Finished, not convinced.** 6 screens.

- **What I liked:** the final report splits results into Proved, Couldn't check and Needs you, and says "Nothing is claimed" for the parts it skipped ([shot](shots/persona-power-user-07-final-report.png)). The AI repair screen shows attempt 1 being undone with a reason ("the old index would not match"). That's the kind of honesty that earns trust ([shot](shots/persona-power-user-06-ai-repair.png)). Per-change diffs come with Undo ([shot](shots/persona-power-user-09-review-changes.png)).
- **What I can't check:** "Tests pass: Proved, 14 of 14" can't be clicked. There's no command, no output and no exit code behind it.
- **The story contradicts itself:**
  - "Checking it runs" says 14 of 14 passed.
  - The AI consent screen says "1 test fails on Windows".
  - The Details log shows `run the tests: proved` at 09:44:31, then the AI's code change at 09:44:40, and **no verify line after it** ([shot](shots/persona-power-user-08-details-log.png)).

  If the log doesn't support the label, the label is a claim.
- **Expected vs happened:** I expected "Proved" to link to pytest output; it's a word with a checkmark.

## Task 4: Portal vs "git push, git pull, ask an AI"

**Finished.** I couldn't find where Pro's changes end up in my repo.

- Pro edits a copy at `C:\Users\Sam\Projects\local-chat`. No screen mentions git, a branch, a commit or a patch, so my Mac repo and the PC copy diverge.
- "Hand-off note saved" doesn't say where it was saved or in what format. In the CLI, `--json` has a `handoff` field, but it's `null` for gunicorn and express and only filled for ripgrep.
- Pro saves me the PC setup, test run and fix-and-retry loop, about 15 minutes. Running `portal-scan`, pushing, pulling and pasting the output into my AI assistant gets 80% of the result for free.
- The device header switches from "Desk PC" to "MacBook Pro" without explanation; it names the *other* machine.

## Task 5: Does it know when *not* to convert?

**Finished.** For Xcode and WPF apps the rebuild verdict is excellent: "Portal doesn't pretend to convert it", plus what carries over and a concrete toolkit choice ([shot](shots/persona-power-user-12-rebuild-verdict.png)).

gunicorn gets none of that. Its 26 `os.fork`/`setuid` calls and 50 signal uses sit under "Known problems with known fixes", with fixes "Use getpass.getuser()" and "Use SIGTERM or SIGINT". Then: "Pro converts it for you… checks that it installs, passes its tests and starts" ([shot](shots/persona-power-user-02-gunicorn-fork-fix.png)). I don't believe it, and it makes me doubt the claims on projects I know less well.

## Would I pay?

**No, not today.** If I had to pick a plan, it would be Own key at $5, because I already pay for an AI.

- The free scan is a decent Python and Node platform linter. I'd keep the CLI around and use `--online`.
- Pro's real value is the harness: a run on the target OS, consent stops, per-change undo, test checks. That's worth something, but not $12 a month for a job I do a few times a year. I'd pay $3–5 per conversion.
- Three things kill it for me:
  - Silent all-clears on languages it doesn't read.
  - Offering to "convert" a Unix-only server.
  - "Proved" labels I can't inspect.

**What would change my mind:**

1. The scan says "Rust/Go not checked" right in the headline.
2. Every "Proved" opens the exact command and its output.
3. Changes land as a git branch or patch I can push.
4. A named provider and model, plus a retention statement.
5. Pay-per-conversion pricing.

## Findings

**1. Unread languages get a clean bill of health.** Severity: blocks use. Screen: scan report and CLI ([shot](shots/persona-power-user-04-ripgrep-report.png)).
- *What happened:* Unix-only Rust and Go code returned "No problems found for Windows". ripgrep was labelled "Scripts: 2 shell scripts".
- *Expected:* "Rust: not checked (102 files)" under Couldn't check, and a headline that doesn't say "No problems".
- *Fix:* put every ecosystem with `support != deep` in Couldn't check. Never print "No problems found" when parsed files are 0.

**2. Pro is offered for software that is Unix-only by design.** Severity: blocks use. Screen: real scan of gunicorn ([shot](shots/persona-power-user-01-gunicorn-report.png), [shot](shots/persona-power-user-02-gunicorn-fork-fix.png)).
- *What happened:* `os.fork`, `setuid` and SIGHUP are labelled "known fixes", with a fix hint about `getuser()`. "Convert with Pro" is offered.
- *Expected:* a verdict like the one for Xcode: "Won't run on Windows; use waitress/uvicorn there."
- *Fix:* treat fork, setuid and process-signal clusters as a blocker or rebuild signal. Write fix text for each call, not one per rule.

**3. False positives a mechanical fix could make worse.** Severity: confuses. Screen: express real scan ([shot](shots/persona-power-user-03-express-url-paths.png)).
- *What happened:* URL routes are flagged as filesystem paths, with the fix "use path.join". `.npmrc` without tokens is flagged as "credentials". gunicorn is classified as "a FastAPI server".
- *Expected:* skip strings passed to redirect, URL and route calls. Flag `.npmrc` only when it contains `_authToken`. Ignore `tests/docker` when deciding the app type.
- *Fix:* as above, and never auto-apply the path fix to URL-shaped strings.

**4. "Proved" can't be inspected and the log contradicts it.** Severity: confuses. Screen: final report and Details ([shot](shots/persona-power-user-07-final-report.png), [shot](shots/persona-power-user-08-details-log.png)).
- *What happened:* there's no output behind any Proved row. The log shows no re-verify after the AI's edit. The screens say both 14/14 and "1 test fails".
- *Expected:* each Proved row opens its command, exit code and output, and a final verify runs after the last change.
- *Fix:* make the rows expandable and keep the demo data consistent.

**5. AI provider, model and retention are never stated.** Severity: confuses. Screens: Send files to the AI, Plans, Own key ([shot](shots/persona-power-user-05-send-to-ai.png)).
- *What happened:* "the AI" and "your AI provider" are never named.
- *Expected:* "Sent to <provider>, <model>; not used for training; kept N days."
- *Fix:* add one line to the consent screen and to Plans.

**6. No git story.** Severity: confuses. Screen: final report, Reviewing changes ([shot](shots/persona-power-user-09-review-changes.png)).
- *What happened:* the changes live in a PC folder, and nothing says how they get back to my repo.
- *Expected:* "Commit on branch portal/windows" or "Export as patch".
- *Fix:* offer a branch, patch or PR as the output. Say where the hand-off note is saved.

**7. The plan math reads like marketing.** Severity: annoys. Screen: Plans ([shot](shots/persona-power-user-10-plans.png)).
- *What happened:* "$20 of AI use" has no unit I can relate to. *Fix:* say "≈100 conversions like your last one" and add pay-per-conversion.

**8. Own key storage doesn't match where the conversion runs.** Severity: annoys. Screen: Own key ([shot](shots/persona-power-user-11-own-key.png)).
- *What happened:* the key is in the Mac's keychain; conversions run on the PC. *Fix:* say which machine calls the AI and whether the key crosses the link.

**9. Small wording issues.** Severity: polish.
- "5 KB won't be sent: tests/" reads as if all tests are skipped; it's 2 gitignored files. Name them.
- The gunicorn secrets sentence runs three paths together with "looks like".
- Long diff lines are clipped with no horizontal scroll.
- The CLI's `handoff` is `null` for 2 of 3 repos. Always fill it, or add `--handoff`.
