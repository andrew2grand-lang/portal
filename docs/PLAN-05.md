# Mission 05 plan: clear, calm, and a real Shelf

Spec: `MISSION-05-clear-calm-and-the-shelf.md`. Build order is the spec's §12.
Decisions go to `docs/DECISIONS.md` (240 on); progress, step times, CI
minutes and "Needs the owner" to `docs/PROGRESS.md`.

## Global constraints (from the spec)

- **One job per place:** Send (home: files and folders), Clipboard (copy here,
  paste there; sync; recently received), Shelf (kept text you put there on
  purpose). Settings holds settings, Help holds explanations, nothing else
  carries paragraphs.
- **Writing rules:** titles ≤ ~6 words; at most one helper line ≤ ~12 words,
  only where it prevents a mistake; "not done" keeps its reason in one short
  sentence, plus a button when there's a fix. No sizes or types on screen
  (Details only).
- **Honest by design**, briefly. No bare spinners, no made-up numbers.
- **No password guessing.** Only markers apps set. Decision 146 is amended
  for Put on Shelf only (read on click, only if the change count still
  matches).
- **Protocol 4** for every wire change; old and new refuse each other with
  "Update SeamDrop on <device>". **No build on the owner's computers until every
  protocol-4 message below is final.** `_portal._tcp` browsing goes.
- The look stays in the family: existing tokens and components.
- Spend nothing. Never a stable release; the mission ends at 0.5.0-beta.1.

## Speed rules (spec §0.6)

- Branch `mission-05`. Pushing it runs only the cheap checks (Linux `ui`,
  Windows `check` when crates change). Don't wait for them between steps.
- CI checked at three milestones only: after step 2, after step 4, final.
  Heavy suites run on `main` (fast-forward at the milestones) and manual
  runs. Minutes summed per milestone for PROGRESS.
- While building: only the tests for what changed. Full Rust and UI suites
  at the end of each step.
- Step times recorded in PROGRESS. Should-haves (step 7) are skipped and
  listed if the mission has run more than ~6 hours by then.

## Protocol 4: every message, decided up front

Decided here so no build reaches a real computer with a half-finished wire
(decision 204: unknown messages are silently ignored).

| Message | Direction | Meaning |
|---|---|---|
| `Ping` (existing) | both, every 1 s | Liveness. Any bytes read count as alive; a link closes after 3 s with nothing read (not while this side is the one holding the reader back). No `Pong` needed, but it's still answered |
| `Hold { tid, on }` | either side of a paused transfer | "Stopped partway here; waiting for the person here to continue" (`on: true`), or released (`on: false`, which starts the resume at once) |
| `Progress { tid, bytes }` (existing) | receiver → sender, also while paused | What the receiver has kept for a paused transfer (its checkpoint), so a held sender can show a percentage that's known |
| `QueueHead { at }` | both, when it changes and on link up | The earliest time (unix ms) something was queued here for you; `None`: nothing |
| `ShelfShare { on, at }` | both | Consent to share the Shelf with this device, like `ClipSync` (newest change wins) |
| `Shelf { items }` | both | Shelf records: on link up, all records whose origin is me or you; later, each change |
| `Picture { picture }` | both, on link up and when it changes | This device's picture: a PNG (≤ 256×256, ≤ 64 KB) or a colour and icon, or none |

`PROTOCOL_VERSION` 3 → 4; a protocol-3 peer is told to update, as today.

## Step 1: plan, skills, decisions (this file)

## Step 2: §1 fixes and the nearby list → **CI milestone 1**

Each: cause found (file:line) written to DECISIONS before the fix, a test that
fails first.

1. **§1.1 receiving glow.** `EdgeGlow.tsx` fill mode → the same compositor
   comet as sending, turning the other way (recorded). Paused: still. 108's
   instant fill → the arrival moment (§1.6). Test: `EdgeGlow` mode tests
   (no `--glow-fill` writes while receiving).
2. **§1.2 liveness.** `net/link.rs`: a counting reader marks `heard` on every
   read of any size; a link-level pinger sends `Ping` every 1 s; a watchdog
   closes the link after 3 s with nothing read, except while the reader is
   blocked handing data to a full transfer sink (our own disk, not the
   network). Node pings (`PING_EVERY`) go. `STALLED_AFTER` 3 → 2 s and the
   window hides speed and says "Waiting for <device>…". Tests (loopback, a
   TCP proxy in the test that can freeze one direction without closing,
   20/20): cut → Paused ≤ 3 s on the survivor (each OS in CI); 300 KB/s never
   pauses; a slow re-hash (the `fault_hash_rate` hook, 2 GB at a rate that
   takes > 10 s) never pauses.
3. **§1.3 time left.** Confirm whether the post-resume ramp is real (the
   estimator restarts in `resume.rs` before the sender's re-hash, so the
   quiet sits in the window). Restart the estimator when the first new byte
   moves, show "Checking what already arrived…" during the re-hash. New
   `estimate.rs` rules in the spec's priority order. Accuracy test replays
   traces (throttled loopback recordings incl. a real resume, and synthetic
   jitter): within ±25 % or ±15 s from the first shown estimate.
4. **§1.4 quit vs drop.** `journal.rs`: `stopped: Quit | Crash | Network |
   Update` (missing = crash). Quit writes `quit`, the update restart writes
   `update`, a drop writes `network`; a kill leaves a running record =
   crash. On launch, quit/crash → held: the window asks "Sending '…' to X
   stopped partway. [Continue] [Cancel]"; the other computer gets `Hold` and
   shows "Paused: waiting for <device> to continue" with its own Cancel.
   Tests: quit → asks, kill → asks, drop → continues, update → continues,
   Cancel either side cleans both.
5. **§1.5 busy and the queue.** Core per-device queue (`node/queue.rs`):
   paths queued for a busy device, saved to disk, each removable; when the
   slot frees, `QueueHead` decides who goes first (earliest, then lower
   device id) and the winner offers as usual. Window: "Busy sending to X." /
   "Next for X: 2 files · starts when the current transfer finishes";
   nothing says "Ready to send" when it can't. Tests: both queued at once;
   three devices.
6. **§1.6 receiving done.** Same tick and moment as sending, "Received 10 GB
   from X" naming the file / "3 files" / the folder; Show in Finder/File
   Explorer (primary) + Done. **Open:** documents, images and media only
   (never apps, installers, scripts, shortcuts, archives) — chosen over
   marking files, recorded. Notification when not in front.
7. **§1.7 segmented controls.** One rule: hug the options. Audit Settings and
   panels. Before/after screenshots `docs/screens/m05-*`.
8. **§1.8 + §2.2 nearby list.** With nothing paired (and whenever pairing is
   possible), list every nearby computer with name, OS, address and Pair;
   never pick one. After a pairing, back to the list with that one marked.
   Check real discovery (Mac↔Mac multicast, the sweep). Test: three peers,
   nothing paired, all listed (loopback) + buildWorld test.
9. **§1.9 sync missed a copy.** Mark a change seen only once read or given up
   after retries; a give-up says "Couldn't read that copy. Try Send
   clipboard." Test with a clipboard that's busy for the first reads
   (fails before).

## Step 3: §3 Clipboard and Shelf

Core first:
- **Shelf store** (`shelf.rs`): records `{id, origin, name, text, private,
  stamp, by, removed}`; stamps `max(now_ms, last + 1)` with device-id
  tie-break (decision 156). Tombstones kept 400 days (≤ 2,000). Limits 200
  live items, 64 KB each. Encrypted at rest (ChaCha20-Poly1305 via `ring`),
  key in the platform secret store: DPAPI on Windows; on macOS test the
  Keychain under ad-hoc signing and record (else a 0600 key file, and say
  what that does and doesn't protect).
- **Sharing:** `ShelfShare` consent per pair (asked at pairing and once for
  existing pairings); `Shelf` records: on link up the records whose origin is
  me or you; each change to its origin's peers (my items → all sharing
  peers; an edit of X's item → X only, which re-broadcasts its own). Never
  relayed. Unshare/unpair: their items leave here. Checks: count, size, ids,
  stamps > 1 day ahead refused.
- **Clipboard:** incoming always to the clipboard (`clip_to` goes); recently
  received (last 5, memory only, images with Save as file); not-synced
  notices once per copy (private → Put on Shelf, too big → Send as file,
  unreadable → Send clipboard); Put on Shelf reads the marked copy only on
  click and only if the change count matches.
- Tests: triangle and chain (add/edit/remove with one offline, converge, no
  relaying), consent off on one side, kept across restart, a removed item
  never returns, no plaintext of private items in the data folder, the
  WebView folder or logs.

Screens: header keys Clipboard, Shelf, Settings; the Clipboard place; the
Shelf place (one subheading); one sync switch per device (Clipboard only).

## Step 4: §4 received files, §6 Help and writing rules, §7 systems → **CI milestone 2**

- Settings → Received files: Straight into a folder (default Downloads) / In a
  SeamDrop folder (optionally per device) / A new numbered folder each
  delivery ("SeamDrop 012 – from X", the next number from what's there).
  Migration as specified, said once. Files and folders dated on arrival;
  "Keep original dates" off by default. First arrival says where, with Change.
- Help page (in Settings, "?" in panels), searchable, the eight sections.
  Every screen audited; word counts before/after in PROGRESS; screenshots.
- Supported systems from `tauri.conf.json` and the builds; Help and README.

## Step 5: §8.2 beta tooling, checked locally first

- `vX.Y.Z-beta.N` tags → pre-releases on `seamdrop-releases` and a fixed beta
  feed URL compiled in; the stable feed untouched. Settings → About → Get
  test builds (off). With it on, check both feeds and take the higher
  (default comparator, two checks). Off never downgrades.
- Check bundling of a pre-release version on macOS locally, and the NSIS
  version rules from the bundler source, before any CI minutes.

## Step 6: §11 fresh-eyes review

Three personas (non-technical two-laptop, developer with keys, three
computers and a big interrupted transfer), only the one-line description.
`docs/review-05/SUMMARY.md`; fix every "blocks use" and "confuses".

## Step 7: should-haves (time limit)

§2.1 removal choices, §2.2 "+", §2.3 device pictures (core message and
checks are already in step 3 so protocol 4 is final), §5 mini mode.

## Step 8: final CI milestone, local install, beta.1, summary

Release build locally → `~/Applications/SeamDrop.app`, relaunch, check
About. Tag `v0.5.0-beta.1` (pre-release). Final summary with the §10 list
and Needs the owner.
