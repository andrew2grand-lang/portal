# Mission 03: SeamDrop, launch-ready free app

**Product:** **SeamDrop** (replaces the working name "Portal")
**Owner:** the owner
**Goal:** make the free Mac ↔ Windows app ready for other people: rename to SeamDrop, resume interrupted transfers, auto-updates, a shared clipboard, and CI ready to sign the apps as soon as I add the accounts.

Read this whole file before doing anything. Work in the existing repo.

---

## 0. How to work (same rules as before)

1. **Skills first.** List every skill you have; load and use all that are relevant (Tauri, Rust, networking, security, frontend/UI design, motion, accessibility, planning, test-driven development, debugging, verification, code review). List them at the top of `docs/PROGRESS.md`.
2. **Work autonomously. Don't ask me questions.** Decide, record it in `docs/DECISIONS.md` with a one-line reason, keep going. Anything only I can do goes under "Needs the owner" in `docs/PROGRESS.md`, with exact steps.
3. **Plan, then build.** Plan in `docs/PLAN-03.md`, then start.
4. **Small, verified steps** in the order of section 9. Tests pass at each step. Commit after each step. CI stays green on macOS and Windows.
5. **Honest reports.** Separate "tested and works" from "written but untested on real hardware". Red-first tests for bug fixes where practical; say when something was only checked by hand.
6. **At the end, run a fresh-eyes review** (section 8) before calling it done.

### Decisions already made
- **Name:** SeamDrop, written exactly like that (capital S, capital D).
- **Focus:** the free transfer app. Project Send (scanner, design lab) stays as it is; don't change it beyond the rename where its UI shows the product name.
- **Money:** spend nothing. Signing accounts are bought by me later (section 5).
- **The app's look stays.** New screens (resume states, update prompt, clipboard) use the existing components, tokens and voice.
- **Honest by design** still applies: no bare spinners, plain words, say what's happening and why.

---

## 1. Rename to SeamDrop

- Replace "Portal" with "SeamDrop" everywhere a user can see it: window titles, menu bar and tray, installers, notifications, settings, docs, README, the design lab.
- Keep the product name in one constant so it's easy to change.
- **Bundle identifier:** change from `com.project1000.portal` to **`com.seamdrop.desktop`** (it must not end in `.app`). Nobody outside my two machines has installed it, so now is the only cheap moment to change it.
  - On first launch of the renamed app, **migrate my existing settings, paired devices and keys** from the old app-data folder, so I don't have to re-pair. Leave the old folder in place (don't delete) and log the migration in Details.
  - Expect macOS to ask for Local Network permission again (new identity). Say so in `docs/MANUAL-TESTS.md`.
- Discovery: change the service type to `_seamdrop._tcp`. During the migration build, also browse the old `_portal._tcp`, so a renamed Mac can still find an un-updated PC (and the reverse). Remove the old one in a later release; note it in `docs/DECISIONS.md`.
- Protocol: bump the protocol version if anything on the wire changes; old and new must refuse each other with a clear "Update SeamDrop on <device>" message, never a silent failure.
- The repo name and internal crate names can stay; don't churn code for no user benefit.

---

## 2. Resume interrupted transfers

People send big files over Wi-Fi; transfers will drop (Wi-Fi blips, a laptop sleeping, the app closing).

**Behaviour**
- If the connection drops mid-transfer, both sides show **"Paused: lost connection to <device>. Will continue when it's back."** with what's done so far (e.g. "3.2 GB of 10 GB").
- When the devices see each other again, the transfer **continues from where it stopped**, automatically, without asking again (the original accept still counts).
- This also works after **either app restarts** or **a Mac wakes from sleep**, within a time limit (e.g. 24 hours; choose and record it). After that, partial data is cleaned up and the sender sees a clear message.
- Either side can **Cancel** a paused transfer; partial files are removed.
- A folder transfer resumes at the file and the byte where it stopped; finished files aren't re-sent.

**How (proposed; improve if you have better)**
- Give every transfer a persistent ID and store progress on both sides (which files are done, byte offset of the current file) in the app-data directory.
- Write received data in chunks to the existing `.portal-incoming` partial area (rename to `.seamdrop-incoming`; handle old partial folders), recording verified ranges.
- On reconnect, the receiver tells the sender exactly what it already has; the sender continues from there.
- **Integrity:** the final BLAKE3 check of every file still runs on the whole file before it's moved into place. Never trust a partial file that changed on disk: if the source file on the sender changed since the transfer started (size or modified time), restart that file and say why.
- Pairing, trust and encryption rules are unchanged; a resumed transfer is only accepted from the same paired device.

**Tests**
- Kill the connection at many points (start, middle, last chunk, between files of a folder); resume; hashes match.
- Restart the sender, restart the receiver, both, mid-transfer; resume works.
- Source file modified mid-pause: that file restarts with a clear note.
- Disk full while resuming; cancel while paused; expiry cleanup.
- Add the cases to `docs/MANUAL-TESTS.md` (e.g. turn Wi-Fi off on the Mac for 30 seconds during a 10 GB transfer; close the lid; quit the app).

---

## 3. Auto-updates

Use Tauri v2's updater plugin.

- **Signing key for updates:** generate an updater key pair (`tauri signer generate`). The public key goes in the app config. The private key is **only** stored as a GitHub Actions secret (`TAURI_SIGNING_PRIVATE_KEY`, plus password). Never commit it. Put "back up the updater private key somewhere safe; if it's lost, existing users can't be updated" in "Needs the owner", with exact steps. If you can't create the secret yourself, give me the exact commands.
- **Where updates live:** the code repo is private, so installed apps can't read its releases without a token (and we must never ship a token). Create a **separate public repo** (e.g. `seamdrop-releases`) that holds only release installers and the `latest.json` the updater reads. CI publishes to it on a version tag. Nothing else goes in it. If you can't create it, put it in "Needs the owner" with steps.
- Enable `createUpdaterArtifacts`; CI uploads the signed update bundles and generates `latest.json`.
- **Versioning:** semantic versions; a git tag `vX.Y.Z` triggers a release. Document the release steps in `docs/RELEASING.md`.
- **In the app:**
  - Check for updates on launch and every few hours, quietly.
  - When one is ready: a calm notice, "SeamDrop 0.4.0 is ready. What's new: …" with **Restart to update** and **Later**.
  - **Never restart during a transfer.** If one is running, wait and say so.
  - Settings: "Check for updates" button, current version, and "Install updates automatically" (on by default).
  - If an update fails (no network, bad signature), say what happened in plain words and keep the current version running.
- Tests: an updater check against a local test feed (newer, same, bad signature, offline). Manual check in `docs/MANUAL-TESTS.md`: install an older build, publish a newer test release, see the prompt, update.

---

## 4. Shared clipboard

Copy on one machine, paste on the other.

**Behaviour**
- **Send clipboard** action: from the menu bar/tray, from the main window, and a **global keyboard shortcut** (pick defaults that don't clash with common shortcuts on each OS; make them changeable in Settings). It sends the current clipboard to the paired device, which puts it on its clipboard and shows a small notice: "Clipboard from MacBook Pro: 'meeting notes…' — ready to paste."
- **Automatic sync** as an option in Settings, **off by default**. When on, copies sync between the two machines within about a second.
- Content: **text** (plain and rich text where both OSes support it) and **images**. Files stay with normal transfers. Set a size limit for clipboard images (choose and record it); bigger ones are sent as a file transfer instead, with a note.
- Only between **paired, trusted** devices, over the same encrypted connection.
- A short **clipboard history** (last 10 items received) in the main window, with a "Clear" button. Keep it in memory only unless you have a strong reason; record the choice.

**Privacy (important)**
- Never sync content that apps mark as sensitive: on macOS, respect the pasteboard markers used by password managers (e.g. the `org.nspasteboard.ConcealedType` / `TransientType` convention); on Windows, respect the clipboard formats that ask monitors to skip content (e.g. `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory`). Verify the exact markers in each OS's documentation and record what you implemented.
- Automatic sync shows a small indicator while it's on, so it's never silent.
- No clipboard content in logs; Details shows only "Clipboard sent (text, 1.2 KB)".

**Tests**
- Text, rich text, image, empty clipboard, huge text, image over the limit, sensitive-marked content (must not sync), auto-sync on/off, device offline.
- Manual tests in `docs/MANUAL-TESTS.md`, including copying a password from a password manager (must not arrive).

---

## 5. Signing, ready but switched off

I'll buy the accounts when we're close to sharing SeamDrop. Make CI ready so it's a matter of adding secrets.

- **macOS:** Developer ID signing + notarisation in CI when these secrets exist (Apple certificate + password, Apple API key or Apple ID credentials, team ID; use the names from Tauri's macOS signing guide). Without them, keep today's ad-hoc signing.
- **Windows:** support a signing step when secrets exist. Two paths to document: **Azure Artifact Signing** (open to UK organisations, not UK individuals) and a **standard code-signing certificate**. Without secrets, build unsigned as today.
- Write `docs/SIGNING.md`: what to buy, roughly what it costs, which secrets to add, and how to check a build is signed. Put "set up signing" under "Needs the owner" as a later item, not a blocker.

---

## 6. Small things to include

- Settings → About: version, "Check for updates", links (placeholder website URL `https://seamdrop.app`, to be confirmed).
- Make sure the per-device "Ask every time / Always accept / Never" and the arrival moment from Mission 02 still work after the rename.
- Update the design lab's product name only.

---

## 7. Definition of done

1. The app is called SeamDrop everywhere a user can see it; my existing pairing and settings survive the update.
2. A renamed device and an un-renamed device still find each other during the migration release; incompatible versions show a clear "Update SeamDrop" message.
3. Transfers resume after a dropped connection, an app restart and a Mac sleeping, with whole-file hashes checked; tests cover the cases in section 2.
4. Auto-update works end to end from the public releases repo: an older build updates itself to a newer test release, and never mid-transfer.
5. Clipboard: send-on-demand and optional auto-sync work both ways for text and images; sensitive-marked content never syncs.
6. CI signs when signing secrets exist, and builds as today when they don't; `docs/SIGNING.md` and `docs/RELEASING.md` exist.
7. CI green on macOS and Windows; `docs/MANUAL-TESTS.md` updated; final summary in `docs/PROGRESS.md` with "Needs the owner".

---

## 8. Fresh-eyes review before finishing

Run the same kind of review as last time, with subagents that don't get the mission or progress docs, only: "SeamDrop moves files and your clipboard between a Mac and a Windows PC." Personas: a beginner on a Windows gaming PC, a non-technical Mac user, a privacy-minded user, a developer moving big files daily. Focus on: install and first pairing, a big transfer that drops and resumes, the update prompt, the clipboard (and whether anything private could leak). Write `docs/review-03/SUMMARY.md`, ranked by severity. **Fix everything ranked "blocks use" and "confuses"**, re-check those fixes, and list the rest for me.

---

## 9. Build order
1. Plan; rename (names, identifier, migration, service type, protocol version).
2. Resume: persistent transfer state, partial files, reconnect, restart, expiry; tests.
3. Auto-update: keys, public releases repo, CI release flow, in-app prompt, settings; tests.
4. Clipboard: send on demand, shortcut, history, auto-sync, privacy markers; tests.
5. Signing hooks in CI + `docs/SIGNING.md`.
6. Fresh-eyes review; fix blocks/confuses; final summary.

---

## 10. Future notes (don't build now)
- iPhone → SeamDrop via LocalSend protocol compatibility.
- Choosing between several devices on the network.
- Project Send: the review backlog in `docs/review/SUMMARY.md`; plans: Free, Pro with our AI (higher monthly), Bring your own AI (small monthly; possibly via an MCP connector to the user's own AI tool). No pay-per-conversion.
- Website and download page at the SeamDrop domain.
