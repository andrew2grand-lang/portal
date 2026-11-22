# Mission 05: Clear, calm, and a real Shelf

**Product:** SeamDrop (free app)
**Owner:** the owner
**Goal:** fix everything from the owner's second real-hardware test (three computers, 20–21 Nov). Rethink the clipboard and the Shelf so each part of the app does one job. Cut the text down to what people need. Make receiving feel as good as sending. Get new builds onto the owner's machines with less hassle.

Read this whole file before doing anything. Work in the existing repo.

Items marked **(our addition)** are not from the owner's report. They're my answer to something he reported, so he may change them.

---

## 0. How to work

1. **Skills first.** List every skill you have. Load and use all that are relevant: Rust, Tauri, networking, security, frontend/UI design, motion, accessibility, content design/UX writing, planning, test-driven development, systematic debugging, verification, code review. List them at the top of `docs/PROGRESS.md`.
2. **Work autonomously. Don't ask me questions.** Decide, record it in `docs/DECISIONS.md` with a one-line reason, and keep going. Anything only I can do goes under "Needs the owner" in `docs/PROGRESS.md`, with exact steps.
3. **Plan, then build.** Plan in `docs/PLAN-05.md`, then start.
4. **Bugs: cause first.** For every bug in §1, write down the cause you actually found (file and line) before fixing. Add a test that fails before the fix. If you can't make one fail, say so.
5. **Honest reports.** Label every result as one of:
   - "CI on real macOS/Windows"
   - "loopback between processes"
   - "this Mac only"
   - "untested on real hardware"
6. **Speed rules.** The last mission took 7 hours, and much of that was waiting.
   - While building, run only the tests for what you touched. Run the full Rust and UI suites at the end of each step of §12.
   - Push at the end of each step, but **don't wait for CI between steps**. Check CI at three milestones only (§12).
   - GitHub Actions minutes are scarce (private repo, macOS counts 10×). Heavy suites run on `main` and manual runs only. Report the minutes used.
   - Record in PROGRESS.md how long each step took.
   - **Time limit for the should-haves:** if the mission has run more than about 6 hours when you reach §12 step 7, skip what's left of it and list it.
7. **At the end:** the fresh-eyes review (§11), then delivery (§8).

### Decisions already made
- **One job per place.** The app has three places, and each does one thing:
  - **Send** (the home screen): files and folders.
  - **Clipboard:** copy here, paste there; automatic sync; a short "recently received" list.
  - **Shelf:** a shared, kept place for text you deliberately put there.

  Settings is for settings, and Help is for explanations. Nothing else carries paragraphs.
- **Honest by design** still applies. The app always says what happened, and what didn't happen and why: plain words, no bare spinners, never made-up information. But it says it **briefly**, and explanations go in Help (§6).
- **The look stays in the same family.** A full visual pass from the owner's references comes later. This mission makes the app simpler and clearer with the existing tokens and components.
- **No password guessing.** SeamDrop never inspects content to decide whether it's private. Only the markers apps set count. Decision 146 is **amended** in one way only (§3.1, Put on Shelf); record it.
- **One protocol version bump** (to 4) covers every wire change in this mission:
  - liveness changes
  - the "held: waiting for you to continue" message
  - the send queue's hand-over
  - Shelf sync
  - device pictures

  Old and new builds refuse each other with the existing "Update SeamDrop on <device>" message. **No build goes onto the owner's computers, locally or as a beta, until every protocol-4 message is final.** Decision 204 means unknown messages are silently ignored, so half-finished protocol-4 builds would quietly fail to talk.
- **Drop the old `_portal._tcp` browsing** (decision 121). Protocol 4 can't talk to Portal anyway.
- **Spend nothing.**

### This mission changes these earlier decisions
Check each one, and record exactly how it changes:
- **74 / 187 / 216:** sending while busy → the queue (§1.5).
- **108 / 175:** the receiving edge fill → the same light as sending. 108's short fill for instant transfers needs a replacement too.
- **133 / 179 and 140 / 233:** quitting and resuming → §1.4. Update restarts still resume by themselves.
- **146:** amended for Put on Shelf only (§3.1).
- **150:** the Shelf kept in memory only → kept on disk, encrypted (§3.2). Automatic arrivals: the in-window notice only.
- **182 and 192:** small windows → §5.
- **206 / 207 / 211 / 226 / 227:** the Shelf, "Clipboard from X: Straight to the clipboard / Shelf only", and clearing on removal → §3.
- **The default received folder `Downloads/SeamDrop`, and keeping senders' file and folder dates** → §4.

---

## 1. Fixes from the second real-hardware test

**What worked, so don't break it:**
- Opening both apps connects in under a second, every time, even after killing the apps from Task Manager or Activity Monitor.
- Typed-code pairing: wrong three times, then a fresh code, then paired. The owner found it "very clear".
- The time left is much calmer during a normal transfer.
- The paused state: the orb is still and "not reachable" shows at the top.
- Resume after the Wi-Fi drops, and after quitting both apps.
- Appearance.
- Send to two computers at once.
- Downloads as the received folder.
- The Shelf's text box as a way to send private text.

### 1.1 Receiving shows a slow, glitchy orange crawl around the window
**Seen:** on the PC and on Sam's MacBook Air, while receiving, an orange light creeps around the window border at about one frame per second, "if not slower". It looks broken. The owner realised it's progress drawn around the window. He doesn't want it, because the progress bar inside the window already shows progress.

**Cause (confirm):** this is the design, not a performance bug. The receiving mode fills an arc from progress in whole degrees, a few times a second (`EdgeGlow.tsx` ~121–132; `--glow-fill` in `EdgeGlow.module.css` ~105–107; decisions 108/175). On a 10 GB transfer one degree takes more than a second.

**Wanted:**
- Remove the progress fill. Receiving shows **the same smooth travelling light as sending**, on the compositor, as Mission 04 built it. A calm difference between the two (direction or tint) is fine; record which.
- Replace 108's short fill for instant transfers with the arrival moment (§1.6).
- Still when paused.

### 1.2 Losing the connection takes 6–9 seconds to show on the other computer
**Seen:** with the Mac's Wi-Fi turned off at 48 %, the PC (on Ethernet) kept showing a speed that slowly drained away. It said "waiting on both devices", and only showed Paused after 6–9 seconds.

**Leads (confirm):**
- The computer that lost Wi-Fi knows at once.
- The 6–9 s on the PC is most likely Windows TCP giving up on retransmits, not our pings (`PING_EVERY` 10 s, `node/mod.rs` ~39) or TCP keepalive (`net/dial.rs` ~48).
- With the Mac as the survivor, it could take up to `IDLE_TIMEOUT`, 30 s (`net/link.rs` ~23).
- The 10 s speed window keeps showing a falling number after the bytes have stopped.

**Traps to avoid:**
- Pings share the stream with 1 MiB frames, up to 8 MiB queued in the app, plus the OS buffer. `link.rs` ~337 times out a whole `read_frame`, and a 1 MiB frame at 300 KB/s takes more than 3 s.
- While the sender re-hashes what already arrived after a resume, it only sends a keepalive every 15 s (`send.rs` ~81).

A naive "ping every second, pause after 3 s" would therefore pause slow Wi-Fi and every big resume. Each pause would re-hash again, so it could loop.

**Wanted:**
- **Any bytes read on a link count as alive.** Liveness pings are sent independently of the reading and writing of big frames, including during the re-hash.
- **The survivor shows Paused within about 3 s** of the other computer really disappearing.
- **As soon as nothing has moved for about 2 s**, hide the speed and say "Waiting for <device>…". Never show a draining number.
- **Tests (loopback, 20 out of 20 each):**
  - Cut one side's traffic without closing the socket: the survivor reaches Paused in ≤ 3 s, with the Windows peer as the survivor and with the Mac peer as the survivor (CI).
  - A link throttled to 300 KB/s never pauses.
  - A multi-GB re-hash after a resume never pauses.

### 1.3 Time left: stuck after a resume, and too slow to update
**Seen:**
- After quitting both apps and reopening, the speed crawled from about 1 MB/s back up to the usual 21 MB/s.
- For a couple of minutes the time left was stuck at about 514–520 minutes, while the percentage climbed quickly.
- Later it showed 7 → 4 minutes when it was really about 2.
- The owner wants it to update realistically, "not every 20 seconds, because some transfers are just 20 seconds long".

**Leads (confirm):**
- The estimator restarts in `run()` (`node/resume.rs` ~93) **before** the sender re-hashes what already arrived (`send.rs` ~235). So that quiet spell sits inside the speed window, despite what `estimate.rs` lines 8–9 say.
- After `WORKING_OUT` (5 s), the first estimate comes from a speed that's still ramping up.
- Then smoothing (`TAU` up to 30 s), `DWELL` (21 s), `NO_RETURN` (65 s), and the rule that a minutes value within 10 % of the shown one never replaces it (`estimate.rs` ~185–187) hold the wrong number.

**Find out first:** is the slow ramp after a resume real (the re-hash, the window, TCP) or only on the display? If it's real, fix the cause where possible. A resume should reach full speed within a few seconds of the re-hash ending, and the screen should say "Checking what already arrived…" while it re-hashes.

**Wanted (these rules, in this priority):**
1. **No estimate until the speed has settled:** after the re-hash, once the window is full and steady. Until then: "Working out time left…". This doesn't count against the accuracy check.
2. **Big corrections show quickly.** If the estimate is more than ~30 % away from what's shown, in either direction, and holds for ~3 s, update at once. This overrides the no-return rule.
3. **Small changes stay calm:** no A → B → A within 60 s, applied only to changes under ~30 %.
4. **Updates scale with the transfer:** about every 3–5 s for short times, longer gaps only for long ones.
5. **Short transfers:** if the whole thing will take under about 30 s, no minutes at all. Show progress and, near the end, "A few seconds".

**Accuracy test:** replay recorded traces, from throttled loopback runs including a real resume with its re-hash, plus synthetic Wi-Fi jitter. From the first shown estimate onwards, the shown time must stay within ±25 % or ±15 s of the real finish, whichever is larger.

### 1.4 Quitting the app during a transfer
**Seen:** after quitting mid-transfer, reopening made the transfer continue by itself. The owner would rather be asked. "Maybe I closed the window on purpose." He wants Continue or Cancel. He liked the automatic resume after the **Wi-Fi drop**, so keep that.

**Cause (confirm):** a normal Quit pauses the transfer and stamps `paused_at` (decision 133; `node/mod.rs` ~705 → `resume.rs` ~139–151 → `journal.rs` ~177–184). On restart that looks the same as "the network dropped, then I quit". Only a kill or crash leaves `paused_at` empty (`resume.rs` ~522–536).

**Wanted:**
- **The journal records why a transfer stopped:** `quit` · `crash` (no clean stop) · `network` · `update` (restart to update). Write the reason on stop, before exit.
- **On the next launch:**
  - **quit / crash:** that computer asks: "**Sending 'big10g.bin' to Den_PC stopped partway.** [Continue] [Cancel]". Use "Receiving … from" on the receiving side. Show the percentage only when it's known for sure; the sender's journal only stores bytes at a pause (`journal.rs` ~180). Once connected, ask the receiver for its verified ranges and show it then.
  - **network / update:** resume by themselves, as today (decisions 140/233).
- **The other computer** shows "**Paused: waiting for <device> to continue**". This is a protocol-4 "held" message. It keeps its own Cancel.
- **Tests:**
  - quit → asks
  - kill → asks
  - network drop → continues by itself
  - update restart → continues by itself
  - Cancel from either side cleans up both

### 1.5 Adding files while a transfer is running
**Seen:** with the 10 GB transfer running, the owner dropped small files on both computers. Both said "Ready to send", with the whole send setup and "add more". Only the disabled Send button hinted that something was busy, and the message "is not big enough or clear enough".

**Wanted:**
- **A clear busy message**, which is what he asked for: "**Busy sending to Den_PC.**" No screen says "Ready to send" when it can't send.
- **A per-device queue (our addition):**
  - Files dropped for a busy device can be queued: "**Next for Den_PC:** 2 files · starts when the current transfer finishes", each removable.
  - When the current transfer ends (done, cancelled or failed), the next one is offered to the other computer as usual (Ask every time / Always accept).
  - It survives a restart and follows §1.4.
  - **Both computers can hold queues for each other.** When a transfer ends and both have something queued, the one queued earliest goes first. On a tie, the lower device id goes first. The other waits for the next free slot (decision 177).
  - **Tests:** both computers queued at once; a three-device queue.

### 1.6 Receiving should feel as good as sending
**Seen:** the sender gets a nice tick and **Done**. The receiver gets "Received from X", a small ✕ and "Show in File Explorer": "no happiness for a nice transfer".

**Wanted:**
- The receiving computer gets **the same finished moment as the sender**: the same tick and animation language, and "**Received 10 GB from macbook one**". Name the file, or "3 files", or the folder name.
- **Actions:** **Show in Finder/File Explorer** (primary) and **Done**. No lone ✕.
- **"Open" safety:** SeamDrop doesn't currently mark received files the way browsers mark downloads (macOS quarantine, Windows Mark-of-the-Web). So "Open" on a received app or script would run it with no Gatekeeper or SmartScreen check.
  - Either mark received files like downloads, and record the trade-off (Office opens marked files in Protected View),
  - or offer **Open** only for documents, images and media, **never** for apps, installers, scripts, shortcuts or archives.

  Choose and record.
- **When the window isn't in front:** a system notification. On the Mac a click on it only activates the app (decision 50), so a click shows **the latest delivery**; say so in the code.

### 1.7 Settings: containers that stretch the wrong way
**Seen:** in Settings, the rounded container behind "System / Light / Dark" stretches to the far right while the options sit on the left. The same goes for "Ask every time / Always accept / Never" (the owner's screenshot).

**Cause (confirm):** `.incoming` (`Stage.module.css` ~877) stretches the `Choice` track.

**Wanted:**
- Each segmented control either **hugs its options**, or spreads its segments **evenly across the full width**. One rule for the whole app; record it.
- Audit every container, row and button group in Settings and the panels for the same problem.
- Before/after screenshots in `docs/screens/m05-*`.

### 1.8 Third computer: only the PC showed up
**Seen:** the new MacBook Air (Sam's), with nothing paired yet, showed only the PC. The owner wanted to pair it with macbook one.

**Likely cause (confirm first):** with nothing paired, the window shows a single nearby computer. `focusOf` falls back to `peers[0]` (`buildWorld.ts` ~124–134; `NearbyView`, `Connection.tsx` ~85), and "Den_PC" sorts before "macbook one". Then check real discovery too: Mac-to-Mac multicast on Wi-Fi, Local Network permission, the sweep.

**Wanted:**
- **The nearby list (§2.2) is a must-have.** Every computer running SeamDrop on the network shows up within about 5 s, where the network passes multicast or the unicast sweep reaches it.
- Add a loopback test with three peers and nothing paired: all are listed.

### 1.9 Automatic sync missed a copy that Send clipboard handled
**Seen:** with automatic sync on, the owner copied an environment variable value in a browser (Vercel's copy button). It wasn't synced automatically. Pressing **Send clipboard** then sent it, so it wasn't marked private. The report doesn't say which computer, so check both.

**Lead (confirm):**
- The watcher marks a copy as seen **before** reading it (`node/clip.rs` ~615).
- On Windows a read returns `Empty` if another app holds the clipboard for more than 0.5 s (`win.rs` ~41–49, ~157).
- Automatic sync drops `Empty` silently (`clip.rs` ~220).
- So a busy browser can make a copy vanish from sync, while a manual send later works.

**Wanted:**
- Retry the read for that same change before giving up. Don't mark a change as seen until it has been read or truly given up on.
- If it's given up, the "not synced" notice says so (§3.1).
- Reproduce with a page's copy button (`navigator.clipboard.writeText`) and with Ctrl+C / ⌘C, on both OSes. The test fails before the fix.

---

## 2. Pairing and devices

### 2.1 When the other computer removes the pairing
**Seen:** the owner removed the pairing on the PC. On the Mac the pairing was gone too, but the Mac only offered **Pair**, with no way to say "fine, forget it".

**Wanted:** "**Den_PC removed this pairing.**" with two choices:
- **Pair again**
- **OK**: it becomes just a nearby computer, with no screen pushing you to re-pair.

### 2.2 Adding and choosing devices
- **The nearby list (must have, §1.8):** whenever pairing is possible, list **every** nearby computer with its name, OS and address, each with **Pair**. Never pick one for the person.
- **After a pairing succeeds,** return to the list with that one marked as paired, so pairing several in a row is quick.
- **Add a device from the home screen:** a "+" at the end of the device strip, not only in Settings.

### 2.3 Device pictures
The owner's idea: give each device a picture so it's easy to tell apart, which also adds colour to the app.
- **Settings → This computer → Picture:** choose a **photo**. It's cropped to a circle on this computer, re-encoded as PNG (max 256 × 256, max 64 KB), and stripped of EXIF and location data.
  - **(our addition)** Or choose a colour and an icon (laptop, desktop, phone, tablet). Default: a colour from the device id plus the right OS icon.
- **Shared with paired devices** over the encrypted link (protocol 4). Shown wherever a device appears: the strip, offers, done screens, the Shelf.
- **Checking pictures from other devices:**
  - PNG only, never SVG.
  - Check the signature, dimensions and size **before** decoding, using the `png` crate already in the tree.
  - Anything else falls back to the default.
- Should have: if time runs short (§0.6), ship the photo only and list the rest.

---

## 3. Clipboard and Shelf: one job each

**The owner's findings:**
- The Shelf panel is "a bunch of information and sort of like a manual".
- There are two automatic-sync switches, one in Settings and one on the Shelf.
- "I myself don't know what it's syncing."
- Shelf items show "rich text, 639 bytes" and "(text)".
- **Send text** put the text on the MacBook but not on the PC where he typed it.
- The Shelf empties when the app quits.
- ✕ doesn't ask, and Clear Shelf isn't red.
- A private copy was silently not synced, and he wants to be told and pointed to the Shelf.
- Images arrived on the Shelf through automatic sync, and he wasn't sure they belong there, but he also doesn't want to dig through screenshot folders on the PC.

He wants the Shelf to be "an individual medium that stays saved, whatever you do with the app": deliberate, kept, professional, and the place for keys and private things.

### 3.1 Clipboard (its own place, reached from the header)
- **Send clipboard:** one big, obvious button, plus the shortcut. It sends to the ticked devices.
- **Files on the clipboard:** a real button, "**Send 2 files to Den_PC**", replacing today's "slightly missable" link (decision 205).
- **Automatic sync: one place only.**
  - Switched per device here, and nowhere else. Settings may link here.
  - Consent stays two-sided (decisions 155/180).
  - Status line: "**Syncing with macbook one**" or "Off". If the device isn't reachable, the status says "macbook one isn't reachable; sync waits", shown **once per time it's away**, in the window only.
  - **Pause** and **Turn off** as before. The tray keeps its quick controls.
- **Recently received:** the last 5 clipboards that arrived here, text or image, with **Copy** and **Put on Shelf** (text).
  - Memory only, cleared on quit.
  - Images show a thumbnail and **Save as file**, so nobody has to dig through screenshot folders.
  - Private items show as "Private text" with no preview.
- **Incoming clipboard** always goes to the clipboard, with a short in-window notice: "**Clipboard from macbook one: ready to paste.**"
- **When a copy is not synced, say so once per copy**, in the window, or as a system notification if the window isn't in front (respect Do Not Disturb / Focus):
  - **Marked private:** "**Not synced: the app you copied from marked it private.**" with **[Put on Shelf]**.
  - **Too big:** "**Too big for the clipboard.**" with **[Send as file]**.
  - **Couldn't be read** (§1.9): "**Couldn't read that copy. Try Send clipboard.**"
- **Put on Shelf from a private copy** amends decision 146: SeamDrop reads the marked copy **only when the person clicks**, and **only if the clipboard's change count is still the one the notice was about**. Otherwise it says "That copy is gone; copy it again." Record the amendment.
- **No sizes or types on screen.** They go to Details only.

### 3.2 The Shelf: shared, kept, deliberate
- **What it is:** a place for text you deliberately keep and paste often (keys, addresses, snippets). One subheading, "**Things you keep, on your devices.**", and nothing else explanatory on the panel.
- **Only what you put there:**
  - typed or pasted into **Add to Shelf**
  - **Put on Shelf** from a private-copy notice or a Recently received item

  Nothing lands on the Shelf by itself. Text only in this mission (multi-line is fine); record the reason.
- **Each item:**
  - an optional **name** ("Vercel key") and a one-line preview
  - small grey "from <device picture> · time"
  - **Copy** as the main action
  - no sizes
- **Private items** (a "Private" switch when adding; on automatically for Put on Shelf from a marked copy):
  - shown as dots
  - **Reveal** (hides again after a few seconds)
  - **Copy** puts it on the clipboard with the private markers
  - never in notifications, logs, Details or previews
- **Sharing (consent and reach):**
  - **Both sides agree**, like clipboard sync: "Share the Shelf with <device>?" is asked at pairing, and once for existing pairings after the update. Off until both say yes.
  - **No relaying.** An item goes from the device where it was added to the devices that share the Shelf directly with that device, and no further. Each device's Shelf shows its own items plus items from devices it shares with directly.
  - **Unsharing or removing a pairing:** that device's items leave here and ours leave there, as decision 227 does today.
  - **Adding on the PC shows on the PC too.** It's storage, not a send.
- **The data model:**
  - Each item has an id.
  - Change stamps follow decision 156: `max(now, last + 1)` with a device-id tie-break, never the bare wall clock.
  - Edits and removals sync. Removals leave markers. Keep the markers long enough that a long-away device can't bring items back (choose and record; small enough to keep a year or more).
  - Undo re-adds the item as a new change.
  - Limits, e.g. 200 items and 64 KB each: adding beyond the limit asks you to remove something first, and items from another device beyond it are refused with a notice.
  - Check everything that arrives: count, size, ids, and stamps far in the future.
- **Kept on disk, encrypted:**
  - It survives quitting, restarts and updates.
  - Encrypted at rest. **State the threat model honestly** in DECISIONS and Help: on the Mac today the pairing keys are a plain 0600 file (`portal-sys` `secret.rs` ~1–18), so a key stored next to the Shelf only stops casual copying, backups and sync tools.
  - Use the Keychain or DPAPI if they work with today's ad-hoc signing without a prompt after every update. Test that and record the result. Otherwise say plainly what protects it.
- **Removing:**
  - **✕** asks: "**Remove 'Vercel key' from the Shelf on your devices?**"
  - **Clear Shelf** is **red**, asks first, and names the devices it clears.
  - **Undo** for a few seconds after either.
- **Tests (loopback):**
  - **a triangle** (A–B–C all sharing) and **a chain** (A–B–C, A and C not paired): add, edit and remove while one is offline, converge, no relaying
  - consent off on one side → nothing shared
  - kept across restart; a removed item never comes back
  - private items leave **no plaintext** in the data folder, the **WebView data folder**, or the logs (grep all three)

---

## 4. Where received files go

**The owner's experience:**
- His `Downloads/SeamDrop` folder was made two weeks ago, so it sits at the bottom of Downloads, and he couldn't find new files without pressing Show in folder.
- On Windows, received files keep the sender's dates (files: `plan.rs` ~30; folders too: `recv.rs` ~575, ~610). So a new file showed as months old and sank in Explorer's date sort.
- When he pointed SeamDrop at Downloads, files arrived loose, which he found fine. He also called the default SeamDrop folder "good, fine" in test 7.
- He asked for two or three clear options, one of them a numbered folder for each delivery.

**Settings → Received files:** one clear choice, each with its folder shown and **Change…**:
1. **Straight into a folder**: files and folders arrive loose. Default folder: Downloads.
2. **In a SeamDrop folder**: `Downloads/SeamDrop` by default, optionally with a subfolder per device.
3. **A new numbered folder for each delivery**: e.g. `SeamDrop 012 – from macbook one`, numbered so they sort in order of arrival.

**Defaults and migration:**
- New installs default to option 1 with Downloads. Record why.
- A computer on the old default `Downloads/SeamDrop` moves to option 1 with Downloads.
- A computer whose folder was **chosen** (the owner's points at Downloads) stays **option 1 with that same folder**.
- Only if "A folder for each device" was on does it become option 2. Say what changed, once.

**Dates:**
- Files **and folders** are dated **when they arrive**, so they're newest in every folder view on both OSes.
- "Keep original dates" is an option, off by default.

**Also:**
- Sent folders keep their own name in every option.
- Folder names are made safe with `fsafe`.
- The first arrival on a computer says where it went, with **Change**.

---

## 5. Small windows and a mini mode

**Seen:** the minimum size is about right. The owner would accept smaller if text went. When the window shrinks, the glowing file icon in the drop area disappears first. He'd rather it was **last**. At really small sizes he'd like a **mini window**: just the icon, super simple, "that's the person's decision".

**Wanted:**
- **The order things drop out as the window shrinks:** helper text first, then device names (pictures stay), then button labels (icons stay). The drop icon stays to the end.
- **Mini mode** below a threshold (choose it, e.g. under 360 px wide or 480 px high):
  - the drop icon, the target device's picture, and a Send clipboard icon
  - dropping still works, and progress shows as a ring around the icon
  - panels ask for a bigger window
- Lower the minimum window size accordingly; record the numbers.
- Check the header fits at the new minimum, with the clipboard, Shelf and Settings icons; Help lives inside Settings.
- Update the overlap check to the new sizes, including mini mode.

---

## 6. Less text everywhere, and one Help page

**Seen across the report:**
- the drop zone's three sentences
- the Shelf's "manual"
- Settings paragraphs ("Every computer on this network that runs SeamDrop sees this name, paired or not", "Ready to paste at once. If this computer keeps a clipboard history…")

"Randomly scattering it about the app is not good." the owner suggested one guide page.

**Wanted:**
- **Help** (in Settings, and a "?" inside panels where it's needed): short, searchable sections in plain words.
  - Pairing
  - Sending files
  - Clipboard and sync
  - The Shelf
  - Privacy (private marks, private windows, what the Shelf's encryption does and doesn't protect, where nothing goes)
  - Where files go
  - Troubleshooting (Connect by IP, firewall, Local Network permission)
  - Supported systems
- **Writing rules for every screen:**
  - A title of at most about 6 words.
  - At most one helper line of about 12 words, only where it prevents a mistake.
  - No paragraphs outside Help.
  - "Not done" messages keep the reason, in one short sentence, plus a button when there's a fix.
- **The drop zone:** the icon, "**Drop files here**", and **Add files · Add folder**.
- **Audit every screen.** Record word counts before and after in PROGRESS.md, with screenshots.

---

## 7. Supported systems (for the owner and Help)

- **Today:**
  - macOS 11 Big Sur and later (Intel and Apple silicon, universal build).
  - Windows 10 and 11 on x64.
  - Windows 11 on ARM runs the x64 build through emulation. Windows 10 on ARM can't.
  - Confirm from `tauri.conf.json` and the builds.
- Write it in Help and README.
- Record whether a native Windows ARM64 build would be cheap; don't build it.
- Older systems aren't worth supporting (very few people, no security updates). Record that.

---

## 8. Getting new builds to the owner (our addition, for his "why can't it just update the app" question)

### 8.1 This Mac: local install
- At the end, once protocol 4 is final (§0), build a release locally.
- Quit SeamDrop, replace `~/Applications/SeamDrop.app`, relaunch, and check About.
- A locally built app has no download flag, so there's **no "Apple could not verify" warning and no Terminal command**.
- macOS may ask for Local Network permission again, because ad-hoc signatures change; say so if it does.

### 8.2 A test-build channel
- **Settings → About → "Get test builds"**, off by default.
- **Tags** `vX.Y.Z-beta.N` publish to `seamdrop-releases` as GitHub **pre-releases**, plus a beta update feed at a **fixed URL compiled into the app**. The stable feed (`releases/latest/…/latest.json`) is never touched.
- **Channel rules:**
  - With test builds on, check **both** feeds and take the higher version, so a beta user still gets stable 0.5.0.
  - Turning test builds off **never downgrades**: you stay on the beta until stable passes it. Keep the default version comparator and never add a custom one.
- **The tooling currently only accepts X.Y.Z** (`release.yml` ~12, `publish-release.mjs` ~17 and its `--latest` ~124, `version.mjs`). Extend it.
- **Check locally that a pre-release version bundles** on macOS (`CFBundleShortVersionString`) and in the Windows NSIS installer **before** spending CI minutes. If the bundle needs a numeric version, map betas to one and record how.
- **You may publish betas. Never publish a stable release.** This mission ends at **0.5.0-beta.1**; 0.4.0 is never published (record).
- **The first beta can't arrive by update,** because today's installs only read the stable feed and have no switch. Under Needs the owner, give exact steps to install beta.1 by hand:
  - on the PC: download from the pre-release page and run the installer
  - on Sam's MacBook Air: the same, plus the Gatekeeper step, `xattr -dr com.apple.quarantine ~/Applications/SeamDrop.app` or Open Anyway

  Then switch on "Get test builds". From beta.2 onwards, updates arrive in the app.
- **If CI minutes run out:** the Mac install still happens. Put the exact command to publish the beta after the reset under Needs the owner.

---

## 9. Definition of done

**Must have:**
1. §1.1–1.9 fixed, each with its cause and a test that fails without the fix (or a note saying why not). This includes the nearby list.
2. Paused within ~3 s on the surviving side, and no false pauses on slow links or re-hashes. The time-left rules and accuracy test pass. No draining speed.
3. Quit / crash → asked. Network / update → continues by itself.
4. A clear busy message and the queue. Nothing says "Ready to send" when it can't send.
5. The receiving done moment matches sending, with safe **Open** handling.
6. Clipboard and Shelf separated as in §3. Sync is switched in one place. Every unsynced copy says why, once.
7. The Shelf is shared by mutual consent, with no relaying, kept and encrypted with an honest threat model; the triangle and chain tests pass.
8. Received-files options, arrival dates for files and folders, and the migration as specified.
9. Help, and the writing rules on every screen, with word counts and screenshots.
10. The local Mac install, and beta.1 published (or the exact command listed). Hand-install steps for the PC and Sam's Mac.
11. CI green on macOS and Windows at the final milestone.

**Should have (time limit §0.6):**
- device pictures
- mini mode
- the pairing-removal choices
- "+" on the home screen

---

## 10. The manual check for the owner

He prefers it **as a plain numbered list in your final message**, not only in a file. Also put it at the top of `docs/MANUAL-TESTS.md`.
- About 25 minutes, ordered by what matters most.
- Each step says what to do, what he should see, and what to send back.
- Use three computers where it matters: discovery with nothing paired, Shelf sharing (triangle), the queue.
- Run the receiving-glow check on Sam's MacBook Air, the slowest machine.

---

## 11. Fresh-eyes review before finishing

Three subagents with no mission, plan or decision files, only: "SeamDrop moves files and your clipboard between your computers, and keeps a shared Shelf." Personas:
- **A non-technical person** with two laptops: first pairing, sending, finding the received file, the Shelf.
- **A developer** copying API keys and env values between a Mac and a PC all day: sync, private items, the Shelf, Recently received.
- **Someone with three computers** and a big transfer that gets interrupted: queue, quit vs drop, time left, discovery.

Write `docs/review-05/SUMMARY.md`, ranked by severity. **Fix everything ranked "blocks use" and "confuses"**, re-check those fixes, and list the rest.

---

## 12. Build order

1. Plan.
2. **§1 fixes**, with the nearby list from §2.2. **CI milestone 1.**
3. **§3 Clipboard and Shelf:** data model, consent and sync first, then screens.
4. **§4 received files**; **§6 Help and the writing rules**; **§7 supported systems**. **CI milestone 2.**
5. **§8.2 beta tooling**, checked locally first.
6. **§11 review**; fix "blocks use" and "confuses".
7. **Should-haves**, within the time limit: §2.1, §2.2's "+", §2.3, §5.
8. **Final CI milestone.** Local Mac install (§8.1). Publish 0.5.0-beta.1. Final summary with the §10 list and Needs the owner.

---

## 13. Future notes (don't build now)
- **Visual pass from the owner's references:** the whole app's look, phone screens, and the website's look, in the design lab.
- **Website:** domain (seamdrop.com is taken; to decide), download page, waitlist.
- **iPhone app** (ahead of Android; the owner has no Android phone):
  - Share sheet
  - a "Send clipboard" action for Back Tap / Shortcuts
  - Recently received and the Shelf when the app opens
  - pairing by QR code
  - needs the Apple Developer account
- **Android app.** SeamDrop Anywhere (relay, paid). Project Send.
- **Shelf images** and pinned files, if asked for.
