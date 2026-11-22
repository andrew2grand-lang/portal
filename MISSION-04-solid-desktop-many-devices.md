# Mission 04: Solid desktop, many devices

**Product:** SeamDrop (free app)
**Owner:** the owner
**Goal:** fix everything the owner's first real-hardware test found, make pairing safer by typing a code, let SeamDrop work with as many devices as someone has, and give the clipboard a Shelf. After this, the desktop app is ready for a small group of testers, and ready for phones to join later.

Read this whole file before doing anything. Work in the existing repo.

---

## 0. How to work (same rules as before)

1. **Skills first.** List every skill you have; load and use all that are relevant (Rust, Tauri, networking, security, frontend/UI design, motion, accessibility, planning, test-driven development, systematic debugging, verification, code review). List them at the top of `docs/PROGRESS.md`.
2. **Work autonomously. Don't ask me questions.** Decide, record it in `docs/DECISIONS.md` with a one-line reason, and keep going. Anything only I can do goes under "Needs the owner" in `docs/PROGRESS.md`, with exact steps.
3. **Plan, then build.** Plan in `docs/PLAN-04.md`, then start.
4. **Small, verified steps** in the order of section 12. Tests pass at each step. Commit after each step. CI stays green on macOS and Windows.
5. **Bugs: cause first.** For every bug in section 1, write down the cause you actually found (file and line) before fixing. Add a test that **fails before the fix** and passes after. If you can't make one fail, say so.
6. **Honest reports.** Label every result as one of: "tested on CI on real macOS/Windows", "tested between processes on one machine (loopback only)", or "written but untested on real hardware".
7. **At the end, run a fresh-eyes review** (section 11) before calling it done.

### Decisions already made
- **Desktop first.** No phone apps in this mission. Nothing you build may assume there are only two devices, or only one Mac and one PC.
- **The look stays for now.** A full design pass is the next mission. Here: fix what's broken, and build new screens from the existing components, tokens and voice.
- **Honest by design** still applies: no bare spinners, plain words, say what's happening and why. Never show made-up information.
- **No password guessing.** SeamDrop never inspects content to guess whether it's a password. It only respects the private markers apps put on the clipboard, and it still never reads marked content (decision 146 stands).
- **One protocol version bump** covers every wire change in this mission (pairing, several transfers, Shelf text, files from the clipboard, per-device sync). Old and new builds refuse each other with the existing "Update SeamDrop on <device>" message.
- **Spend nothing.**
- **CI budget:** the repo is private, so GitHub Actions minutes are limited, and macOS minutes count 10×. Run the heavy suites (browser traces, overlap screenshots, four-peer tests, phone compile checks) on pushes to `main` and on manual runs, not on every commit to a branch. Report the minutes this mission used in PROGRESS.md. If they run low, say so under "Needs the owner" rather than skipping tests silently.

### This mission replaces these earlier decisions (record each in DECISIONS.md)
- **9 / 132:** one transfer at a time → one at a time *per device* (§3).
- **29:** theme → a user choice (§1.7).
- **77 / 134:** the orb when paused → stopped and replaced (§1.4).
- **144:** files on the clipboard → offered as a send (§4.2).
- **148 / 155:** clipboard sync for two devices → defined for N devices (§3).
- **The pairing scheme in `pairing.rs`** → replaced (§2).

---

## 1. Fixes from the owner's real-hardware test (Mac on Wi-Fi, PC on Ethernet, 14 Nov)

What worked, so don't break it:
- Pairing survived the rename.
- Resume worked after turning the Wi-Fi off, after force-quitting the Mac app, and after force-quitting the Windows app.
- The clipboard worked both ways, and password-manager copies weren't sent.
- Settings that affect both computers took effect on both.
- The update check worked.

### 1.1 "Isn't reachable" when both apps have just opened
**Seen:** both apps were opened at the same time. Both said the other "isn't reachable", and the owner had to press **Try again** to connect.

**Causes found so far (confirm each, and look for more):**
- `presenceOf()` in `src/core/buildWorld.ts` turns "paired but not online" into `offline` straight away. So "isn't reachable" shows from the first second.
- The core only tells the UI `online` (`PeerView`, `node/mod.rs` ~742). It has no "still trying" state.
- Dialling a stale stored address backs off 1 → 2 → 4 → 8 → 15 s (`node/trust.rs` 13–14, 45–48, ~96). Only a never-seen address resets it (`node/mod.rs` ~986–990).
- The unicast sweep runs at 4 s, then not again until 30 s (`node/netwatch.rs` 22–26).
- The announce burst lasts only 10 s (`net/discovery.rs` ~419–423).
- The owner's router may pass neither multicast nor broadcast between Wi-Fi and Ethernet (decision 64).

**Wanted:**
- **The core reports reach state per device:** connecting since, attempts made, last failure, next try. The UI shows **Connecting to Desk PC…** from that, saying what's happening in plain words ("looking on this network", "trying the address it was last seen at"). It shows "isn't reachable" only after a grace period (pick it, ~15–20 s, and record it) **and** real failed attempts. The UI never guesses this from timers of its own.
- **Reset and retry on events:** after launch, wake and a network change, retry promptly and reset the back-off.
- **Target:** if both apps are open on the same network, they connect within 5 s without pressing anything.
- **Tests (loopback only, label them so):** two `examples/peer.rs` processes with start offsets from 0 to 20 s, in both orders. Variants:
  - discovery off, so only the stored address is used
  - a stale stored address
  - a listener that starts late

  The test must fail on today's code. If it doesn't, find the case that does.
- **Try again** stays, but nobody should need it normally.

### 1.2 Time left and speed jump around
**Seen:** the time left went from 30 down to 3 minutes, then flicked between 3, 4, 5 and 6. Near the end, "about 10 seconds left" changed constantly. The speed number was just as jumpy.

**Causes found:**
- `emit_progress` runs every 100 ms (`node/mod.rs` ~572, ~596). `rate*0.7 + instant*0.3` (`node/xfer.rs` ~914–925) therefore remembers only about 0.3 s.
- `formatEta()` (`src/engine/format.ts`) rounds every new value with no hysteresis.
- After a resume, `bytes` jumps to the resumed offset, which looks like a burst.
- On the sender, bytes count as soon as they're queued (up to 8 MiB, decision 68), not when they arrive.

**Wanted:**
- **Speed source:** use bytes the receiver has acknowledged, if they're available on the sender. Otherwise say which source you used and why.
- **Speed sample:** a time-stamped window of about 10 s (pick it and record it). Leave out resume and "checking" jumps.
- **Speed display:** update at most every 2 s, rounded to 2 significant figures.
- **Time left:**
  - Compute it from the smoothed speed, then smooth it again.
  - Show **"Working out time left…"** for the first ~5 s and after a resume.
  - Wording: "About 6 minutes", "About 2 minutes", "Less than a minute", "About 40 seconds" (in 10 s steps), "A few seconds".
- **Tests:** run jittery traces through the logic (±40 % Wi-Fi swings, a stall, a burst, a resume). Assert:
  - The shown value **never goes A → B → A within 60 s**.
  - It changes **at most 3 times a minute**.
  - It reaches "A few seconds" without bouncing.

### 1.3 The edge glow stutters on Windows
**Seen:** on the Mac it rotates smoothly. On Windows it slows down and freezes partway, and looks broken.

**Causes found:**
- The angle is written from JavaScript on every frame (`useAnimationFrame`, `EdgeGlow.tsx` ~41–49) into a CSS variable inside a `conic-gradient`.
- Repainting a gradient every frame runs on the main thread. When React or the webview is busy, it freezes.
- The blurred halo is **not** the problem: it's hidden in the real app (`EdgeGlow.module.css` ~138).

**Wanted:**
- **Rotation:** the moving glow is a CSS or Web Animations API animation of `transform` (and `opacity`) only, so it runs on the compositor even when JavaScript is busy. Paint the gradient once on a layer, rotate the layer, and mask it to the ring.
- **Receiving fill:** the arc that grows with progress (`--glow-fill`) needs its own approach that doesn't repaint every frame. For example, update it a few times a second, or use stepped segments. Say which.
- **The look stays the same**, and reduced motion is respected as today.
- **Proof:**
  - Record a performance trace while the glow runs: Playwright Chromium on Linux, **and** `channel: 'msedge'` on the Windows CI runner (the same engine as WebView2).
  - Assert no per-frame repaints of the glow after the first.
  - Also assert the rotation keeps going while the main thread is blocked for 500 ms.

### 1.4 Paused should look paused
**Seen:** when paused, the orb stays frozen mid-movement, which looks like a crash.

**Wanted:** while paused, the orb and the glow stop and are replaced by a calm paused state, e.g. a still pause mark next to the percentage. Movement means work is happening. Keep the percentage and "3.2 GB of 10 GB" as they are; the owner liked those.

### 1.5 Text overlaps in small windows
**Seen:**
- With the window small, texts overlap, especially in the drop area.
- Opening Details squashes everything above it.
- After **Send clipboard**, and after a clipboard arrives, the window gets messy.

**Wanted:**
- **Sizes:** every screen works at a fixed list of sizes: the minimum (420 × 640 today; raise it if needed and say why), 552 × 780, 800 × 900 and 1280 × 900. Both OS chromes, both themes, and with 1 and with 6 devices (§3).
- **Automated test, built before fixing:**
  - Walk every state using the mock core.
  - Run the Mac chrome in WebKit on a macOS runner, and the Windows chrome in Edge (`msedge`) on a Windows runner.
  - Assert that no two visible text elements' boxes overlap and nothing is clipped.
  - Exclusions are listed in the test up front: `aria-hidden`, visually hidden text, and anything mid-animation (wait for animations to settle).
- **Screenshots:** of each state, in `docs/screens/m04-*`.
- **Clipboard messages:** arrival and send messages get one fixed, tidy place (§4.1) and never push the layout around.

### 1.6 Details: useful, calm, tucked away
**Seen:** the last log line is always visible under every screen and looks like an error console. Repeated lines fill it up. Red text alarms people about things that don't matter. Review 03 said the same.

**Wanted:**
- **Closed by default:** a small **Details** button with a terminal icon at the bottom. No preview of the last line.
- **When open, it reads like a calm progress story**, the way Claude Code shows its work: "Found Desk PC on Wi-Fi", "Resumed at 1.2 GB (nothing sent twice)", "Every file checked ✓", "Saved in Downloads/SeamDrop".
- **Rules:**
  - Every line comes from a real event. Nothing invented.
  - Plain words first, technical detail after.
  - Tone: neutral by default. Amber only when something is wrong *and* affects you. Red only when something failed and needs you.
  - Identical lines in a row collapse: "Clipboard sent to Desk PC ×12, last at 19:31".
  - Never clipboard content.
- **Copy technical log** keeps the full raw log for support.

### 1.7 Light / dark switch
- Add **Appearance: System · Light · Dark** in Settings, and a quick switch in the menu bar / tray menu.
- Default: System.

### 1.8 Where received files go
Keep one folder, `Downloads/SeamDrop`. The owner agrees one folder beats a folder per delivery. Make it obvious and easy to change:
- **First arrival:** the done screen says where the files went, with **Show in folder** and **Change folder**.
- **Every done screen** keeps **Show in folder**.
- **Settings** shows the full path with **Open** and **Change**.
- **Optional, off by default:** "A folder for each device" (`SeamDrop/From MacBook Pro/…`).
  - Use **this computer's** saved name for that device, not the name the other device sends.
  - Make the name safe with `fsafe`.
  - Handle two devices with the same name.
  - Keep the folder if the device is renamed later.

---

## 2. Safer pairing: type the code

Today both computers show the same 6-digit code and each person presses Confirm (`crates/portal-core/src/pairing.rs`). People tend to press Confirm without comparing. Typing makes them actually check.

**Behaviour**
1. You start pairing on the computer in front of you and pick the other device.
2. **The other device asks first:** "MacBook Pro (192.168.1.20) wants to pair · **Show code** / **Ignore**". Nothing appears without that question, and Ignore gives the requester no hint.
3. After Show code, it shows a fresh random 6-digit code. You type that code on the computer where you started.
   - A stranger on the network can never finish pairing, because they can't see your screen.
4. **Wrong code:**
   - The person gets a couple of typo retries within the same attempt.
   - Separately, a **cap on failed attempts across all requesters** locks pairing on the showing computer until its person presses something.
   - Limit per device or address alone isn't enough: new keys cost nothing and IPv6 addresses change.
   - Keep the 2-minute timeout.
5. **After success**, both computers say "Paired with <name>" and show a notification.
6. **Existing pairings** keep working (their keys are pinned). Nobody re-pairs.

**How: a PAKE (password-authenticated key exchange) with the code as the password**
- **Use a specified PAKE that a phone could implement from the document:**
  - SPAKE2 per RFC 9382, with its test vectors, or a pinned CPace draft with test vectors.
  - Choose and record why.
  - If you use a crate, check it matches that spec (the RustCrypto `spake2` crate follows magic-wormhole and has never been independently audited; record that). Otherwise write `docs/PROTOCOL.md` so phones must link `portal-core`, and say so.
- **Roles:** fixed: A = TLS client, B = TLS server. **No symmetric mode.**
- **Binding:** put both certificate fingerprints and the TLS exporter into the PAKE identities/transcript **and** into the key derivation.
- **Key confirmation** both ways, with **different labels per role**, so your own confirmation echoed back fails. Compare in constant time.
- **The code goes in only through the PAKE.** Never send a hash, MAC or anything else derived from the code: with a 6-digit code, that lets an attacker try all million codes offline.
- **No old scheme:** the new protocol has no handlers for the old `PairCommit` / `PairReveal` messages and never falls back to them.
- **No fallback scheme.** If a PAKE turns out to be impossible here, stop this section, keep today's pairing, and explain why in PROGRESS.md.

**Tests**
- Right code pairs both sides.
- Wrong code fails on both, and the limits and global cap hold.
- A simulated man in the middle (two separate TLS sessions) fails on **both** sides.
- **Reflection test:** a confirmation echoed back is rejected.
- **No-downgrade test:** old pairing messages are refused.
- Ignore, timeout, and cancel on either side.
- An existing pairing still connects after the update.
- Add the manual steps to `docs/MANUAL-TESTS.md`.

---

## 3. As many devices as you have

The owner wants to use SeamDrop with 3–4 computers at home, and phones will join later. **The current code assumes one other device in many places.** Change every one of them (confirm the list, and find the rest):

- **UI model:** `Side = 'mac' | 'win'` (`src/engine/types.ts:1`), and the World is keyed by it (~264–290). Two Macs can't even be represented.
  → Devices keyed by device id; this computer is just one of them.
- **Core:** `transfer: Option<Transfer>` (`node/mod.rs` ~408). Code that relies on it:
  - the wait on quit (~612)
  - the update wait
  - `upgrade_links` (`trust.rs` ~106)
  - resolving duplicate links (`mod.rs` ~1364)
  - `cmd_set_incoming` (`trust.rs` ~234)

  → Transfers keyed by transfer id, at most one running per device.
- **Resume:** `load_paused` keeps only the newest paused transfer and **deletes the others' partial data** (`node/resume.rs` ~469–475).
  → Keep every paused transfer.
- **Clipboard:** sync is one global switch (`store.rs` ~27) with one `peer_on`/`request` (`node/clip.rs` ~42–46). `clip_target` sends to the first connected device (`node/clip.rs` ~88–100).
- **Shell:** one accept window (`shell.rs` ~32), a single-device tray (`tray.rs` ~112, 138), sends going to `primaryPeer` (`CoreProvider.tsx` ~156, 162), and one progress/rate value.

**Behaviour**
- **Pair and remember any number of devices.** The main window shows them all, each with its status (connected · connecting · not reachable) and how it's connected. With only one other device, the screen looks like today's, so the simple flow stays simple.
- **Choose where things go.** Before sending files or the clipboard, tick one or several devices. Remember the last choice. The **shortcut and the tray** send to that remembered choice; say so in Settings.
- **Send to several at once:**
  - Each device gets its own transfer, progress, pause and resume.
  - Across devices, transfers run in parallel; one at a time per device.
  - A computer can receive from more than one device at once. **Reserve free space** across all incoming transfers, not each one separately (`xfer.rs` ~365).
  - Say clearly what happens when the per-device limit is hit.
- **Per-device settings stay per device:**
  - "Files from <device>: Ask every time · Always accept · Never"
  - where an incoming clipboard goes (§4.1)
- **Automatic clipboard sync with several devices:**
  - Consent is per pair of devices, as decision 155 already works for two.
  - Turning sync off on a device stops it with every device it was syncing with.
  - Each device shows who it's syncing with.
  - **Pause (§4.4) is local only and doesn't revoke consent.**
- **Accept window:** handles requests from more than one device (queued or stacked; pick one and record it).
- **Room to grow:** the window can get bigger to show more devices side by side. Keep it tidy with 1 device and with 6. The visual design comes next mission; get the structure right here.
- **Remove a device** from Settings, saying plainly what that means.

**Tests (loopback only)**
- Four headless peers (`examples/peer.rs`):
  - pair all
  - send to two at once
  - receive from two at once (space reserved)
  - drop one mid-transfer and resume it while the others carry on
  - restart with two paused transfers (both survive)
  - clipboard to one and to several
  - auto-sync with two, with no ping-pong loops
- Add a "3–4 computers" section to `docs/MANUAL-TESTS.md`. The owner has other laptops at home to try it with.

---

## 4. Clipboard: the Shelf, and the rough edges

### 4.1 The Shelf
The received-clipboard list becomes **the Shelf**: one place in the window where everything sent to this computer waits, with **Copy** next to each item and the device it came from. It's kept in memory only, as today.
- **Per device, where does an incoming clipboard go?**
  - "Straight to the clipboard (and the Shelf)": the default, because if you pressed Send you usually want to paste.
  - "**Shelf only**: don't replace what I copied".
- **Add text yourself:** a small box on the Shelf. Type or paste, pick the device(s), press **Send**. It lands on their Shelf, and on their clipboard if their setting says so. Keep it compact so the window doesn't feel crowded.
  - This is also the deliberate way to send something an app marked as private (§4.3): the person pastes it into the box themselves.
- **Arrival and send messages** ("Sent to Desk PC. Paste it there with Ctrl+V") show in one fixed, consistent place and fade, without moving the layout.

### 4.2 Files on the clipboard
**Seen:** copying files and pressing Send clipboard says "Files are on the clipboard. Drop them on the window to send them." (`node/clip.rs` ~143).

**Wanted:** offer it in one step: "2 files on the clipboard · **Send them to Desk PC**", which starts a normal file transfer to the chosen device(s). A screenshot copied as an image still goes as an image.

### 4.3 A browser copy refused as private
**Seen:** text copied from claude.ai in **Tor Browser** on the Windows PC was refused as private. Text from the Claude desktop app went through.

**What's known:**
- SeamDrop doesn't guess. The refusal comes only from the formats in `portal-sys/src/clipboard/mod.rs`.
- Tor Browser is built on Firefox and always runs in private browsing. Since Firefox 94 / ESR 91.3, Firefox marks everything copied from a Private Browsing window on Windows so it stays out of Clipboard History and Cloud Clipboard (Mozilla security blog, 15 Dec 2021). **So the marker worked as designed.** The blog doesn't name the formats; find out which ones are set.
- Chromium-based browsers reportedly do the same for incognito / InPrivate copies (Windows Latest, Feb 2025): `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory = 0`, `CanUploadToCloudClipboard = 0`. Confirm this in Chromium's source.

**Wanted:**
- **Name the marker.** `Read::Private` carries which format or type caused the refusal: its name only, never the content. Details says it in plain words, e.g. "Not sent: your browser marked this copy as private, as it does in private windows (Tor Browser always does)." Where the source app is known, use its name.
- **The on-screen message doesn't say "we detected".** It says: "The app you copied this from marked it as private, so it wasn't sent. To send it anyway, paste it into the Shelf's text box."
  - There is **no "Send anyway" button**: decision 146 (marked content is never read) stands.
- **Reproduce on the Windows CI runner**, with Firefox (normal and a private window), Edge and Chrome (normal and incognito/InPrivate):
  - "Normal" needs `launchPersistentContext`, because Playwright's default contexts behave like incognito.
  - Test both Ctrl+C and a page's copy button (`navigator.clipboard.writeText`).
  - Record exactly which browsers set which markers, and when.
  - If a normal (not incognito) copy is marked, write down why. Don't change the rule without recording it.

### 4.4 Pause and off within one click
- **In the menu bar / tray**, visible whenever sync is on:
  - **Pause clipboard sync** (1 hour · until SeamDrop restarts). This is local only.
  - **Turn off clipboard sync**.
- **While sync is on**, the window and the tray show it, as today.
- **One line where it matters** (Settings → Clipboard, and the first time sync is turned on): "Goes only to your paired devices, encrypted, never through a server. Anything an app marks as private, as password managers do, is never sent."

### 4.5 Clipboard stress tests
- Copying fast and repeatedly.
- Very long text; emoji and other scripts.
- Rich text from Word or Pages.
- Images from screenshots and from a browser.
- Copying while a transfer runs (allow **Send clipboard** during a transfer).
- The other app closed; the other device asleep.
- Sync with two devices at once, with no loops.
- Marked content.

Automate what you can. Put the rest in `docs/MANUAL-TESTS.md`.

---

## 5. Small things

- **Hide the Project · Paid tab** for now (all four reviewers found it confusing). Keep the code, behind a developer setting that's off.
- **Placeholders:** the website is `https://seamdrop.com` and the feedback address is `feedback@seamdrop.com`, both in one constants file.
  - **We don't own seamdrop.com** (someone else has registered it). So no build may open that site or send email to it until the owner switches a "confirmed" flag in that file.
  - Until then, About shows the address as text marked "not live yet", and Send feedback saves the log and says feedback isn't set up yet. "Needs the owner": the final domain and address.
- **Send feedback**, in Settings → About and in Details:
  - Once confirmed, it opens an email to the address in that constant.
  - The body is short: the SeamDrop version and OS.
  - If the person ticks "Include technical log", the log is saved to a file and shown in Finder / Explorer for them to attach. A `mailto:` link can't attach files, and long bodies get cut off.
- **Review 03's "annoys" list:** fix what's cheap (keyboard shortcuts on the accept window; saying where files go on the accept screen). List the rest.

---

## 6. Phone-ready core (no phone app yet)

- **Write `docs/PROTOCOL.md`:** discovery, pairing (the new scheme, with test vectors), messages, transfers and resume, the clipboard. A phone app should be able to implement it from this document, or it should say plainly which parts require linking `portal-core`.
- **Keep `portal-core` free** of anything desktop-only that a phone build would trip over.
- **Prove it compiles for phones:** add CI jobs that run `cargo check -p portal-core` for Android (`aarch64-linux-android`, using the NDK already on GitHub's Ubuntu runners) and iOS (`aarch64-apple-ios`, on the macOS runner). Run them on pushes to main only. If either needs more than about an hour of work, stop, and write down exactly what blocks it.
- **Leave room for a relay later:** the pairing keys and the message framing must not assume a direct LAN connection, so SeamDrop Anywhere can later carry the same end-to-end encrypted stream through a relay. Note how in `PROTOCOL.md`; don't build it.
- **Note in `docs/DECISIONS.md`** what would change for iOS and Android: background limits, no global shortcut, the share sheet. Don't build it.

---

## 7. Only if everything else is done and green: LocalSend bridge

This is a stopgap, so an iPhone can send photos to SeamDrop now using the free LocalSend app, until there's a SeamDrop iPhone app.
- **Receive only, off by default:** Settings → "Receive from the LocalSend app (temporary)".
- **Every LocalSend transfer asks every time.** It's labelled "from LocalSend on <name> (name not verified)", and the bridge can be turned off from the prompt.
- **It opens a new, unauthenticated way in**, so:
  - limit how often prompts can appear
  - cap sizes
  - reuse `fsafe` and the staging folder
  - never let it touch SeamDrop pairing or trust
- **Implement the public protocol** (github.com/localsend/protocol) faithfully.
- **If there's no time, skip it** and say so. It's not required for done.

---

## 8. Not in this mission
- Visual redesign (next mission)
- Phone apps
- Website
- SeamDrop Anywhere
- Project Send
- Signing (the hooks already exist)

---

## 9. Definition of done

**Must have (testers need these):**
1. Two computers connect by themselves within 5 s of both opening (loopback tests in §1.1, including the stale-address and late-listener variants). "Isn't reachable" appears only after the grace period and real failed attempts, driven by the core's reach state.
2. Time left and speed pass the trace tests in §1.2.
3. The glow passes the trace tests in §1.3, including on Edge on Windows. Paused looks paused.
4. The overlap test in §1.5 passes on both runners at every listed size, theme, and with 1 and 6 devices.
5. Details is closed by default, collapses repeats, is honest and calm.
6. Any number of devices: everything in §3's tests passes, and no paused transfer is ever discarded.
7. Pairing by typed code, with every §2 test passing. Old pairings still work.
8. The Shelf, Shelf only, the text box, files on the clipboard, the named private marker, and pause/off in one click.
9. CI is green on macOS and Windows.
10. `docs/MANUAL-TESTS.md` has a short check for the owner (~20 minutes) covering this mission, in plain words. There's a final summary in `docs/PROGRESS.md` with "Needs the owner".

**Should have:**
- The appearance switch.
- The received-folder changes.
- `docs/PROTOCOL.md`, and `portal-core` compiling for Android and iOS (or a written list of exactly what blocks it).
- The Project tab hidden.
- Send feedback.
- The browser-marker findings.

**Optional:**
- The LocalSend bridge.

---

## 10. Testing you can't do
You can't run two real machines on the owner's network. Test between processes and on CI, and label where each result comes from. The manual check for the owner should be short, in plain words, ordered by what matters most, and say what to send back.

---

## 11. Fresh-eyes review before finishing
Use subagents that get no mission, plan or decision files, only: "SeamDrop moves files and your clipboard between your computers."

**Personas:**
- a beginner pairing two computers for the first time
- someone with four computers at home
- a privacy-minded user (pairing requests from strangers, the clipboard, the Shelf)
- a developer moving big files daily (time left, resume, Details)

**Focus:** first pairing with a typed code, choosing devices, a big transfer that drops and resumes, the Shelf, Details, small windows.

**Output:** write `docs/review-04/SUMMARY.md`, ranked by severity. **Fix everything ranked "blocks use" and "confuses"**, re-check those fixes, and list the rest for the owner.

---

## 12. Build order
1. Plan.
2. §1.1: connecting at launch (core reach state first).
3. §1.2 time left and speed; §1.3 glow; §1.4 paused.
4. §3 many devices: **the data model first** (core transfers by id, UI devices by id, clipboard consent per pair, keep all paused transfers), then the screens.
5. §1.5: build the overlap test, then fix, now on the new UI model. Then §1.6 Details, §1.7 appearance, §1.8 received folder.
6. §2 typed-code pairing.
7. §4 clipboard: Shelf, files, the named marker, pause/off, stress tests.
8. §5 small things; §6 protocol document.
9. §7 LocalSend bridge, only if everything above is green.
10. §11 fresh-eyes review; fix "blocks use" and "confuses"; final summary.

---

## 13. Future notes (don't build now)
- **Mission 05, design pass:** redesign the desktop app from the owner's references; phone screens at iPhone and Android sizes in the design lab; the website's look in the same lab, so it's one visual language.
- **Mission 06:** the website (domain, download buttons from the public releases repo, a waitlist for the phone apps, privacy policy).
- **Mission 07:** the Android app (built in CI, tried in the Android emulator on the PC; a real phone later).
- **iPhone app:** when Xcode or the Apple Developer account is available. Pairing by QR code for phones (the same code, scanned).
- SeamDrop Anywhere (the relay, paid) and Project Send come later.
