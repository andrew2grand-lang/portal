# Mission 03 plan: SeamDrop, launch-ready free app

Spec: `MISSION-03-seamdrop-launch-ready.md`. Build order is the spec's §9; every
step ends green (Rust tests, UI tests, CI on macOS and Windows) and committed.

## Global constraints (from the spec)

- Name: **SeamDrop** (capital S, capital D). One constant: `productName` in
  `src-tauri/tauri.conf.json` (the UI reads it as `APP_NAME`; Rust reads
  `package_info().name`; the core gets it through `Config`).
- Identifier `com.seamdrop.desktop`. Repo and crate names stay.
- Spend nothing. Signing accounts come later (CI ready, switched off).
- The look stays: new screens use existing components, tokens and voice. No bare
  spinners; plain words; say what's happening and why.
- Project Send (scanner, lab) changes only where it shows the product name.

## Step 1: rename

**Names.** Every user-visible "Portal" → "SeamDrop": window titles, tray,
installer (NSIS firewall rule names), notifications, keep-awake reason, core
messages and doctor checks, settings, README, docs, design lab, the default
receive folder (`Downloads/SeamDrop`). Log file `seamdrop.log`. Internal names
(`PortalWindow`, `portal-core`, the `portal:event` channel, `portal_*`
commands) stay: no user benefit.

**Wire-level names that stay** (renaming them would make old and new unable to
even say "update"): TLS ALPN `portal/1`, the pairing exporter label, the UDP
announce magic `PORTAL1`. These are protocol ids, not product names.

**Identifier + migration** (`portal_core::migrate`, called by `Node::new`
before the identity loads, when `Config::migrate_from` is set; the shell sets it
to `<app data parent>/com.project1000.portal`):
- Runs only when the new folder has no identity yet and the old one has one.
- Copies the identity key, `settings.json`, `peers.json` (files only; private
  files stay private: 0600 on macOS, DPAPI blobs are per-user on Windows so a
  copy still opens). The old folder is never touched.
- A saved receive folder equal to the old default (`Downloads/Portal`) becomes
  `Downloads/SeamDrop`; any other choice is kept. Earlier files stay put.
- Logged in Details: what moved, from where, that the old folder is left.
- The shell also moves "Open at login" from the old app (if it was on, it's
  turned on for SeamDrop and the old login item is removed), best effort.

**Discovery.** Advertise and browse both `_seamdrop._tcp` (new) and
`_portal._tcp` (old) this release; host name `seamdrop-<id>.local.`.
`NSBonjourServices` lists both. Remove `_portal._tcp` in a later release
(DECISIONS).

**Protocol 2.** Resume and clipboard change the wire, so this release speaks
protocol 2. Old and new refuse each other at the hello (as today) and the new
app says which side to update: "Update SeamDrop on Desk PC" (theirs older) or
"Update SeamDrop on this Mac" (theirs newer). The snapshot carries `me.proto`;
a dial or an incoming hello that fails on version marks that peer incompatible
(today only discovery did). A peer with a live compatible link isn't flipped to
incompatible by a stale sighting.

**The old app still running on the same computer** (same device id after
migration): a sighting of our own id on another port or protocol raises a
doctor check: "The old Portal app is still open on this Mac. Quit it from the
menu bar and move it to the Trash."

Tests: migration (copies, keeps old, receive-folder rule, runs once, nothing
to migrate, unreadable old files), incompatible text both ways, own-id
sighting check, both service types advertised (unit test on the service list).

## Step 2: resume

**Behaviour.** A drop (link closed or silent) pauses instead of failing. Both
sides show "Paused: lost connection to <device>. Will continue when it's back."
with "3.2 GB of 10 GB". When a trusted link to the same device comes back, the
transfer continues by itself (no new prompt). Survives either app restarting
and sleep. Expires 24 h after it paused: partial data removed, both sides say
so. Either side can Cancel while paused (partials removed; the other side
learns on reconnect). Terminal failures (disk full, access denied, damaged
file, unreadable source, protocol error) stay terminal and clean up, as today.

**State on disk** (`<app data>/transfers/<tid>.json`, written atomically):
- Sender (`out`): peer, what was dropped, the offer, dirs and file list with
  the size and modified time seen when it was planned, paused-at time.
- Receiver (`in`): peer, the offer, staging folder, name rules, the namer's
  state (so names stay stable), folder times, how many folder entries and
  files are done, the partial file (index, local path, bytes written, BLAKE3
  of those bytes), paused-at time. Written by the writer thread every second
  and on pause, so it only ever records bytes that are really on disk.
- A cancelled or expired transfer leaves a small tombstone (24 h) so the other
  side hears "cancelled" or "expired", not "unknown".

**Wire (protocol 2).**
- `resume_ask {tid}` sender → receiver when a trusted link comes up.
- `resume_from {tid, dirs, file, offset}` receiver → sender (on link up, or in
  answer to `resume_ask`): folder entries received, files done, bytes of the
  partial file it trusts.
- `file {…, from}`: the byte the data starts at (0 unless resuming).
- Unknown, cancelled or expired tid → `cancel {tid, error: expired|unknown|cancelled}`.

**Integrity.** On resume the receiver re-hashes the partial file's trusted
prefix and compares it with the hash it recorded; a mismatch (the file changed
on disk) restarts that file from 0. The sender re-hashes the source prefix it
skips; if the source's size or modified time changed since planning, it
restarts that file from 0 with a note ("“report.pdf” changed on this Mac, so
it's being sent again from the start"). Every file still gets the whole-file
BLAKE3 check before anything moves into place. Only the same paired device
(pinned key) can resume; "Files from: Never" set while paused cancels it.

**Partial area** `.seamdrop-incoming`; old `.portal-incoming` folders are
removed at start (old builds kept no resume state).

Tests (core, two nodes, relay to cut the link): drop at start, mid-file, last
chunk, between files of a folder; restart sender / receiver / both; source
modified while paused; partial modified on disk while paused; disk full while
resuming; cancel while paused (both sides, online and offline); expiry; an old
`.portal-incoming`; hashes match and nothing partial is left.

## Step 3: auto-update

- `tauri-plugin-updater` (Rust side only; the UI gets our own events). Updater
  key pair made with `tauri signer generate`; public key in `tauri.conf.json`;
  private key and password only as GitHub secrets (`TAURI_SIGNING_PRIVATE_KEY`,
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) set with `gh secret set`; backup steps
  under Needs the owner.
- Public repo `andrew2grand-lang/seamdrop-releases`: README only, plus
  releases holding installers, updater bundles and `latest.json`. Endpoint:
  `https://github.com/andrew2grand-lang/seamdrop-releases/releases/latest/download/latest.json`.
- `release.yml` on tag `vX.Y.Z`: checks the tag matches the app version,
  builds macOS universal + Windows NSIS with `createUpdaterArtifacts`, and
  publishes to the public repo with `RELEASES_TOKEN` (a fine-grained token for
  that one repo: Needs the owner). Without the token it still builds, and
  `npm run release:publish -- vX.Y.Z` publishes from a Mac with `gh` logged in.
- Versions: one source (`scripts/version.mjs X.Y.Z` sets Cargo, Tauri and npm
  versions). This release is 0.3.0. Notes come from `CHANGELOG.md`.
- In the app: check 10 s after launch and every 4 h, quietly. Install
  automatically (default on) = download in the background, then the calm
  notice "SeamDrop 0.4.0 is ready. What's new: …" with **Restart to update** /
  **Later**. Off = the notice offers **Update and restart**. Never restarts
  while a transfer runs: it says "Restarts when the transfer to Desk PC
  finishes" and does so then (a paused transfer doesn't block: it resumes after
  the restart). Settings → About: version, Check for updates (with its result in
  words), Install updates automatically, website link. Failures in plain words;
  the current version keeps running.
- Tests: the update check against a local test feed (newer, same, bad
  signature, offline) with a committed test-only key. Manual end to end on this
  Mac: install 0.3.0, publish 0.3.1 to the public repo, see the prompt, update.

## Step 4: clipboard

- **Send clipboard**: tray item, a key in the main window, a global shortcut
  (macOS ⌃⌥⌘C, Windows Ctrl+Alt+Shift+C; changeable in Settings; a taken
  shortcut says so). Receiver puts it on its clipboard and shows "Clipboard
  from MacBook Pro: “meeting notes…” — ready to paste."
- **Automatic sync** (off by default): one setting for the pair (turning it on
  or off on either computer does so on both, newest change wins), copies sync
  within about a second, a visible "Clipboard sync on" indicator in the window
  and the tray while it's on.
- Content: plain text, RTF (both OSes use RTF natively), HTML, images (PNG on
  the wire). Limits: images 25 MB as PNG, text 8 MB; bigger goes as a normal
  file transfer with a note. History: the last 10 received, memory only, with
  Clear and "Copy again".
- Privacy: never read content that's marked sensitive. macOS: the
  nspasteboard.org types (`org.nspasteboard.ConcealedType`, `TransientType`,
  `AutoGeneratedType`) plus 1Password's legacy marker. Windows:
  `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`
  = 0, `CanUploadToCloudClipboard` = 0, `Clipboard Viewer Ignore`. Checked
  before and after reading (the change counter must not move in between).
  Received content carries our own marker so it never echoes back. Logs say
  only kind and size.
- Wire: `clip {id, kinds, total}` + `clip_part` chunks (≤ 1 MiB) on the
  control channel; `clip_sync {on, at}`. Paired, trusted links only.
- Tests: the pure parts in Rust (limits, chunking, marker rules, echo
  suppression, sync-setting merge, history cap) and the node round trip (text,
  rich, image, empty, huge, over-limit → file, sensitive refused, sync on/off,
  peer offline); the OS clipboard adapters by hand (MANUAL-TESTS, incl. a
  password manager copy that must not arrive).

## Step 5: signing hooks

`build.yml` / `release.yml`: macOS Developer ID + notarisation when the Apple
secrets exist (Tauri's names), else ad-hoc as today; Windows signing when
Azure Artifact Signing or certificate secrets exist, else unsigned.
`docs/SIGNING.md`: what to buy, cost, secrets, how to check.

## Step 6: fresh-eyes review

Four isolated persona agents (beginner on a Windows gaming PC, non-technical
Mac user, privacy-minded user, developer moving big files daily) get only
"SeamDrop moves files and your clipboard between a Mac and a Windows PC.",
the app's design canvas with its mock states, and the README. Findings →
`docs/review-03/SUMMARY.md`; fix every "blocks use" and "confuses", re-check,
list the rest.

## Review focus (inputs no step's tests exercise by default)

1. Two SeamDrops sharing an identity (old app still running): must say so, not flap.
2. A drop while the receiver is moving finished items into place: the move is
   local and finishes; resume must not resend a transfer that completed.
   (`complete` lost on the wire: the sender asks to resume, the receiver
   answers from its tombstone "already done".)
3. A resume racing a new offer: a paused transfer holds the one transfer slot.
4. Clipboard ping-pong between two machines with sync on.
5. The update arriving mid-transfer and the person pressing Restart.
