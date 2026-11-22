# Mission 04 plan: solid desktop, many devices

Spec: `MISSION-04-solid-desktop-many-devices.md`. Build order is the spec's §12.
Every step ends with the Rust and UI suites green, a commit, and (at milestones)
CI green on macOS and Windows. Decisions go to `docs/DECISIONS.md` (167 on),
progress and "Needs the owner" to `docs/PROGRESS.md`.

## Global constraints (from the spec)

- Desktop only. Nothing may assume two devices, or one Mac and one PC.
- The look stays: fix what's broken, build new screens from the existing
  components, tokens and voice. Honest by design: no bare spinners, plain
  words, never made-up information.
- No password guessing: only the private markers apps set; marked content is
  never read (decision 146 stands).
- **One protocol version bump** (2 → 3) covers every wire change in this
  mission. Old and new refuse each other with "Update SeamDrop on <device>".
- Spend nothing. CI budget: heavy suites (browser traces, overlap
  screenshots, four-peer tests, phone checks) run on pushes to `main` and on
  manual runs only.
- Every bug in §1: cause found in the code (file:line) before the fix, and a
  test that fails before it. Every result labelled: "CI on real
  macOS/Windows", "loopback between processes", or "untested on real hardware".

## How the work flows (CI budget)

- Work on branch `mission-04`. Pushing the branch runs the cheap checks only
  (`ui` on Linux; `check` on Windows when crates change).
- Merge to `main` (fast-forward) at milestones: after steps 2–3, 4, 5, 6, 7,
  8 and at the end. Each `main` push costs about 80 billed minutes (macOS
  build ×10), so about 8 of them. `scripts/ci-minutes.mjs` sums the mission's
  minutes from the job timings for PROGRESS.md.

## Step 1: plan (this file), skills, decisions

## Step 2: connecting at launch (§1.1)

**Causes to confirm (and test red first):**
- `buildWorld.ts:presenceOf` turns paired-not-online into `offline` at once.
- `PeerView` has only `online`.
- `trust.rs` back-off 1 → 2 → 4 → 8 → 15 s, reset only by a never-seen path
  (`mod.rs:on_sighting`). A fresh sighting of a known path doesn't dial.
- Sweep at 4 s, then 30 s (`netwatch.rs`); announce burst 10 s (`discovery.rs`).

**Core: reach state per paired device** (`node/reach.rs`, new):
`Reach { since, attempts, failures, last_failure, next_try, step }`, reset on
launch, wake, network change, Connect, Try again and when a link drops.
`PeerView.reach: Option<ReachView { state: connecting | unreachable,
since_ms, attempts, last_failure, next_try_ms, step: looking |
trying_known | trying_found }>`. `unreachable` only after `REACH_GRACE`
(20 s) **and** at least 2 failed dials; the UI never uses its own timers.

**Retrying:** during the first 20 s of a connecting period dial every 1 s
(no exponential back-off), then 2, 4, 8, 15 s. A fresh sighting of a paired,
unconnected device dials at once (at most once a second). Announce burst 30 s
after start / wake / change; sweeps at 2, 6, 12, 20 s while alone, then the
existing 30 s, 1, 2, 4, 5 min.

**UI:** "Connecting to Desk PC…" with what's happening ("Looking on this
network", "Trying the address it was last seen at"), "isn't reachable" only
from the core's `unreachable`. Try again stays.

**Tests:**
- Core unit/integration (in-process): reach is `connecting` before the grace
  and `unreachable` after it with ≥ 2 failures (short grace via `Config`).
- **Two `examples/peer.rs` processes** (`tests/launch.rs`, loopback only,
  `#[ignore]`, run on `main` CI and locally): start offsets 0, 3, 8, 20 s,
  both orders; variants: discovery off (stored addresses only), one stale
  stored address, a listener that starts late (`--listen-after`). Assert both
  online within 5 s of the later one being up. Must fail on today's code
  (expected: the stale-address variant at 8 and 20 s).
- `peer.rs` gains `--loopback`, `--udp-port`, `--announce-to`, `--listen-after`
  and prints `ONLINE <name> <ms>` lines the test reads.

## Step 3: time left and speed (§1.2), the glow (§1.3), paused (§1.4)

**Speed/ETA** in the core: `transfer::estimate::Estimator` (pure, fed
`(elapsed, bytes)`), one per transfer.
- Source: the receiver's acknowledged bytes (`progress {bytes}` every 250 ms,
  already sent; the sender ignores them today) on the sender, received bytes
  on the receiver. Stored in `ProgressState::acked`.
- Window 10 s of time-stamped samples; reset at every (re)start, so resume
  and "checking" jumps never count.
- Speed shown at most every 2 s, 2 significant figures.
- Time left from the windowed speed, smoothed again (time constant 8 s),
  shown with hysteresis and a 20 s minimum between changes; "Working out
  time left…" for the first 5 s and after a resume. Ladder: About N minutes
  (≥ 90 s) · About a minute · Less than a minute · About N seconds (10 s
  steps) · A few seconds. Exact thresholds tuned by the trace tests.
- `ProgressView { speed: Option<f64>, eta: Eta }`; the UI only words them.
- **Trace tests** (Rust): ±40 % Wi-Fi swings, a stall, a burst, a resume.
  Shown value never A → B → A within 60 s; ≤ 3 changes in any 60 s; reaches
  "A few seconds" without bouncing. Red first against today's formula
  (ported as the baseline).

**Glow:** a pre-painted conic layer rotated by a Web Animations API animation
of `transform` (compositor), inside the ring mask; lap speed changes with
`updatePlaybackRate` (no jump). Receiving fill: `--glow-fill` updated at
most 4× a second in 1° steps. Same look; reduced motion unchanged.
- Proof (`scripts/glow-trace.mjs`, Playwright): a glow-only page
  (`?glow=sending`), devtools trace for 3 s: no glow paints after the first
  second; DrawFrame events keep coming during a 500 ms main-thread block.
  Chromium on Linux (ui workflow, `main` only) and `channel: 'msedge'` on the
  Windows runner. Red first against today's glow.

**Paused:** the orb and glow stop; a still pause mark sits next to the
percentage; "3.2 GB of 10 GB" stays. Stalled shows the same still mark
(movement means work). Replaces 77 / 134.

## Step 4: many devices (§3) — data model first, then screens

**Core**
- `transfers: Vec<Transfer>` keyed by `tid`; at most one *active* (preparing,
  offered, running, paused) per device; parallel across devices. Every
  command names its transfer: `accept(tid, always)`, `decline(tid)`,
  `cancel(tid)`, `dismiss(tid)`, `send_again(tid)`; `send(devices, paths)`
  makes one transfer per device and says per device what didn't start
  ("Desk PC is busy with another transfer …").
- Wait on quit, the updater's wait, `upgrade_links`, duplicate-link
  resolution, `cmd_set_incoming`, `peer_back`, expiry: per transfer.
- **Free space reserved** across incoming transfers (offer needs total +
  margin; free space minus what running/paused incoming transfers still need).
- **Resume keeps every paused transfer** at start (no newest-only); with one
  device, paused transfers continue one after another.
- **Clipboard per pair:** consent per device (`TrustedPeer.clip_sync`,
  `clip_sync_at`; the old global `clipboard_sync` migrates to every paired
  device), the other's consent per link; sync with a device = both said yes.
  Off for a pair mirrors on both (as 155). "Turn off clipboard sync" turns it
  off with everyone. Auto copies go to every device sync is on with; incoming
  auto copies only from those. `send_clipboard(devices)`.
- Remembered choice: `Settings.send_to: Vec<DeviceId>` (shortcut and tray use
  it).
- Snapshot: `transfers: Vec<TransferView>`, per-transfer progress events.

**Shell:** accept window shows one offer at a time, oldest first ("1 of 2");
tray lists devices with status, sends to the remembered choice; updater waits
for any active transfer.

**UI model:** devices keyed by id; `Side` stays only as the window's chrome
(the OS it runs on). `World { me, devices[], target[], sessions[] … }`,
progress per transfer. The simulated two-window demo (`SimProvider`,
`reducer`) goes: the mock core drives every state (`/?mock=…`), and `/`
lists the scenarios. The design lab keeps a frozen copy of the two-window
model it needs (it designs Project Send between two computers; out of
scope here).

**Screens** (existing components): with one paired device it looks like
today. With more: a device list (status: connected · connecting · not
reachable, and the link), ticks to choose where files and the clipboard go
(remembered), one row per transfer with its own progress, pause and cancel.
Settings → Devices: per device "Files from …", where its clipboard goes,
sync, rename, Remove (says what it means).

**Tests (loopback, `tests/many.rs`, four in-process peers plus a
four-process smoke run of `examples/peer.rs` on `main`):** pair all; send to
two at once; receive from two at once (space reserved); drop one mid-transfer
and resume while the others carry on; restart with two paused (both survive);
clipboard to one and to several; auto-sync with two, no ping-pong.

## Step 5: small windows (§1.5), Details (§1.6), appearance (§1.7), folder (§1.8)

- **Overlap test first** (`scripts/overlap.mjs`, Playwright): every mock
  scenario × sizes 420×640 (raise the minimum if needed), 552×780, 800×900,
  1280×900 × both themes × 1 and 6 devices. Mac chrome in WebKit on a macOS
  runner, Windows chrome in Edge on a Windows runner (`main` only). Asserts
  no two visible text boxes overlap and nothing is clipped; exclusions up
  front (aria-hidden, sr-only, mid-animation: waits for animations to end).
  Screenshots to `docs/screens/m04-*`. Then fix what it finds.
- Clipboard messages get one fixed place (a status line above the action row)
  that fades and never moves the layout.
- **Details:** closed by default; a small Details key with the terminal icon;
  no last-line preview. Open: a calm story from real events. `LogLine` gains
  `detail` (technical, shown dimmed after the plain words); identical lines
  in a row collapse ("×12, last at 19:31"); tones reviewed (amber only when it
  affects you, red only when it failed and needs you). Copy technical log.
- **Appearance:** Settings → Appearance System · Light · Dark (core setting,
  so the tray can switch it too); tray submenu.
- **Received folder:** done screens say where and offer Show in folder (and
  Change folder on the first arrival); Settings shows the full path with
  Open / Change; optional "A folder for each device" (this computer's name
  for the device, made safe with `fsafe`, numbered on clashes, kept when the
  device is renamed: `TrustedPeer.folder`).

## Step 6: pairing by typed code (§2)

**PAKE:** SPAKE2 per RFC 9382, ciphersuite SPAKE2-P256-SHA256-HKDF-HMAC,
implemented in `portal-core::pake` on the `p256` crate's arithmetic (0.14.0,
July 2026) with `ring`'s SHA-256/HKDF/HMAC; the RFC's four test vectors pass.
The RustCrypto `spake2` crate follows magic-wormhole (not RFC 9382) and has no
independent audit, so it's not used (recorded).
- Roles: A = TLS client, B = TLS server (M and N per the RFC). No symmetric mode.
- `w`: HKDF-SHA256 of the six digits (salt "SeamDrop pair v1 w"), 40 bytes,
  reduced mod n (64 bits more than n, as the RFC advises). No MHF: online
  guessing only, capped.
- Identities: `"SeamDrop pair v1 A" ‖ exporter ‖ fpA` and `… B ‖ exporter ‖
  fpB`; AAD = exporter ‖ fpA ‖ fpB (in the confirmation-key derivation).
- Confirmation both ways (KcA ≠ KcB), HMAC compared in constant time.
- Messages (v3): `pair_ask`, `pair_ready`, `pair_start {s}`, `pair_answer
  {s, mac}`, `pair_key {mac}`, `pair_wrong`, `pair_result {ok, tries_left}`,
  `pair_stop`. Old `pair_request/commit/nonce/reveal/confirm` decode as
  unknown and are ignored; nothing falls back.
- Limits: 3 tries per shown code; 10 failed runs across all requesters lock
  pairing on the showing computer (saved) until its person presses **Allow
  pairing again**; prompts rate-limited per key, per address and overall;
  2-minute timeout. Ignore sends nothing.
- UI: requester picks a device → "Waiting for Desk PC to show a code"; the
  other shows "MacBook Pro (192.168.1.20) wants to pair · Show code / Ignore",
  then the code; the requester types it (6 boxes). Both: "Paired with …" and
  a notification.
- Tests: right code; wrong code on both sides, retries and the global cap;
  MITM with two TLS sessions fails on both; reflected confirmation rejected;
  old messages refused; Ignore, timeout, cancel on either side; a 0.3.1
  pairing (fixture data folders saved before any change) still connects.

## Step 7: clipboard (§4)

- **Shelf:** everything sent to this computer, with Copy and the device;
  memory only. Per device: incoming clipboard goes "Straight to the clipboard
  (and the Shelf)" (default) or "Shelf only". A small text box: type or
  paste, pick devices, Send (`send_text`), lands on their Shelf (and
  clipboard per their setting).
- **Files on the clipboard** (`Read::Files(paths)`; macOS file URLs, Windows
  CF_HDROP): "2 files on the clipboard · Send them to Desk PC" starts a normal
  transfer to the chosen devices. Images stay images.
- **Named marker:** `Read::Private { marker, app }` (the format's name; the
  owner's process name on Windows when known). Plain message without "we
  detected"; Details names the marker. No Send anyway.
- **Browsers on the Windows runner** (`scripts/clip-browsers.mjs`): Firefox
  normal + private, Edge and Chrome normal + InPrivate/incognito
  (`launchPersistentContext` for normal), Ctrl+C and
  `navigator.clipboard.writeText`; a small Rust probe lists the formats set.
  Findings in DECISIONS and MANUAL-TESTS.
- **Pause / off:** tray "Pause clipboard sync ▸ 1 hour · Until SeamDrop
  restarts" (local only, consent kept) and "Turn off clipboard sync"; the one
  privacy line in Settings → Clipboard and when sync is first turned on.
- **Stress tests** (in-memory clipboard): fast repeated copies, very long
  text, emoji and scripts, rich text, images, copying during a transfer
  (Send clipboard allowed), the other app closed / device asleep, sync with
  two devices without loops, marked content. The rest in MANUAL-TESTS.

## Step 8: small things (§5), protocol and phones (§6)

- Project · Paid tab hidden behind a developer setting (off).
- `src/links.ts` + `crates/portal-core/src/links.rs`: website
  `https://seamdrop.com`, feedback `feedback@seamdrop.com`, `CONFIRMED =
  false`: nothing opens or mails them until it's true; About shows "not live
  yet"; Send feedback saves the log and says feedback isn't set up yet.
- Send feedback in About and Details (mailto with version + OS once
  confirmed; "Include technical log" saves a file and reveals it).
- Review 03 "annoys": keyboard shortcuts on the accept window (Return accepts,
  Esc declines), the accept screen says where files go; the rest listed.
- `docs/PROTOCOL.md`: discovery, pairing (with vectors), messages, transfers
  and resume, clipboard, relay notes. `cargo check -p portal-core` for
  `aarch64-linux-android` and `aarch64-apple-ios` in CI (`main` only);
  desktop-only parts of `portal-sys` behind `cfg`. iOS/Android notes in
  DECISIONS.

## Step 9: LocalSend bridge — only if everything above is green

## Step 10: fresh-eyes review (§11), fixes, final summary

Four persona subagents with only "SeamDrop moves files and your clipboard
between your computers." → `docs/review-04/SUMMARY.md`; fix "blocks use" and
"confuses", re-check, list the rest; MANUAL-TESTS ~20-minute check; PROGRESS
summary with CI minutes and Needs the owner.

## Review focus (inputs no test above names yet)

1. A device renamed on one side while transfers with it are paused: names in
   paused rows and folders stay consistent (folder kept, name updated).
2. Removing a device with a running or paused transfer: it ends cleanly and
   its partial files go.
3. Two offers arriving at the same moment from different devices with only
   enough space for one: the second is declined with the space message.
4. A pairing request arriving while this computer is itself typing a code to
   another device: one pairing at a time, the second waits silently.
5. Clipboard sync on with three devices, one asleep: copies keep flowing to
   the other two; nothing queues up for the sleeper.
