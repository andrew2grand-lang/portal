# Progress

# 0.5.0-beta.2: the Settings hotfix (2026-11-24)

**Why:** in beta.1 Settings did nothing in the real app (decision 300). **Fixed, and guarded:**
- Settings save again (`SettingsPatch` fields all optional). A **UI ↔ Rust contract test** puts every invoke the window sends (98, all 54 commands, one partial patch per Settings field) through the real Rust types; it failed on 24 calls before the fix (decision 300). CI runs it on Windows.
- **No silent failures:** a failed action says so in the window and adds a line to Details (decision 301).
- **No false update error** while only test builds exist: "No updates yet." / "You're on the latest test build." (decision 302).
- **End-to-end test of the real built app** on Windows and Linux runners (tauri-driver): 30 controls in Settings, the Shelf and Details, each checked against the real core; 30 of 30 on Linux, 29 of 30 then fixed on Windows. What one computer can't exercise is listed (decision 305).
- **Found on the way:** no Help in Settings; the macOS and Windows window configs never got the 300 × 360 minimum (mini mode was unreachable) (decision 306); the main window is built by setup so Edge's WebDriver reaches it (decision 307); the first CI runs since Mission 05 fixed (decision 308).
- **The drop area** says where files go and keeps its rings and glowing icon; meaning put back on five screens §6 cut too far (decisions 303–304).

**Tested:** CI on real macOS and Windows (check, ui, build, release) and Linux (e2e); here: every Rust test (404), clippy, fmt, 110 UI tests, the layout check. Installed on this Mac (local build). **The 5-minute Settings check** is at the top of `docs/MANUAL-TESTS.md`.

# Mission 05: clear, calm, and a real Shelf

Plan: `docs/PLAN-05.md`. Spec: `MISSION-05-clear-calm-and-the-shelf.md`. Work happens on branch `mission-05` (cheap checks on push) and goes to `main` at the three CI milestones (heavy suites). Started 2026-11-21 15:49 GMT.

## Skills (Mission 05)

**Loaded and used:**
- Process: `superpowers:writing-plans` (PLAN-05), `superpowers:test-driven-development` (a failing test before every §1 fix), `superpowers:systematic-debugging` (cause first, file and line), `superpowers:verification-before-completion`, `superpowers:dispatching-parallel-agents` (the fresh-eyes personas), `engineering:system-design` (liveness, the queue's hand-over, Shelf sync without relaying), `engineering:testing-strategy` (loopback proxies, traces, three-peer tests), `security-review` (the Shelf at rest, private items, received-file safety, picture checks), `engineering:code-review`.
- UI and words: `impeccable`, `frontend-design`, `apple-design` (the receiving light, the arrival moment, mini mode), `design:ux-copy` (the writing rules, Help), `design:accessibility-review`, `design:design-critique`, `web-design-guidelines`, `vercel-composition-patterns`, `vercel-react-best-practices`, `playwright-cli` (screenshots, the overlap check).

**Available but not relevant:** GSAP, Lenis, react-bits, image generation and brand-kit skills, `design-md`, the taste skills, the marketing library, finance / legal / sales / PE / IB / small-business / financial-advisor plugins, Vercel / Wix / Shopify / Canva / Figma / Notion connectors, docs/pptx/xlsx/pdf skills.

**Missing:** no Rust, Tauri, networking or cryptography skill is installed. I work from the crate docs (`ring`, `png`, `tokio`), Apple's and Microsoft's documentation (Keychain, DPAPI, quarantine, Mark-of-the-Web) and the Tauri v2 guides and bundler source.

## Status (Mission 05)

| # | Step (spec §12) | Status | Time |
|---|---|---|---|
| 1 | Plan | done (`docs/PLAN-05.md`) | 15:49–15:52 (3 min) |
| 2 | §1 fixes and the nearby list; CI milestone 1 | done: each §1.1–1.9 cause confirmed in the code and a test that failed first (decisions 240–270, 281); CI milestone 1 couldn't start (decision 290) | 15:52–16:59 (67 min) |
| 3 | §3 Clipboard and Shelf | done: the Shelf (mutual consent, no relaying, sealed at rest), Clipboard as its own place, Recently received, every unsynced copy said once (decisions 271–280) | 16:59–17:35 (36 min) |
| 4 | §4 received files, §6 Help and the writing rules, §7 supported systems; CI milestone 2 | done: three receive modes, arrival dates, the migration; Help; 1,875 → 1,028 words on 25 screens; supported systems in Help and the README (decisions 281–286); CI milestone 2 couldn't start | 17:35–18:12 (37 min) |
| 5 | §8.2 beta tooling, checked here first | done: Get test builds, both feeds, beta tags in the tooling (decisions 287–289) | 18:12–18:26 (14 min) |
| 6 | §11 fresh-eyes review | done: `docs/review-05/SUMMARY.md`; 3 blocks use and 20 confuses fixed and re-checked (screenshots, 99 UI tests, layout check) | 18:28–19:12 (44 min) |
| 7 | Should-haves (mission at 3 h 23 min, under the 6-hour limit) | done: §2.1 Pair again / OK, §2.2 "+" in the strip, §2.3 device pictures, §5 mini mode (decisions 292–296) | 19:12–19:30 (18 min) |
| 8 | Final checks, local install, beta.1 | done here: every Rust test, clippy, fmt, the UI checks and the layout check (3,744 layouts, none with problems); one core bug found and fixed (decision 297); version 0.5.0-beta.1; local release build installed on this Mac (decision 298). **Not published:** CI can't start (Needs the owner 1) | 19:51–21:05 (74 min; 19:30–19:51 paused at the usage limit) |

## Summary (Mission 05)

**Done**
- **§1, the second hardware test's bugs:** each cause confirmed in the code and a test that failed first (decisions 241–270, 281). Receiving is a travelling light, not an orange crawl; a vanished computer shows Paused within 2.6 s (20 of 20 runs), and a 300 KB/s link or a long re-check never pauses (20 of 20 each); time left follows the brief's five rules and the recorded traces; Quit or a crash asks Continue or Cancel, a dropped network or an update continues by itself; a busy computer gets a clear message and a queue that hands over earliest first; receiving gets the same finished moment as sending, with Open only for safe single files; segmented controls hug their options; every nearby computer is listed with Pair; a copy another app holds for a moment still syncs.
- **§3 Clipboard and Shelf, one job each:** Clipboard (send, sync switched only there, Recently received, every unsynced copy said once with Put on Shelf); the Shelf (things you keep on purpose; shared by mutual consent, never relayed, sealed on disk with an honest threat model; triangle and chain tests).
- **§4 received files:** straight into Downloads by default, a SeamDrop folder, or a new numbered folder per delivery; dated on arrival unless "Keep original dates"; the update moves settings as specified.
- **§6 and §7:** Help with eight searchable sections; every screen cut to the writing rules (1,875 → 1,028 words on 25 screens, before/after screenshots in `docs/screens/m05-words-*`); supported systems in Help and the README (native Windows ARM64 would be cheap, not built; older systems not worth it).
- **§8.2 test builds:** Settings → About → Get test builds (off by default) checks the stable and the beta feed and takes the higher version, never going back; the release tooling takes `vX.Y.Z-beta.N` and publishes it as a pre-release with its own fixed feed, leaving the stable feed alone. **0.4.0 is never published.**
- **§11 fresh-eyes review:** three reviewers; 3 "blocks use" and 20 "confuses", all fixed and re-checked (`docs/review-05/SUMMARY.md`).
- **Should-haves:** §2.1 "Desk PC removed this pairing." with Pair again / OK; §2.2 "+" at the end of the device strip; §2.3 device pictures (a colour and an icon, or a photo cut to a circle; a disc wherever a device appears); §5 mini mode below 360 × 480, the minimum window now 300 × 360, the drop icon the last thing to go.
- **Found by the final check, fixed:** a link's first messages could be lost when they reached the core's loop before the link itself (decision 297); a computer coming back after a restart could miss the other's Shelf answer, status and clipboard-sync answer.

**Tested, and how**
- **This Mac only** (CI couldn't start all mission: GitHub billing, decision 290): every Rust test in the workspace (391), clippy with warnings as errors, rustfmt; the UI's type check, lint and 99 unit tests; the layout check in Chrome, 3,744 layouts (624 states × 6 sizes including mini mode, both windows, both themes, 1 and 6 devices), none with problems.
- **Loopback between processes / cores in one process:** liveness, the queue, quit vs drop, the Shelf triangle and chain, received files, pictures, pairing removal.
- **Untested on real hardware:** everything between two or three real computers: that's the 25-minute check at the top of `docs/MANUAL-TESTS.md`. The Windows build of this version hasn't been built at all (no CI).
- **CI minutes this mission: 0** (no job could start).

**Needs the owner**
1. **Publish 0.5.0-beta.1 once GitHub runs jobs again.** GitHub refuses to start any job on the private repo ("recent account payments have failed or your spending limit needs to be increased"). We spend nothing, so don't raise the limit: wait for the monthly minutes to reset (Settings → Billing and plans shows the date), or fix the payment method if one really failed. Then, in Terminal:
   ```bash
   cd ~/Code/portal
   gh workflow run check.yml --ref main && gh workflow run ui.yml --ref main
   git push origin v0.5.0-beta.1
   ```
   The tag is already made on this Mac (`git tag -l v0.5.0-beta.1`). Wait for Actions → release to pass (about 15 minutes), then:
   ```bash
   npm run release:publish -- v0.5.0-beta.1
   ```
   It creates a **pre-release** (never "latest") in seamdrop-releases, writes the beta feed, and checks the stable feed didn't change. Send me the links of any red run.
2. **Install beta.1 by hand on the PC and Sam's MacBook Air** (today's installs only read the stable feed, and have no Get test builds switch):
   - **PC (Den_PC):** open https://github.com/andrew2grand-lang/seamdrop-releases/releases, open "SeamDrop 0.5.0-beta.1" (marked Pre-release), download `SeamDrop_0.5.0-beta.1_x64-setup.exe` and run it; at "Windows protected your PC" press More info → Run anyway. It replaces 0.3.1 and keeps your settings and pairings.
   - **Sam's MacBook Air:** from the same page download `SeamDrop_0.5.0-beta.1_universal.dmg`, open it and drag SeamDrop into Applications (replace the old one). Then either run `xattr -dr com.apple.quarantine /Applications/SeamDrop.app` in Terminal (or `~/Applications/SeamDrop.app` if that's where it lives), or open it once, and when macOS says it can't check it: System Settings → Privacy & Security → **Open Anyway**.
   - **To start before the reset with the two Macs only:** if Sam's MacBook Air has an Apple chip (Apple menu → About This Mac → "Chip: Apple M…"), AirDrop `~/Downloads/SeamDrop_0.5.0-beta.1_aarch64.dmg` from this Mac and install it the same way. The PC can't talk to 0.5 until it has beta.1 too ("Update SeamDrop on Den_PC").
   - Then on each computer: Settings → About → turn on **Get test builds**. From beta.2 on, updates arrive in the app.
3. **This Mac now runs 0.5.0-beta.1** (installed in `~/Applications`). macOS may ask again to let SeamDrop find devices on your local network, because a new build has a new ad-hoc signature: press **Allow**. If it doesn't ask and nothing shows up: System Settings → Privacy & Security → Local Network → SeamDrop on. The old 0.3.1 is kept in `~/Applications/.seamdrop-0.3.1-backup` if you ever need to go back.
4. **Back up the new updater key** (decision 299): copy `~/.tauri/seamdrop-updater.key` somewhere off this Mac, and keep its password in your password manager. If either is lost, installed copies can't be updated any more.
5. **A leftover Keychain dialog** from the Keychain test (decision 271), if you see one asking about "seamdrop": press **Deny**. The test item was already deleted.
6. **Run the 25-minute check** at the top of `docs/MANUAL-TESTS.md` once all three computers have beta.1, and send me what it asks for.
7. Still open from Mission 04: the website and feedback address (`src/links.ts`, `confirmed: false`), the Firefox/Tor private-window test, the update key backup, and the optional `RELEASES_TOKEN`.

**Next mission** (from review 05's annoyances, `docs/review-05/SUMMARY.md`): a "last received" line after Done; files that wait for a sleeping computer; "ran out of space" says how much; Recently received with sizes and item-by-item clear; Private before you paste; transfers stay visible while you add more; progress while checking after a resume; a checklist when Pair another finds nothing; the finished summary says it continued after a pause; a native Windows ARM64 build (decision 286).

# Mission 04: solid desktop, many devices

Plan: `docs/PLAN-04.md`. Spec: `MISSION-04-solid-desktop-many-devices.md`. Work happens on branch `mission-04` (cheap checks) and is merged to `main` at milestones (heavy suites).

## Skills (Mission 04)

**Loaded and used:**
- Process: `superpowers:writing-plans` (PLAN-04), `superpowers:test-driven-development` (a failing test before every fix), `superpowers:systematic-debugging` (cause first for every §1 bug), `superpowers:verification-before-completion`, `superpowers:dispatching-parallel-agents` (fresh-eyes personas), `engineering:system-design` and `engineering:architecture` (reach state, transfers by id, pairing), `engineering:testing-strategy` (process-level and four-peer tests, traces), `security-review` (PAKE pairing, the clipboard), `engineering:code-review`.
- UI: `impeccable`, `frontend-design`, `apple-design` (motion: the compositor glow, paused state), `design:accessibility-review`, `design:ux-copy`, `design:design-critique`, `web-design-guidelines`, `vercel-composition-patterns`, `vercel-react-best-practices`, `playwright-cli` (screenshots, overlap test, performance traces).

**Available but not relevant:** GSAP, Lenis, react-bits, image generation and brand-kit skills, `design-md`, the taste skills, finance / legal / sales / PE / IB / small-business / financial-advisor plugins, Vercel / Wix / Shopify / Canva / Figma / Notion connectors, docs/pptx/xlsx/pdf skills.

**Missing:** no Rust, Tauri, networking or cryptography skill is installed. I work from RFC 9382 (SPAKE2), the crate docs, Chromium's and Mozilla's sources, Apple's and Microsoft's documentation, and the Tauri v2 guides.

## Status (Mission 04)

| # | Step (spec §12) | Status |
|---|---|---|
| 1 | Plan | done (`docs/PLAN-04.md`) |
| 2 | §1.1 connecting at launch (core reach state) | done: 34 start orders of two `peer.rs` processes all connect within 3.0 s (loopback between processes; 7 missed 5 s before); reach state tested in-process; Windows runs the process test on `main` |
| 3 | §1.2 time left and speed; §1.3 glow; §1.4 paused | done: trace tests (Rust) fail on the old formula and pass; the sender measures confirmed bytes (tested in-process, loopback); glow trace 0 repaints/s and 30 frames during a blocked main thread (Chrome here; Chromium/Edge on `main` CI); paused shows a still mark |
| 4 | §3 many devices (data model, then screens) | done: transfers by id, one active per device, space reserved, every paused transfer kept, clipboard consent per pair, remembered choice, tray for N devices (`tests/many.rs`, 4 cores, loopback); UI keyed by device id with a device strip and a transfer list; all 33 mock states load without errors with 1 and 6 devices |
| 5 | §1.5 overlap test and fixes; §1.6 Details; §1.7 appearance; §1.8 received folder | done. §1.5: the test (built before the fixes) walks 50 states × 4 sizes × 2 chromes × 2 themes × 1 and 6 devices: 215 of 752 Mac-chrome layouts had problems before, 0 of 1,600 after (Chrome 157 here; WebKit on macOS and Edge on Windows in CI on `main`); screenshots `docs/screens/m04-*.jpg`. §1.6: every core line in plain words with its detail apart, calm tones (tests: a good transfer has no amber/red, a failed one exactly one red line in its screen's words), repeats collapsed (unit-tested), Copy technical log (not synced; tested). §1.7: System · Light · Dark, a core setting shared with the tray (kept across restarts, tested); native window theme follows (untested on real hardware). §1.8: first-arrival note with Change folder, Open/Change in Settings, a folder per device (3 two- and three-core tests: names, renames, duplicates, restart) |
| 6 | §2 pairing by typed code | done: SPAKE2 (RFC 9382, P-256) with all four RFC vectors; 14 two-core tests (loopback, in-process): right code, typos, three wrong, the global lock and Allow pairing again, Ignore (silent, timeout), cancel either side, timeouts, a man in the middle with two TLS sessions fails on both sides, an echoed confirmation is refused, protocol-2 messages and hellos pair nobody, restart, removal, a changed key; the upgrade test on 0.3.1 data folders; protocol 3; `docs/PROTOCOL.md` §1–3; screens for both sides (overlap-checked). Untested on real hardware: two computers on a real network (MANUAL-TESTS §4, §4b) |
| 7 | §4 clipboard: Shelf, files, named marker, pause/off, stress tests | done: Shelf text box (with Private since the review), files offered as a transfer, named private markers with the brief's wording, privacy line, stress tests (fast copies, long text and every script, Send clipboard during a transfer, three computers without loops: in-process, loopback); which markers browsers set, measured on the Windows runner (decision 214) |
| 8 | §5 small things; §6 protocol document, phone checks | done: Project tab behind a developer setting (off); `src/links.ts` with `confirmed: false` (tested: nothing opens or mails); Save technical log / Send feedback in About and Details; a Shelf key beside Settings; review 03's annoyances (below); `docs/PROTOCOL.md` complete (discovery, hello, pairing, transfers, resume, clipboard, relay); Android and iOS checks green in CI on `main` |
| 9 | §7 LocalSend bridge (optional) | skipped, said why (decision 213); what it would take is under "Next mission" |
| 10 | §11 fresh-eyes review, fixes, final summary | done: `docs/review-04/SUMMARY.md`; 3 blocks use and 31 confuses, all fixed or traced to the simulation (and the simulation fixed), each re-checked; the summary is below |

## Summary (Mission 04)

**Done**
- **Opening both computers connects by itself** (§1.1). The core reports how connecting is going ("Connecting…" first, "isn't reachable" only after a grace period and real failed attempts), and both sides dial each other at start. 34 start orders of two app processes all connected within 3 s.
- **Calm numbers** (§1.2–1.4): time left from confirmed bytes, smoothed, in coarse words ("About 2 minutes"); speed from what the other side confirmed; at 100 % "Finishing…"; the finished speed counts only what crossed the wire. The edge glow costs nothing while it runs; paused shows a still mark.
- **No overlapping text** (§1.5): a layout check walks every state at four window sizes, both window styles, both themes, one and six devices. 215 of 752 layouts had problems before; after review 04's changes, 2,048 layouts (64 states, both window styles, both themes, one and six devices, four sizes) and none with problems (Chrome 157 here; WebKit and Edge in CI on `main`). The check now fails a page that hasn't rendered (decision 238).
- **Details tells a story** (§1.6) in plain words, newest last, calm colours, repeats folded; the technical part is behind a switch; Copy technical log has everything.
- **Appearance and the received folder** (§1.7, §1.8): System, Light or Dark (window and tray); the first arrival offers Change folder; Open/Change in Settings; a folder per device.
- **Pairing by typed code** (§2): SPAKE2 (RFC 9382) bound to both certificates and the TLS session. One computer shows six digits, you type them on the other. Typos are allowed, three wrong codes end it, ten in all lock pairing until allowed again (and say who and when). No old scheme, no fallback. Existing pairings keep working (tested on 0.3.1 data folders). Protocol 3.
- **Many computers** (§3): any number of paired devices; tick where files and the clipboard go; one transfer per device, several devices side by side; every paused transfer kept; clipboard sync agreed per pair; send more while transfers run.
- **Clipboard** (§4): the Shelf (arrivals first, take one off, Clear asks), a text box with **Private**, files on the clipboard offered as a transfer, the private marker named, pause and off. Which browsers mark private windows was measured on Windows.
- **Small things** (§5): Project tab behind a developer setting; links and feedback gated on `LINKS.confirmed` (false: nothing opens or mails; "Save technical log" says what the log holds).
- **Protocol and phones** (§6): `docs/PROTOCOL.md` describes everything on the wire; the core checks for Android and iOS in CI.
- **LocalSend bridge** (§7): skipped, with why (decision 213).
- **Fresh-eyes review** (§11): four reviewers; 3 blocks use and 31 confuses, every one fixed or traced to the simulation (which was fixed too), and re-checked (`docs/review-04/SUMMARY.md`).

**Numbers**
- 350 Rust tests in the workspace, all passing here (plus the ignored real-clipboard, firewall and process tests that CI runs); clippy clean with warnings as errors.
- 95 UI tests; type-check and lint clean.
- Layout check: 2,048 layouts, 0 with problems (above); screenshots `docs/screens/m04-*.jpg`.
- Branch CI on `882261e`: ui and check (Windows) green, including the real Windows clipboard with the new private marks.
- `main` CI on `70ef754` (every review fix): green for the macOS and Windows builds, the Android and iOS checks, the scan, the glow traces (Chromium, Edge), the browser-marker run, the layout check in WebKit on macOS (512 layouts, none with problems), and the Windows core tests with the real clipboard and the firewall repair. Two things failed: the layout check in Edge on Windows (507 of 512 fine; 5 failed in one case because a simulated transfer now finishes and finished while being measured), and the Windows launch test (it never started its helper: newer Cargo moved test executables). Both are fixed in `7b65315` (decision 239), and both fixes pass here, in Chrome and on this Mac. **CI couldn't check them:** GitHub stopped starting jobs ("your spending limit needs to be increased") because the free minutes for the month are used up.
- CI minutes this mission: 928 billed (Windows 292 × 2, macOS 31 × 10, Linux 34; `node scripts/ci-minutes.mjs 2026-11-14`).

**Tested where**
- **CI on real macOS and Windows** (`main`): builds for both; the UI suite; the layout check in WebKit on macOS (and in Edge on Windows, all but the one case fixed after); the glow trace in Chromium and Edge; the core and system crates on Windows, with the real Windows clipboard (including the new private marks) and a real Windows Firewall repair; the browser-marker run; Android and iOS checks. Not yet on Windows: two app processes opened together (the fix waits for CI minutes).
- **Loopback between processes** (this Mac): two `peer.rs` app processes started in 34 orders.
- **In-process, loopback** (this Mac; most core tests): pairing (14 two-core tests, the RFC vectors, a man in the middle), transfers and resume, four cores side by side, the clipboard, the reach state, the doctor.
- **The real macOS pasteboard** (this Mac): private marks on what arrives as private.
- **Untested on real hardware:** anything between two real computers on a real network: pairing, connecting at launch, a big transfer that drops, the clipboard both ways, Win+V leaving private text out, Firefox and Tor Browser private windows. That's the 20-minute check.

**The 20-minute check** is at the top of `docs/MANUAL-TESTS.md` ("Mission 04: the 20-minute check"): opening both connects; pair by typing the code; a 10 GB transfer that drops and comes back (and dropping more files meanwhile); the smallest window; the clipboard, the Shelf and private text; appearance and the received folder; a third computer if you have one.

**Needs the owner**
1. **Run the 20-minute check** (`docs/MANUAL-TESTS.md`, top) on the Mac and the PC at home, then tell me what you saw. Both computers need this build: protocol 3 can't pair or send with 0.3.x.
2. **Release 0.4.0 once the check passes**, so both computers update together (protocol 3): follow `docs/RELEASING.md` from step 2 with `0.4.0`. Its notes are already in `CHANGELOG.md` (they start "Update both computers"); change them if you like. I haven't tagged it: a release publishes to the public repo.
3. **Confirm the website and feedback address.** We don't own seamdrop.com. When you have a domain and an inbox: put them in `src/links.ts` (`website`, `feedback`), set `confirmed: true`, and update the two allowed URLs in `src-tauri/capabilities/default.json` (`https://seamdrop.com/*`, `mailto:feedback@seamdrop.com*`) to match. Until then the app opens nothing and mails nothing, and says so.
4. **Firefox and Tor Browser private windows** on the PC (MANUAL-TESTS §7c): Playwright's Firefox didn't mark its copies, Mozilla says real Firefox does, and your Tor Browser copy was refused. Copy a sentence in a real private window, press Send clipboard, and tell me what the message says.
5. **CI minutes ran out for the month.** GitHub won't start jobs on the private repo until the free minutes reset (Settings → Billing and plans shows the date). Don't raise the spending limit (we spend nothing). After the reset, run the two checks that are waiting: in Terminal, from the repo, `gh workflow run check.yml --ref main` and `gh workflow run ui.yml --ref main`, then look at Actions: "check" should show "Two apps opened together connect within 5 s" green on Windows, and "ui" the Edge layout check green. If either is red, send me the run's link. (Making the repo public would make minutes free, but it's private on purpose: your call.)
6. **Back up the update key** if you haven't (Mission 03, Needs the owner 1). Signing and the releases token are as before (Mission 03, 2 and 7).

**Next mission**
- The LocalSend bridge (§7): an HTTPS listener on 53317, multicast announces, its own rate limits and size caps, prompts for unverified names; never touching SeamDrop pairing or trust.
- From review 04 (annoys): free space checked when an offer arrives and a pause instead of failing when the disk fills; a Pause key; a stalled transfer's elapsed time; speed and an opened view per row with several transfers; "Why?" on an unreachable tile; Settings rows that expand.
- From review 03, still open: Pause (above); the clipboard history is cleared by an update's restart (memory only, by design).

# Mission 03: SeamDrop, launch-ready free app

Plan: `docs/PLAN-03.md`. Spec: `MISSION-03-seamdrop-launch-ready.md`.

## Skills (Mission 03)

**Loaded and used:**
- Process: `superpowers:writing-plans` (PLAN-03), `superpowers:test-driven-development` (red first for every behaviour change), `superpowers:systematic-debugging`, `superpowers:verification-before-completion`, `engineering:system-design` (resume protocol), `engineering:architecture` (decision log), `engineering:testing-strategy` (two-node tests, relay cuts, local update feed), `security-review` (updater keys, clipboard privacy), `engineering:code-review` and an independent reviewer, `engineering:deploy-checklist` (release flow).
- Design: `impeccable`, `frontend-design`, `apple-design` (motion, macOS conventions, reduced motion), `design:ux-copy`, `design:accessibility-review`, `design:design-critique`, `web-design-guidelines`, `vercel-composition-patterns`, `playwright-cli` (screenshots of every new state, both windows, both themes).

**Available but not relevant:** GSAP, Lenis, react-bits, image generation and brand-kit skills, `design-md`, the taste skills, finance / legal / sales / PE / IB / small-business / financial-advisor plugins, Vercel / Wix / Shopify / Canva / Figma / Notion connectors, the DJ and watch skills, docs/pptx/xlsx/pdf skills.

**Missing:** no Rust, Tauri, networking or clipboard skill is installed; I work from the crate docs, Apple's and Microsoft's documentation, and the Tauri v2 guides.

## Status (Mission 03)

| # | Step | Status |
|---|---|---|
| 1 | Plan; rename (names, identifier, migration, service type, protocol version) | done |
| 2 | Resume interrupted transfers | done (real hardware untested) |
| 3 | Auto-update | done; tested end to end from the public repo (0.3.0 → 0.3.1 on this Mac, 2026-11-03) |
| 4 | Shared clipboard | done (macOS clipboard tested for real here; Windows clipboard tested on the CI runner; between two real machines: untested) |
| 5 | Signing hooks in CI + `docs/SIGNING.md` | done (the signing paths themselves untested: no accounts yet) |
| 6 | Fresh-eyes review; fixes; final summary | done (`docs/review-03/SUMMARY.md`, summary below) |

## Summary (Mission 03)

**Done**
- **Renamed to SeamDrop** (identifier `com.seamdrop.desktop`). The first start brings over Portal's settings, paired devices and keys, and leaves Portal's folder where it was. SeamDrop and Portal see each other and say which one to update (protocol 2). Checked on this Mac's real Portal data.
- **Transfers pause and resume** after a drop, a restart, a sleep or a quit, for up to 24 hours. They continue file by file and byte by byte, every file is still checked with BLAKE3, and a source changed meanwhile is sent again with a note. Cancel works while paused, and a paused transfer expires and cleans up after 24 hours.
- **Auto-update** from the public repo [seamdrop-releases](https://github.com/andrew2grand-lang/seamdrop-releases). Updates are signed; the private key lives only in GitHub secrets (and a local file for the owner to back up). SeamDrop checks at launch and every 4 hours, says "SeamDrop 0.3.1 is ready" with **Restart SeamDrop to update** / Later, and never restarts by itself or during a transfer. Failures are said in plain words. Releases come from tags `vX.Y.Z` (docs/RELEASING.md) and publish SHA-256 checksums and the update key's fingerprint.
- **Shared clipboard:**
  - **Send clipboard** in the window, the tray and a global shortcut you can change.
  - Text, rich text and images; anything bigger is sent as a file.
  - Automatic sync is off by default and runs only when **both** computers turn it on.
  - Anything an app marks as private is never sent.
  - History is the last 10 items, in memory only.
  - No clipboard contents ever go into logs.
- **Signing is ready but switched off.** CI signs as soon as the Apple or Azure secrets exist (docs/SIGNING.md).
- **Fresh-eyes review** (docs/review-03/SUMMARY.md): four personas and one engineering reviewer. Every *blocks use* and *confuses* finding is fixed and re-checked, except the two left to the owner (below). The headline fixes:
  - The Mac install leads with Open Anyway; Terminal is only for Macs without an administrator.
  - A Windows Firewall **Fix it** button.
  - Clipboard sync needs both computers to agree.
  - Cancel asks first; afterwards a Cancelled screen offers **Send again**.
  - Plainer paused, done and update wording.
  - A user-first README.
  - Seven resume bugs the engineering review found.

**End-to-end update, tested 2026-11-03.**
1. Tagged and built v0.3.0 and v0.3.1 in CI (macOS universal and Windows), signed with the updater key.
2. Published both with `npm run release:publish`.
3. Downloaded 0.3.0 from the public release page, checked its SHA-256 against `SHA256SUMS.txt`, and installed it to `~/Applications`.
4. Opened it with `--update-now`. Log: `SeamDrop 0.3.0 starting` → `update check: Ready Some("0.3.1")` → `installed 0.3.1; restarting` → `SeamDrop 0.3.1 starting` → `UpToDate`. The bundle on disk said 0.3.1 within about 15 s.

**Numbers**
- 287 Rust tests across the workspace, all passing here. CI runs the core and system crates on Windows too, including the real Windows clipboard and a real Windows Firewall repair.
- 80 UI tests.
- CI on `475b5e3` (0.3.1 plus two CI fixes for version bumps): green on macOS and Windows (ui, check, scan, build).
- Resume: a 4-file folder cut at the start, mid-file, between files and on the last chunk resumes every time. A 6-second prefix check doesn't stall the transfer, however long the check takes.

**Tested and works**
- Migration from Portal on this Mac's real data.
- The update itself, end to end from the public repo on this Mac (above).
- Bad signature, wrong version, offline and up-to-date, against a local feed (`src-tauri/tests/updater.rs`).
- Resume: two computers over loopback (drops, restarts of either or both, a changed source, a changed partial file, disk full, cancel on either side, expiry, a crash leaving stale state, a long prefix check). Also a scripted sender on a raw link for the exact interleavings (`tests/resume_wire.rs`).
- The real macOS pasteboard (this Mac) and the real Windows clipboard (CI runner): text, RTF, HTML, images and private markers.
- The Windows Firewall repair script on real firewall rules (CI runner, as administrator, no UAC prompt).
- Every screen in both windows and both themes against the mock core (`node scripts/real-shots.mjs`); key ones in `docs/screens/m03-*.png`.

**Written but untested on real hardware**
- Anything between a real Mac and a real PC: resume over Wi-Fi with a real sleep or lid close, clipboard both ways, sync consent, and the Portal ↔ SeamDrop version message. Covered over loopback only. Run docs/MANUAL-TESTS.md §0, §7, §7b, §7c and §9.
- The update on Windows (NSIS in passive mode, administrator prompt) and "never during a transfer" in the real app. The waiting logic is in `updates.rs`, but I couldn't start a transfer in the installed app without someone clicking. Manual test §7b.
- **Fix it** with a real UAC prompt (CI has no prompt) and answering No to it.
- Password managers marking their copies private: only the markers themselves were tested.
- Signing: no accounts yet.
- Flushing finished files before a checkpoint counts them: would need a power cut to test.

**Needs the owner**
1. **Back up the update key now.** Without it, installed apps can never be updated again. The files are `~/.tauri/seamdrop-updater.key` and `~/.tauri/seamdrop-updater.password` on this Mac.
   - Open your password manager and create a secure note called "SeamDrop updater key".
   - Paste the contents of both files into it. In Terminal, `cat ~/.tauri/seamdrop-updater.key | pbcopy` puts the key on the clipboard; then do the same for the `.password` file.
   - Keep the GitHub secrets as they are.
2. **Optional: let releases publish themselves.**
   - On GitHub go to Settings → Developer settings → Fine-grained tokens → Generate new token.
   - Repository access: only `andrew2grand-lang/seamdrop-releases`. Permissions: Contents → Read and write. Expiry: up to a year.
   - Copy the token, then run `gh secret set RELEASES_TOKEN --repo andrew2grand-lang/portal` and paste it.
   - Until then, `npm run release:publish -- vX.Y.Z` from this Mac does it (docs/RELEASING.md).
3. **Run the manual tests** on your Mac and PC: docs/MANUAL-TESTS.md §0 (from Portal), §7 (pause and resume), §7b (updates, including Windows and mid-transfer), §7c (clipboard, including your password manager: tell me which one), and §9 (Public network and **Fix it**). Install from https://github.com/andrew2grand-lang/seamdrop-releases/releases/latest; the Mac here already has 0.3.1 in `~/Applications`.
4. **After the move from Portal works on both computers:** quit and remove Portal. This Mac still has `~/Applications/Portal.app`, which I left alone. Then delete Portal's old data folder, which holds its old keys (README → Coming from Portal).
5. **Decide on the Project · Paid tab.** Every reviewer stumbled on it, but the mission said not to change Project Send. Options: hide it until it ships, move it into Settings, or drop the "Paid" badge.
6. **Decide on public networks.** The installer allows SeamDrop on Public networks so home Wi-Fi marked Public works. A "receive only on my home network" setting and a Pause switch were asked for (review C21).
7. **Signing, when you buy the accounts.** Apple Developer Program for the Mac; Azure Artifact Signing (UK organisations, not individuals) or a standard certificate for Windows. Steps and secret names are in docs/SIGNING.md; CI switches signing on by itself.
8. **Confirm the website domain.** Settings → About links to https://seamdrop.app, a placeholder until you confirm or buy it.
9. **Later, when every computer runs SeamDrop:** stop browsing for the old `_portal._tcp` service. One line in `crates/portal-core/src/consts.rs`.
10. **Annoyances and polish from the review** (docs/review-03/SUMMARY.md): none block use. The ones mentioned most:
    - The Details strip is always visible.
    - The accept screen doesn't show where files go or the free space.
    - There's no Pause button.

---

# Mission 02: Project Send design lab + real scanner

Plan: `docs/PLAN-02.md`. Spec: `MISSION-02-project-send-design-and-scanner.md`.

## Skills (Mission 02)

**Loaded and used:**
- Process: `superpowers:writing-plans` (PLAN-02), `superpowers:test-driven-development` (every rule: broken fixture first), `superpowers:verification-before-completion`, `superpowers:systematic-debugging` (when something fails), `engineering:architecture` (crate shape, decision log), `engineering:testing-strategy` (fixtures, golden files, clean set), `engineering:code-review` / independent reviewer at the end.
- Design (the lab): `impeccable` (context loaded: PRODUCT.md + DESIGN.md), `frontend-design`, `apple-design` (motion, macOS window conventions, reduced motion), `design:design-system`, `design:ux-copy`, `design:accessibility-review`, `web-design-guidelines`, `vercel-composition-patterns`, `playwright-cli` (screenshots).

**Available but not relevant:** GSAP / Lenis / react-bits (the design uses Motion), image-generation and brand skills, finance / legal / sales / PE / IB / small-business plugins, Vercel / Wix / Shopify deploy skills, the DJ skill, Notion / Canva / Figma connectors.

**Missing:** no Rust, Tauri or tree-sitter skill is installed; I work from the crate docs. No third-party skills installed (not asked for; supply-chain risk).

## Status (Mission 02)

| # | Step | Status |
|---|---|---|
| 1 | Plan + workspace (`portal-scan`, `design-lab`) | done |
| 2 | Scanner skeleton: walk, ignore rules, ecosystems, schema, golden tests | done |
| 3 | Snapshot: versions, packages, run commands, left-out folders | done |
| 4 | Rules + fixtures (Python, JS/TS, scripts, files, Docker, native, secrets, services) | done |
| 5 | Tier, headline, AI usage, hand-off note | done |
| 6 | `--online` PyPI wheel check | done (live PyPI: see untested) |
| 7 | Design lab shell | done (with 8) |
| 8 | Screens 1–9 (real scans for 1–3) | done |
| 9 | App fixes: "Files from <device>", minimum arrival moment | done (real hardware untested) |
| 10 | CI, README, screenshots, final summary | done |

Steps 2–6 were built and committed together: the snapshot, rules, tier and
online check share one report type and one fixture set, so splitting them
would have meant committing half a report.

## Summary (Mission 02)

**Done**
- **`portal-scan`**, a real, read-only project scanner (Rust, tree-sitter): `portal-scan <folder> [--target windows|macos] [--json] [--online]`. 42 rules in 15 categories, five honest states (Blocker, Will need a fix, Needs you, Couldn't check, Info), tier with its reason, a headline that never says "will work", an AI-use estimate, a hand-off note, versioned JSON with a published schema (`docs/scan-report.schema.json`). It never runs project code, never reads secret values, and only goes online with `--online` (PyPI's JSON API).
- **Design lab** (`npm run lab`, http://localhost:5190): all 9 Project Send screens, 37 states, in real Portal windows built from the app's own components and tokens, both window styles, both themes, reduced motion, 6 demo scenarios, and **Scan a real folder** that runs the real scanner and shows its live progress and report.
- **App fixes:** "Files from <device>: Ask every time · Always accept · Never" replaces Send / Both / Receive (enforced in the core, migrated from Mission 01, old builds still interoperate); near-instant transfers get a minimum arrival moment without a progress-bar flash.

**Numbers**
- Scanner speed (release build, this Mac): the Portal repo, 388 files, in 0.82 s; a 5,000-file project (3,750 parsed) in 0.51 s. Live `--online` check of 8 pinned packages: 0.6 s (cached afterwards).
- Tests: 33 broken fixtures, every planted problem found; 12 clean projects with zero Blockers and zero "Will need a fix" in both directions; 217 Rust tests across the workspace; 55 UI tests (incl. 20 lab tests that validate every rendered report against the schema).

**Screenshots** (`docs/screens/`): `lab-*` are the key lab screens, each in the window that shows it (Mac or PC) in light or dark; `app-*` are the two app fixes (Settings → Files from, the sender's "doesn't accept files" screen, the window without role keys, the arrival moment and the landing). Every lab state in both styles and themes: `node design-lab/scripts/lab-shots.mjs` (needs `npm run lab`).

**Written but untested on real hardware**
- Both app fixes between a real Mac and a real PC, and between a new build and a Mission 01 build (covered by core tests over loopback and protocol tests, not on two machines). See `docs/MANUAL-TESTS.md` §8 and §8b.
- The migration on the owner's own saved settings (tested on files shaped like Mission 01's).
- `portal-scan` and the lab on Windows by a person: CI builds and runs every scanner test on Windows and macOS runners (green), and the built CLIs are attached to the `scan` runs, but nobody has run the lab (`npm run lab`) on Windows.
- Conversion, verification, the repair agent, accounts and billing are design only, with demo data, by the mission's decision.

**Needs the owner**
- Install the new CI build on both computers and run `docs/MANUAL-TESTS.md` §8 (Files from …, including the update from Mission 01) and §8b (a tiny file still arrives visibly).
- A small one, not changed (outside this mission's two fixes): a near-instant transfer's done screen says things like "633 MB in 0.0 s (15813 MB/s)". Saying "in under a second" would read better; say if you want it.
- Look through the lab (`npm run lab`, or the screenshots) and say what to change. Open questions worth a decision: plan prices and the AI allowance (the lab uses placeholders: Pro $12 with $20 of AI use, Own key $5, a $5 gift); whether "Convert with Pro" should be the primary key on the free report.
- CI: GitHub's hosted macOS (arm64) runners twice didn't pick up jobs today ("not acquired… capacity constraints"), so those runs show as cancelled. Re-run, they passed: scanner tests on macOS and Windows, both app builds, the Windows core tests. If it happens again, re-run the cancelled job (Actions → the run → Re-run failed jobs).

**Independent review** (a separate reviewer read the code and the spec, not my notes; it reproduced the core, `.env`, link and FIFO findings directly). Everything it found is fixed. The core and scanner fixes each have a test that failed before the fix; the bridge got its assertions together with the fix; the timer and Cancel changes are UI behaviour, checked by hand, not covered by a test:
- Re-pairing over a connection that stayed open kept an old "Never" on the sender → both sides now send their answer when pairing completes (`pairing_again_forgets_an_old_never`).
- A multi-line quoted `.env` value (a private key) could leak one line as a "key name" → skipped (`env_files_give_names_only_even_with_multi_line_values`).
- Installed packages were read through links out of the project → refused (`installed_packages_are_never_read_through_links_out_of_the_project`).
- A FIFO named `.env` hung the scan; the shebang sniff opened secret files → regular files only, sniff only extensionless non-secret files (`a_fifo_never_hangs_the_scan`).
- A crafted `==version` could steer the `--online` cache file on Windows → names and versions must be plain (`only_plain_names_and_versions_reach_pypi_or_the_cache`).
- Going back to a Mission 01 build would lose pairings; a failed migration write stopped Portal starting → compatible field kept, best-effort save (`older_builds_can_still_read_the_saved_devices`, `a_migration_that_cannot_be_saved_still_opens_with_the_migrated_settings`).
- The arrival-moment timer could stop re-arming after a clock step; Cancel during the hold would dismiss a delivered transfer → clamped timer, no Cancel while held.
- The lab bridge accepted network paths and, without `Sec-Fetch-Site`, any origin → refused (lab tests).
The reviewer also checked and found fine: Never can't be bypassed on receive; Mission 01 wire compatibility; a second link restores the answer; migration twice is harmless; the moment doesn't replay; the scanner only runs `--version` and only contacts pypi.org with `--online`; odd input (binary, invalid UTF-8, 200k-deep folders, broken JSON/TOML/YAML) causes no panics; the bridge has no command injection.

## What works (tested, Mission 02)

**Scanner, steps 2–6 (2026-10-28)**
- `portal-scan <folder> [--target windows|macos] [--json] [--online] [--progress]`; the target defaults to the other OS. Text summary by default, versioned JSON (`schemaVersion: 1`) with `--json`. The schema in `docs/scan-report.schema.json` is generated from the report types; a test fails if it's stale.
- 42 rules in 15 categories (the MSR 2026 seven plus Copied environments, Scripts & line endings, Local AI hardware, Docker, Interpreter on Windows, Native apps, Secrets, Services). Code rules walk tree-sitter syntax trees (Python, JS/TS/TSX, Bash for shell scripts and npm scripts); no regex on code. Platform-guarded code (`if sys.platform…`, `try/except ImportError`, `hasattr`, `IS_WINDOWS`) raises nothing.
- 33 broken fixtures, each planting specific problems with an exact file, line and state in `expected.json`: every planted problem is found (`every_planted_problem_is_found`). 12 clean sample projects (Python CLI, FastAPI, multiprocessing done right, notebook, portable ML, Express, Vite + React, static site, docs folder, shell with twins, TS library, src-layout package) raise **zero** Blockers and zero "Will need a fix" in both directions.
- Safety tests: secret values never reach the report (key names only); nothing in the project is run (trap files in `setup.py`, `conftest.py`, `package.json` scripts, `Makefile`, `run.sh` would leave a marker); symlinks out of the project aren't followed; unreadable files are reported, not fatal; an empty folder gives an honest empty report.
- `--online`: tested against a fake PyPI (wheels for the target, other systems only, source only, missing, lookup failure). Without `--online` the report says "Offline: nothing was looked up on the internet." and wheel availability is "Couldn't check".
- Golden reports for four fixtures keep the JSON shape stable.
- Speed (release build, this Mac, M-series): the Portal repo, 388 files / 98 parsed, in **0.82 s** (most of it sizing the 28 GB `target/` folder it leaves out); a generated 5,000-file project (3,750 parsed) in **0.51 s**. Debug builds: about 1 s and 2 s.
- Scanning the Portal repo itself reports the planted fixture problems (MLX, Xcode, `fcntl`…): they're real files in the repo, so that's the right answer.
- `cargo test -p portal-scan`: 19 unit + 2 fixture + 12 report tests pass; `cargo clippy -p portal-scan --all-targets -D warnings` clean.

**Design lab, steps 7–8 (2026-10-28)**
- `npm run lab` (http://localhost:5190): every Project Send screen in real Portal windows, built from the app's own `AppWindow`, `PeerHeader`, `ModeSwitch`, `EdgeGlow`, `Details`, `Key`, `Chip`, `TextField`, `Checkbox`, stage `Pane`/`Head`/`Actions`/`Blockers` and `tokens.css`, imported. The shell and the screens were built together (step 7 is the frame the screens sit in), so they're one commit.
- 9 screens, 37 states: Project tab (drop, dragging over), Scanning (live demo, three frozen steps, real), Blocker report (answer, a finding opened, exported, real), Consent (run a command, send files to the AI), Conversion (planning, applying with diffs line by line, checking, AI repair attempt 2 of 3), Final report (answer, change review, a change undone, exported), Rebuild verdict (verdict, migration plan), Accounts and plans (sign in, account, plans, $5 gift, own key, allowance used up), Edge states (empty folder, huge folder, unreadable files, offline, AI budget used up, consent declined).
- 6 demo scenarios; every report is real `portal-scan` output for a demo project in `design-lab/projects`: local-chat (Python + Ollama, Mac → Windows: 5 fixes, 2 need you), shop-web (Node, npm scripts), ledger-tool (Windows tool → Mac: winreg, CRLF), Sketchbook (Xcode → Rebuild), InvoiceDesk (WPF → Rebuild, added), Handbook (documents: nothing to fix), plus an empty folder.
- Real scans: typed path or folder browser → the real scanner via the dev-server bridge, live progress on the Scanning screen, then the real report. Tested in Chrome: a fixture folder scanned and reported in 1.1 s (including a 0.7 s hold so the ticked steps are seen); a path that isn't a folder says so; no console errors.
- Screenshots (`node design-lab/scripts/lab-shots.mjs`): every state in both window styles in light (69 images) and the report, conversion, final and edge screens in dark (39) were reviewed in rounds; fixes from review: the tally strip (dropped, it repeated the headline), a seven-row pipeline (now one row), checks under a long diff list (now first), exports confirmed below the fold (now on the key), unreadable files at the bottom of the report (now first), copy that named the wrong computer in a forced window style.
- `npx vitest run design-lab`: 20 tests. Every report the lab renders (and the scanner's golden reports) matches `docs/scan-report.schema.json` under a strict validator that also rejects unknown properties (a drift test proves it fails); every mission screen and state is listed; both computers start a scenario; demo reports and hand-off notes contain no secret value; demo projects survive a Windows checkout; the bridge refuses relative paths, files and other sites.

**App fixes, step 9 (2026-10-28)**
- **Files from <device>** replaces Send / Both / Receive. Settings → Paired devices: "Files from MacBook Pro: Ask every time · Always accept · Never" (default Ask). The receiving core enforces it on every offer; the sender learns it through the existing `Status` message and stops before offering, with "Desk PC doesn’t accept files from this Mac" and the exact setting to change; it sends by itself once allowed again. Mission 01 builds keep working against it (they read Never as "set to Send").
- `cargo test -p portal-core --test incoming`: 9 tests (default Ask, Always skips the question, ticking Always on the offer, Never refuses with the sender told which setting, the receiver refuses even before the sender knows, allowing again resumes, Never only affects incoming, dismissing forgets, survives a restart). Mutation-checked: with the receiver's refusal disabled 4 fail; with the sender's early stop disabled the "never offered" check fails.
- Migration from Mission 01's saved files (`tests/store.rs`): Always accept → Always; a computer set to Send → Never for every device; Receive → Ask; saved once without the old fields. Protocol compatibility pinned in `tests/proto.rs`.
- **Arrival moment:** a near-instant transfer shows no progress bar, keeps the transfer screen with the edge light for at least 0.9 s, then lands with the flash; under reduced motion the same order with the still ring. `src/core/moment.ts`: 8 unit tests. Seen in Chrome with the mock core's new `instant` scenario: offer → "Receiving from MacBook Pro" without a bar → "Received" after the moment, in normal and reduced motion.
- Design click-through (`npm run verify`) passes with the new setting, no console errors. Workspace: 210 Rust tests pass, clippy clean; 55 UI tests pass; the UI ↔ Rust command contract checks out.

---

# Mission 01: the working core

## Skills

**Loaded and used this mission:**
- `superpowers:writing-plans` (docs/PLAN.md), `engineering:architecture` (architecture and decision log), `engineering:testing-strategy` (test plan)
- `superpowers:test-driven-development` (every core module: failing test first)
- `superpowers:verification-before-completion` (no claim without a fresh command run)
- Carried over from the design phase and still applied to the new screens: `impeccable`, `frontend-design`, `design:design-system`, `design:ux-copy`, `design:accessibility-review`, `web-design-guidelines`, `vercel-composition-patterns`, `apple-design`, `playwright-cli`

**Available but not relevant here:** GSAP/Lenis/react-bits (animation libraries; the design uses Motion), image-generation and brand skills, finance/legal/sales/PE/IB plugins, Vercel deploy skills, Notion/Canva/Figma connectors.

**Missing:** no Tauri or Rust skill is installed. I didn't install third-party skills (supply-chain risk, not asked for); I worked from the Tauri and crate docs instead.

---

## Status by step

| # | Step | Status |
|---|---|---|
| 1 | Plan + skeleton | done |
| 2 | Discovery + Connect by IP | done |
| 3 | Pairing + trusted devices + settings | done |
| 4 | Transfer | done |
| 5 | Cross-OS file safety | done |
| 6 | Roles end to end | done |
| 7 | Link detection + cable | done; cable deprioritised in Update 1 |
| 8 | Connection doctor | done (Windows probes: CI smoke only) |
| 9 | Background running | done (notifications untested) |
| 10 | CI + README + manual checklist | done |
| U1 | Update 1: Wi-Fi first, cable silent, managed Mac | done (real hardware untested) |
| U2 | Update 2: send list (collect, then Send) | done |
| U3 | Update 3: thinking orb on transfers, Disconnect | done |

## What works (tested)

**Step 1 (2026-10-24)**
- Cargo workspace: `crates/portal-core` (no Tauri), `crates/portal-sys` (OS glue), `src-tauri` (app). `cargo test -p portal-sys -p portal-core`: 18 tests pass on macOS (no-replace rename incl. a 16-thread race, private 0600 key file, DPAPI/secret round trip, identity create/load/regenerate, settings + trusted-peer store). `cargo clippy -D warnings` clean on macOS; `portal-sys` also cross-checked for `x86_64-pc-windows-msvc` from the Mac.
- `npx tauri build --debug --bundles app` produces an ad-hoc-signed `Portal.app` (bundle id `com.project1000.portal`); the built `Info.plist` contains `NSLocalNetworkUsageDescription` and `NSBonjourServices = [_portal._tcp]`; the app launches, shows its window and quits cleanly. Main executable LC_UUID `059DE253-…` (unique per build).
- In the app the UI renders one window for this machine (native traffic lights on macOS; undecorated window with working caption buttons on Windows). Checked in Chrome with a stubbed Tauri bridge for both OS layouts and both themes (`scripts/native-shots.mjs`): no console errors.
- Windows CI (`check.yml`) green: DPAPI, `MoveFileExW` no-replace rename incl. the race test pass on a real Windows runner.

**Step 2 (2026-10-24)**
- TLS 1.3 only (ring, no `tls12` feature), mutual certificates, pinned fingerprints compared in constant time, every handshake signature verified with `verify_tls13_signature`. 7 tests incl. forged server and forged client certificates (right fingerprint, wrong key) being rejected; a mutation check (signature check disabled) makes those tests fail, so they really test it.
- Protocol: framed JSON control + raw data frames, versioned hello, unknown messages ignored, oversized/unknown/truncated frames rejected (8 tests).
- Link: TLS + hello exchange, session exporter secret, reader/writer tasks; transfer messages stay in order with file data (a test caught and fixed control messages overtaking data) (5 tests).
- Discovery: mDNS (`_portal._tcp`, host `portal-<id>.local.`) + UDP announce to directed broadcasts, 255.255.255.255 and `ff02::1` per interface, immediate unicast replies (3 tests + an opt-in real-mDNS test that passes on this Mac in 0.9 s).
- Node: two cores on loopback find each other by broadcast in under 5 s, Connect by IP works, connecting to ourselves is refused, role changes reach the peer (6 tests). Connect-by-IP parsing incl. `fe80::…%en7` zones (4 tests). Interface classification incl. idle macOS `utun`s not counting as VPN (10 tests).
- **Real app on this Mac:** the built app advertises `_portal._tcp` (seen with `dns-sd -B`), listens on 47920 IPv4+IPv6; the headless peer finds it via mDNS and IPv6 link-local broadcast and the app finds the peer; Connect by IP from the peer to the app's Wi-Fi address establishes an authenticated link.
- UI: real screens (searching, still looking + suggestions, nearby, offline, needs-update, Connect by IP, cable how-to, settings, Project "coming soon") screenshotted against the mock core for macOS and Windows layouts, light and dark (`scripts/real-shots.mjs`, 44 shots, no console errors). Design demo still passes all 16 click-through checks (`npm run verify`). 12 vitest tests for the core→UI mapping.

**Step 3 (2026-10-24)**
- Pairing by numeric comparison: the TLS server commits to its nonce first, the code is SHA-256 over the TLS exporter, both session fingerprints and both nonces (scheme in `docs/PLAN.md` §4, code in `crates/portal-core/src/pairing.rs`). 7 unit tests: same code on both sides, every input changes it, role swap changes it, codes spread over all six digits, commitments only open with their own nonce, rate limits (3 per 10 min per device and per address, 60 s cool-down after a cancel).
- 6 end-to-end tests between two cores: the same code shows on both screens and one confirmation isn't enough; trust survives a restart of both apps and they reconnect with no code; cancelling on either side pairs nobody and tells the other side; removing a device revokes trust at once and the other side is told; a device whose key changed is refused (`security key changed` in Details); an unpaired device can't immediately raise another prompt after a decline. Ran 8× in a row without a failure.
- Untrusted devices: only hello, status and pairing messages are accepted; anything else gets `not_paired`.
- Trusted devices are re-dialled automatically (pinned to their key) from fresh sightings or last-known addresses, with backoff (1 s → 15 s); the device with the larger id waits 1.5 s so both rarely dial at once.
- Settings: device name (re-announced over mDNS and to connected peers), receive folder (native folder picker), skip system files, open at login (autostart plugin), trusted devices list with rename, always-accept and remove (two-step).
- UI screenshots against the mock core: pairing code (asker and responder), waiting for the other side, paired, removed-by-the-other-side, settings, both OS layouts and themes. 15 vitest tests.

**Step 4 (2026-10-24)**
- Transfers end to end (14 integration tests, two cores on loopback): a 20 MB file with its modified time; a 6-level folder with an empty folder, contents and folder times identical; **5,000 small files in 2.3 s** (pipelined: one stream, no round trip per file); a 256 MiB file; existing names never overwritten (`notes (2).txt`, `Photos (2)`); decline sends nothing; always-accept skips the prompt; cancel on either side cleans up both; the other computer vanishing mid-transfer is reported and cleaned up; not enough free space is refused before anything moves; a disk filling mid-transfer stops cleanly with "ran out of space" on both sides; unpaired devices can't send; symlinks skipped and counted on both sides.
- Security: a trusted but malicious peer sending `..`, `a/../../x`, `a\..\..\x`, `C:`, empty and `.` components is refused with a protocol error, nothing lands outside the receive folder, staging is removed.
- BLAKE3 per file on both ends while streaming; files stage in `<receive>/.portal-incoming/<transfer>/` (hidden on Windows) and each top-level item moves into place with a no-replace rename only after the whole transfer verifies; crash leftovers are removed at start.
- **Throughput:** release build, 2 GiB over loopback on one Mac (both ends encrypting, hashing and using the same SSD): **330 MB/s** — about 3× gigabit's ceiling, so the pipeline isn't the limit for the ≥ 90 MB/s cable target.
- A real bug found and fixed on the way: the receiver's writer thread could keep handling queued commands after a failure and recreate the staging folder after cleanup; the writer now stops first, then cleanup runs.
- UI: offer in/out, sending/receiving with live progress and edge glow, done (time, average speed, notes, "Show in Finder" / "Show in File Explorer" revealing the first item), failed, declined, the three role-blocked states, and a refused send — screenshotted for both OS layouts and themes against the mock core. 21 vitest tests. `npm run check:bridge` checks every UI command matches a registered Rust command and its arguments (16/16).

**Step 5 (2026-10-24)**
- `crates/portal-core/src/fsafe.rs` holds every rule; 13 rule tests (`tests/fsafe_rules.rs`) + 5 module tests: Windows forbidden characters `< > : " | ? *` and control characters → `_`; reserved names checked case-insensitively before the first dot (`NUL.tar.gz`, `COM¹ COM² COM³ LPT¹ LPT² LPT³`, `AUX .txt`) while look-alikes (`CONSOLE`, `COM10`) are left alone; trailing dots/spaces removed; NFC everywhere (NFC-only changes aren't reported as renames); 255 UTF-16 units on NTFS and 255 UTF-8 bytes on APFS, cut on a character boundary keeping the extension, and re-checked after ` (2)`; collisions after sanitising detected with case-folding + NFC; wire-path validation; junk list.
- End to end, a Mac sending to Windows rules: `Résumé: Q3?.pdf` (stored NFD, as macOS does) arrives as `Résumé_ Q3_.pdf`; `CON.txt` → `CON_.txt`; `trailing dot.` → `trailing dot`; `a:b.txt` and `a?b.txt` → `a_b.txt` and `a_b (2).txt`; a 120-character CJK name passes untouched on NTFS. Details logs each rename with its reason ("‘:’ and ‘?’ aren’t allowed in Windows names"), the done screen says "7 names changed so they work on Windows", and the sender hears the count too.
- Each skipped link or system file gets its own Details line (up to 50, then a count); done screens say what kind ("2 links and 3 system files").

**Step 6 (2026-10-24)**
- Roles are enforced in the core, not the UI (logic built in step 4, tests added here; a mutation check that disables the checks makes 5 of the 7 tests fail). Default is Both, persisted, and sent to the peer on every change.
- A device set to Receive can't start a send: the core plans what was dropped (so the screen can list it), sends no offer, and shows "This Mac is set to Receive" with Switch to Both / Not now.
- A device set to Send refuses offers with a `role_send` reason: the sender shows "Desk PC is set to Send. On Desk PC, switch to Receive or Both. Sending starts as soon as it does."; the receiver shows "MacBook Pro is trying to send" with the items. Switching either side sends automatically with no new drop; "Not now" forgets it (no surprise send later).

**Step 7 (2026-10-24)** (superseded in part by Update 1: a direct link is now silent and shown as Ethernet)
- Path logic is pure and unit-tested (`crates/portal-core/src/node/paths.rs`, 7 tests): a link-local peer over a wired interface = "Cable · offline"; wired through a router = "Ethernet"; link-local over Wi-Fi stays "Wi-Fi"; ranking cable IPv6 link-local > cable 169.254 > Ethernet > Wi-Fi; only a strictly better kind triggers a switch; a duplicate link is kept only if it isn't worse.
- Between transfers, a linked peer is re-dialled over a strictly better path (e.g. a cable just plugged in; at most every 10 s per peer); the device with the smaller id keeps the better of the two links and closes the other, never the one carrying a transfer. Details logs "switched to the cable  works offline".
- Dialling: IPv6 link-local carries our interface's scope id; IPv4 169.254 binds the source address and `IP_BOUND_IF` (macOS) so it can't leave over Wi-Fi. On this Mac, a scoped dial to `fe80::1%lo0` works and the unscoped one fails; an interface-bound dial leaves from the bound address (2 tests).
- mDNS on this Mac advertises the IPv4 address and the `en0` link-local IPv6 address with its scope (checked with `dns-sd -G v4v6`); a stray `fe80::1` from the loopback interface was being advertised and is now excluded.

**Step 8 (2026-10-24)**
- `crates/portal-core/src/doctor.rs`: a pure `diagnose(facts)` with 15 fake-fact tests. Checks: on a network (Wi-Fi / Ethernet / a cable); same network as the paired device was last time; **macOS Local Network** inferred as *likely denied* from "no route to host" on local dials (cleared by any working local dial), with an "Open Settings" deep link and the tip that Connect by IP from the PC still works; **Windows** firewall Block rules for our exe (fail, with how to delete them; Block beats Allow), our Allow rule missing / partial / present, each connected network's Public/Private profile (Public is fine when our rule covers it; otherwise how to switch to Private), **Controlled folder access** when the receive folder is under Documents/Desktop; VPN only when a tunnel has IPv4 or the default route; **router isolation** as a *likely* cause after 10 s with a router, nothing seen and last-known addresses silent; "Portal is open on the other computer" as unknown. Problems sort first.
- Windows probes in `portal-sys` (COM: `INetworkListManager` profiles per adapter, `INetFwPolicy2` rules for our exe, Controlled Folder Access from the registry) cross-compile from the Mac; smoke tests run on the Windows CI runner.
- The node feeds the doctor real facts (dial results, last-known addresses, refused writes, Windows probes every 15 s) and puts the checklist in every snapshot. UI: "Still looking", "isn't reachable" and the Connection check panel show it with Try again / Connect by IP / Use a cable, plus per-check buttons (opener allow-list: `x-apple.systempreferences:`, `ms-settings:`, `windowsdefender:`). Fixed on the way: the rescue buttons weren't docked below the scrolling checklist.

**Step 9 (2026-10-24)**
- Closing the main window hides it; the app stays alive (last-window exit is prevented) and "Quit Portal" in the menu bar / tray is the only real exit. macOS: clicking the Dock icon brings the window back. A second launch focuses the running instance (single-instance plugin), or opens the waiting offer.
- Menu bar (macOS template icon, adapts to light/dark) and system tray (Windows, coloured): a status line ("Ready", "Desk PC wants to send", "Receiving from Desk PC · 45%", "Looking for your other computer"…), the paired device with its link ("Desk PC · Cable · offline"), "Review incoming files…" (enabled only when an offer waits), Open Portal, Quit Portal. The icon gets a dot while an offer waits. On Windows a left click opens the app, or the offer if one waits.
- Native notification for an incoming offer, a finished receive and a failure — only when the main window isn't visible and focused. Clicking it opens the compact accept window: macOS through an app-activation observer (`NSApplicationDidBecomeActiveNotification`), Windows through the toast's activation callback (installed builds; dev runs use PowerShell's identity as Tauri's docs warn).
- Compact accept window (`index.html?view=accept`, 440×560, always on top): the offer, "Always accept from…", Accept / Decline; it closes itself when answered, withdrawn, or handled in the main window. Screenshotted in both OS layouts/themes.
- "Open at login" (autostart plugin, off by default) launches with `--hidden`.
- macOS Local Network prompt is triggered at launch (UDP datagram to a link-local IPv6 address on each interface, repeated twice, per Apple TN3179).
- Checked here: the debug app builds and runs hidden with the tray and no errors. Not checked: anything that needs eyes or clicks on the real windows (no Screen Recording / Accessibility permission in this session).

**Step 10 (2026-10-24)**
- `build.yml` (push to `main` + manual): UI checks on Linux; macOS universal `.app` + `.dmg` and Windows x64 NSIS installer via `tauri-action`, uploaded as run artifacts. First run green: https://github.com/andrew2grand-lang/portal-archive/actions/runs/40093187469 — artifacts `Portal-darwin-universal-dmg`, `Portal-darwin-universal-app`, `Portal-windows-x64-nsis`.
- After the security fixes, everything ran green again: build https://github.com/andrew2grand-lang/portal-archive/actions/runs/40094467400 (UI checks, macOS universal, Windows x64; same three artifacts) and check https://github.com/andrew2grand-lang/portal-archive/actions/runs/40094467206 (all core tests on Windows, incl. the new ghost-link test). The informational step there also passed: two Portal daemons found each other by mDNS on a Windows runner, alongside Windows' own mDNS service on port 5353.
- The CI step that checks the built Mac app confirms `NSLocalNetworkUsageDescription` and `NSBonjourServices = [_portal._tcp]` in its Info.plist, bundle id `com.project1000.portal`, an ad-hoc signature, a universal binary (x86_64 + arm64) and per-architecture LC_UUIDs.
- The CI-built DMG was downloaded here, mounted (with the Applications shortcut), and the release app started from it, logging to `~/Library/Logs/com.project1000.portal/portal.log`.
- `README.md` (install steps incl. Open Anyway / SmartScreen / admin prompt, dev, build, tests, limitations) and `docs/MANUAL-TESTS.md` (brief §4, step by step).
- Details shows "middle 80 %" speed after big transfers (the brief's measure); a log file is written on both OSes.

**Independent security review (2026-10-24)** — a separate reviewer read the core and found:
1. *Critical, fixed:* trust was keyed on the device id a peer claims, not on its key. A device that read a peer's id from mDNS and connected before pairing kept that connection after the real pairing, and could push files or receive the victim's sends. Now a link is trusted only if its certificate matches the pinned key; pairing closes every other link claiming that id; duplicate-link handling compares links with the same key only. A regression test reproduces the attack (it failed 3/3 before the fix, passes now).
2. *Medium, fixed:* unlimited unauthenticated connections → capped per address and overall, idle ones closed after 2 min (test).
3. *Low, fixed:* pairing limiter keyed on a free-to-change id and never shrinking → keyed on key + address, expired entries evicted (test).
4. *Low, fixed:* a failure while moving items into place could leave earlier items in the receive folder → rolled back (test).
The reviewer found path validation, the pairing state machine, constant-time pinning and the staging/verify/rename pipeline sound.

**Update 1: Wi-Fi first (2026-10-24)** (The owner's update: cable deprioritised, Wi-Fi is the product, managed Mac)
- *Cable is silent.* No screen, notice, address, doctor advice, tray text or doc mentions cables; the "Use a cable" panel is gone. The direct-link code still runs: a direct link is used if there is one, and shows as "Ethernet". A test checks no doctor advice says "cable".
- *Bug fixed for the main setup* (PC wired to the router, Mac on its Wi-Fi): the PC classified the Mac's link-local IPv6 address as a direct cable, labelled the link "Cable · offline" and kept trying to "upgrade" to it. A wired interface now counts as a direct link only without a router, and through a router IPv4 is tried first (unit tests: the routed-LAN case failed before the fix).
- *Reconnecting is fast even with stale addresses:* addresses are dialled in parallel, 250 ms apart, first handshake wins. Test: a dead address that accepts and stays silent, listed first, held reconnecting up for **12.1 s** before; now **2.3 s** (`tests/wifi.rs`).
- *Network changes:* a new IPv4 address on Wi-Fi/Ethernet (checked every 3 s) or waking from sleep (wall clock jumps ahead of the monotonic clock, which stops on macOS while asleep) makes the core re-announce, re-dial at once and ping every link; a link that says nothing for 6 s is closed and re-dialled. Tests: the "slept" rule and "IPv6 privacy rotation isn't a move" (unit); a link through a relay that goes dead is replaced by a direct one **7.0 s** after the change (integration; it fails with the probe switched off).
- *A device back at a new address* (DHCP) reconnects without pairing, in 0.04 s once it's up (integration test).
- *Discovery when the router passes neither multicast nor broadcast between wired and Wi-Fi:* while alone, Portal sends its announce by unicast to every address of its own small private network (/24 or smaller; never on bigger, public, router-less or VPN networks), after 4 s, then less and less often; the other side's unicast reply is already tested. Unit tests for which networks are swept; on this Mac (home Wi-Fi, 192.168.1.200/24) a lone peer starts sweeping after 4 s ("asking every address on this network directly"). Broadcast announces now go out every 5 s (was 10 s).
- *Wi-Fi drop mid-transfer:* after 3 s with nothing moving, both windows say "Connection interrupted. Waiting for it to come back…" (speed 0, "Waiting" instead of time left). When the link is found dead, both stop with "The connection to Desk PC dropped at 31%. Nothing was saved. Send it again …", also as a notification on the sender now. Integration test through a relay that drops out at 25 %: both show "stalled", both stop with that message, nothing partial is left (it fails without the probe).
- *Throughput on Wi-Fi-like links* (relay adding latency and a bandwidth cap): 2,000 files over a 50 ms round trip arrive in **0.6 s** (no round trip per file; one each would take 100 s); a 16 MB/s link with a 20 ms round trip carries **15.8 MB/s** (99 %). Socket buffers deliberately stay with the OS (fixed sizes turn auto-tuning off). The 100 ms progress timer now only runs during transfers.
- *Keep awake:* while a transfer runs, macOS gets a user-initiated activity (no idle sleep, no App Nap) and Windows a `PowerRequestSystemRequired` request. Tests: taken/released repeatedly and from another thread; on this Mac `pmset -g assertions` lists it while held and not after (Windows: type-checked and clippy-clean from this Mac; runs in Windows CI).
- *Managed Mac (no admin, no Xcode):* this Mac is one (Mosyle MDM, standard user). The CI-built app, copied into `~/Applications`, with a browser-style quarantine flag added and then cleared with `xattr -dr com.apple.quarantine` (no password), started from there, listened on 47920 (IPv4 + IPv6), ran mDNS and broadcast discovery, and wrote its log and data under the home folder. "Open Anyway" needs an administrator's password, so the README now uses the `xattr` route. The Apple Development certificate recommendation is gone.
- *Local Network permission off:* per Apple's TN3179, accepting incoming connections isn't covered by it and MDM can't manage it; so Connect by IP from the PC to the Mac is the fallback, and the PC keeps reconnecting to the Mac's address by itself afterwards. The doctor's fix now names this Mac's address; pairing or Connect by IP from the Mac says macOS is blocking it instead of "couldn't reach" (unit test for when that applies).
- *Fixed on the way:* the ✕ on panels (Settings, Connect by IP, Connection check) had slid to the left edge, half outside the card (CSS order); screenshots before/after.
- Screenshots (both OSes, both themes) of the changed screens: stalled transfer, failed transfer, searching, still looking, Connect by IP, Settings, PC on Ethernet. `docs/MANUAL-TESTS.md` rewritten for home Wi-Fi with the PC on Ethernet: Local Network fallback, sleep/wake, new networks, router restart, Wi-Fi drop mid-transfer, and the LocalSend comparison in both directions.
- Totals: 162 core + sys tests pass on this Mac, clippy clean (macOS; portal-sys also for Windows), 24 UI tests, lint, types, bridge check, design demo check.
- CI for this update, all green: check https://github.com/andrew2grand-lang/portal-archive/actions/runs/40107581916 (every core test on Windows, incl. the Wi-Fi tests with the same results as here, e.g. a dead link replaced in 7.0 s, 2,000 files over a 50 ms round trip in 2.7 s, and the keep-awake power request; two daemons found each other by mDNS next to Windows' own) and build https://github.com/andrew2grand-lang/portal-archive/actions/runs/40107581948 (artifacts `Portal-darwin-universal-dmg`, `Portal-darwin-universal-app`, `Portal-windows-x64-nsis`).
- That CI Mac build, installed the README way on this managed Mac (quarantine flag added, then cleared without a password), ran from `~/Applications` and from `~/Downloads`; a headless peer found it by mDNS on all its addresses; hidden and alone it started its sweep after 4 s; idle wake-ups dropped from about 70 to about 7 a second. Test copies removed afterwards.

**Update 2: the send list (2026-10-24)** (The owner: "let me press Send; files accumulate, then I send in bulk")
- Dropping files or folders, or **Add files** / **Add a folder**, now adds them to a "Ready to send" list instead of sending. Each item shows its size (folders: file count and size, counted by the core the way a send counts), can be removed, and the title shows the totals. **Send** sends everything as one transfer; **Clear** empties the list. A refused send keeps the list; drops during a transfer join it ("1 more item waiting…").
- Fix that came with it: two items with the same name from different folders in one send used to merge (folders) or fail (files, on the receiver's no-overwrite rule). The sender now numbers them ("Photos (2)"). Integration test: two "Photos" folders and `notes.txt` / `NOTES.txt` from different folders all arrive intact (failed before the fix).
- Tests: `describe` unit tests (counts like a send, skips junk when set, links and missing paths aren't sendable), UI test for the list in the world, bridge check (17 commands). Screenshots, both OSes and themes: the list, counting (Send disabled), after Send ("Waiting to accept" with every item), refused (list kept).
- `docs/MANUAL-TESTS.md`: new test 12 (collect from several places incl. a same-named file, remove one, one offer, numbering, drops during a transfer, Clear); the other tests now say to press Send.

**Update 3: thinking orb and Disconnect (2026-10-25)**
- Thinking orb: one 32 px orb beside the transfer percentage (working / weaving), still while a transfer is stalled. A first pass that put orbs on every busy state was reverted at the owner's request; searching, pairing and waiting are back to the spinner and tiles.
- **Disconnect** (on "isn't reachable" and in Settings → Paired devices; **Connect** to undo): stays paired, stops connecting either way, remembered across restarts. The other side is turned away during the TLS handshake (no "Connected" flicker), backs off to one try a minute, and its connection check says who disconnected and that pressing Connect there fixes it. Sending to a disconnected device says to press Connect. Tests (`tests/disconnect.rs`): the link is cut and nothing reconnects (the other side keeps trying and is turned away), the pairing survives, the other side's message, the send message, still disconnected after a restart, Connect reconnects with no code and clears the message; doctor unit test; UI presence test. Screenshots: "isn't reachable" with Disconnect, Disconnected, Settings connected and disconnected, both OSes and themes.
- Totals: 169 core + sys tests, clippy clean, 26 UI tests, bridge check (18 commands), design demo check.

## Written but untested on real hardware

- Notifications and their click-through on both OSes, the menu bar / tray menu, close-to-hide and Quit, the accept window opened from a notification.
- Windows: DPAPI key protection, `MoveFileExW` no-replace rename, computer name, undecorated window + caption buttons, NSIS hooks that add/remove the firewall rules (needs the CI build).
- macOS: native window corners vs the glow ring radius (16px assumed; this session can't take screenshots of real windows: no Screen Recording permission).
- Discovery between a real Mac on Wi-Fi and a real PC on Ethernet through a home router (mDNS, broadcast, the unicast sweep): tested between processes on this Mac and on one Windows CI runner, never across two machines or a router that filters multicast.
- Real sleep/wake, switching Wi-Fi networks, a router restart and a Wi-Fi drop mid-transfer: the logic is tested with a relay that goes dead and with simulated clock jumps, not with a real Mac going to sleep.
- Real Wi-Fi throughput and the LocalSend comparison (the relay tests prove the protocol keeps a slow, high-latency link busy; they can't measure Wi-Fi itself).
- The keep-awake request actually holding off sleep during a long transfer (the assertion is visible in `pmset`; Windows only type-checked here).
- Whether App Nap slows a hidden, idle Portal enough to matter (a running transfer is protected; idle links rely on 10 s pings with a 30 s timeout).
- The Local Network-off path (Connect by IP from the PC to the Mac) between two real machines.
- Direct cable / Thunderbolt links: still in the code, used silently if present; untested and no longer a goal.

## Needs the owner

1. **Run the manual checklist** in `docs/MANUAL-TESTS.md` at home, PC on Ethernet and Mac on the router's Wi-Fi (not on school Wi-Fi): install, finding each other in 5 s, the Local Network-off fallback (Connect by IP from the PC), pairing, 10 GB + 5,000 files both ways, sleep / new network / router restart, a Wi-Fi drop mid-transfer, roles, Public network, notifications with the window closed, `Résumé: Q3?.pdf`, and speed against LocalSend. It says exactly what you should see and what to send back if not.
2. **Installing needs you once per machine.** Mac (no administrator): copy Portal into the Applications folder in your home folder, run one Terminal command (`xattr -dr com.apple.quarantine ~/Applications/Portal.app`), then click Allow when macOS asks about your local network. PC: SmartScreen "More info → Run anyway" and the administrator prompt (it adds the firewall rules). Steps are in `README.md` → Install a test build. If the school has blocked Terminal on the Mac, tell me: there's no other way to open an unsigned app without an administrator.
3. **GitHub Actions minutes.** The repo is private: macOS minutes count 10×. One `main` build costs roughly 150–250 of the free 2,000 minutes a month. If builds stop, either run them only on demand (Actions → build → Run workflow) or make the repo public. Your call.
4. **Allow notifications** when macOS first asks (needed for test 10).

## Next step

The owner runs `docs/MANUAL-TESTS.md` on the real Mac and PC. Mission 02 (resume) builds on this.
