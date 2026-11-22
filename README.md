# SeamDrop

SeamDrop moves files and your clipboard between your computers at home, Macs and Windows PCs, as many as you have. Pair each one once by typing the code the other one shows; after that they find each other by themselves, over Wi-Fi or plugged into the router. No account and no cloud: files go straight from one computer to the other, encrypted.

Source available. No licence yet: all rights reserved.

## Install

Download from the releases page: **https://github.com/andrew2grand-lang/seamdrop-releases/releases** (test builds are marked Pre-release)

- Mac (Apple Silicon and Intel): `SeamDrop_<version>_universal.dmg`
- Windows 10 and 11: `SeamDrop_<version>_x64-setup.exe`

**Supported systems:** macOS 11 Big Sur and later (one app for Apple Silicon and Intel); Windows 10 and 11 on x64. Windows 11 on ARM runs the x64 app through its built-in emulation; Windows 10 on ARM can't run it. Older systems aren't supported: they no longer get security updates.

SeamDrop isn't signed by Apple or Microsoft yet, so each computer asks you once whether to open it. Once installed, it keeps itself up to date.

### On a Mac that's yours

1. Open the downloaded `.dmg` and drag **SeamDrop** into **Applications**, as with any app.
2. Open SeamDrop from Applications. macOS says it can't check it for malicious software, because it isn't signed by Apple yet. Click **Done**.
3. Open **System Settings → Privacy & Security**, scroll down to "SeamDrop was blocked…", click **Open Anyway**, and type your Mac password. Then click **Open**. You only do this once.
4. When macOS asks whether SeamDrop may **find devices on your local network**, click **Allow**. (Changed your mind? System Settings → Privacy & Security → Local Network.)

Closing the window keeps SeamDrop running in the menu bar at the top of the screen; **Quit SeamDrop** is in its menu.

### On a work or school Mac (no administrator password)

**Open Anyway** asks for an administrator, so use your own Applications folder and Terminal instead:

1. Open the `.dmg`. In Finder press **⇧⌘H** (your home folder) and open **Applications** there; make a folder called `Applications` if there isn't one. Drag **SeamDrop** into it. (Not into the Applications shortcut in the disk image window: that one needs an administrator.)
2. Open **Terminal** (in Applications → Utilities) and paste this line, then press Return. It tells macOS you chose to open this download; no password is needed:
   ```bash
   xattr -dr com.apple.quarantine ~/Applications/SeamDrop.app
   ```
3. Open SeamDrop and click **Allow** when it asks about your local network. That choice is yours, not your school's.

Keep one copy of SeamDrop on the Mac (delete an old one when you install a new one): macOS's Local Network list gets confused by several copies.

### On Windows

1. Run `SeamDrop_<version>_x64-setup.exe`. Windows says "Windows protected your PC" because SeamDrop isn't signed yet: click **More info → Run anyway**.
2. Click **Yes** when Windows asks to let it make changes. The installer adds two Windows Firewall rules so the Mac can reach this PC: one to receive files, one to find each other. They apply on networks Windows calls Public too, because Windows often labels home Wi-Fi that way.
3. Closing the window keeps SeamDrop running in the system tray (bottom right, maybe behind the ^). **Quit SeamDrop** is in its right-click menu.

Uninstalling (Settings → Apps) removes the firewall rules too.

### Check your download (optional)

Each release lists the SHA-256 checksum of every file (in its notes and in `SHA256SUMS.txt`). On a Mac: `shasum -a 256 ~/Downloads/SeamDrop_*.dmg`. On Windows, in PowerShell: `Get-FileHash $HOME\Downloads\SeamDrop_*.exe`. The numbers should match the release page.

## Using SeamDrop

**Pairing.** On the computer in front of you, press **Pair** next to the other one (Settings → Paired devices → **Pair another computer** once you have one). The other computer asks first: "MacBook Pro wants to pair · **Show code** / **Ignore**". Press Show code there, and type its six digits where you started. A stranger on your network can't finish pairing: they can't see your screen.

**Several computers.** With more than one paired, the window lists them with a tick each: ticked ones are where files and the clipboard go (remembered; the shortcut and the menu use it too). Each one shows whether it's connected; **+** at the end pairs another.

**Sending files.** Drop files and folders on the window, or use **Add files** / **Add folder**. They collect in a list; press **Send** and everything goes as one transfer. The other computer asks first (unless you chose **Always accept** for it). A computer that's busy with another transfer gets them next: they wait, and start by themselves when it's free.

**Receiving files.** Nothing to do on this computer: send from the other one, accept here, and the files land in **Downloads** (Settings → Received files: straight in, in a SeamDrop folder, or a new folder for each delivery). They're dated when they arrive, so they're the newest in the folder. Every file is checked when it arrives.

**If the connection drops** (Wi-Fi, sleep, an update), the transfer pauses within a few seconds and continues by itself from where it stopped when both computers see each other again, for up to 24 hours. **If SeamDrop is quit** (or closes unexpectedly) during a transfer, it asks next time: **Continue** or **Cancel**; the other computer waits and says so. Your files on the sending computer are never touched. **Cancel** asks first and says what it removes.

**The clipboard.** Copy something, then press **Send clipboard** in **Clipboard** (top right), in the menu bar or tray menu, or ⌃⌥⌘C on a Mac / Ctrl+Alt+Shift+C on Windows. It goes to the ticked computers, ready to paste straight away. Copied files are offered as a normal transfer. **Automatic sync** is switched in Clipboard, per computer, and runs only when both turn it on; you can pause it there. Copies made while the other computer is away aren't sent later. Anything an app marks as private, as password managers and private browser windows do, is never sent, and the window says so once, with **Put on Shelf**. **Recently received** lists the last five things that arrived (images can be saved as a file), until SeamDrop quits. Like anything you copy, what arrives can show up in your clipboard history, unless it was marked private.

**The Shelf.** Text you keep on purpose, to copy here or on your other computers: addresses, passwords, snippets you paste often. Nothing lands on it by itself: add it with **Add to Shelf**, or **Put on Shelf** on something that arrived. Tick **Private** and it shows as dots until you reveal it, and copies are kept out of clipboard history. Share it with a paired computer with **Share with …** on the Shelf; both have to say yes, and items aren't passed on to a third computer. It's kept on disk, encrypted (Help → Privacy says exactly what that protects).

**Details.** The **Details** button at the bottom of the window tells what happened, in plain words, newest last. **Copy technical log** there is for support.

**Updates.** SeamDrop checks this project's releases page on GitHub when it opens and every 4 hours, and only installs updates that carry its signature. It never restarts by itself: press **Restart SeamDrop to update** when it suits you, and if a transfer is running it waits for it to finish.

### If they don't find each other

Both computers must be on the same home network (not a guest, school or office network). If the window says "Still looking", it lists what it checked, often with a button that fixes it (on Windows, **Fix it** repairs a firewall rule). The way round almost anything is **Connect by IP**: in Settings (gear, top right) one computer shows its address under "This computer's addresses"; type that address into Connect by IP on the *other* computer. If the Mac's Local Network permission is off, connect **from the PC to the Mac**.

### Coming from Portal

SeamDrop is Portal's new name. The first time SeamDrop opens, it brings over Portal's settings, paired devices and keys, so the two computers stay paired (Details lists what came over). Then:

- **Quit Portal and remove it.** Mac: menu bar → **Quit Portal**, then move Portal from Applications to the Trash. Windows: tray → **Quit Portal**, then Settings → Apps → Portal → Uninstall. If Portal opened at login, SeamDrop takes that over. If Portal is still open, SeamDrop's connection check says so.
- **macOS asks again** to find devices on your local network: SeamDrop is a new app to macOS. Click **Allow**.
- **Update both computers.** Portal and SeamDrop see each other but can't connect; SeamDrop says which computer to update.
- If you used the default receive folder, new files arrive in **Downloads/SeamDrop**; earlier ones stay in Downloads/Portal.
- Portal's data folder is left where it was (Mac: `~/Library/Application Support/com.project1000.portal`, Windows: `%APPDATA%\com.project1000.portal`). It holds Portal's old pairing keys: delete it once SeamDrop works.

## Known limitations

- One transfer at a time with each computer (any number with different computers at once).
- A paused transfer continues for up to 24 hours. Until then, what arrived waits in a hidden `.seamdrop-incoming` folder inside the receive folder; it's removed if the transfer is cancelled or runs out of time.
- Not signed by Apple or Microsoft yet (see Install). Notifications on Windows show only for the installed app.
- On the Mac the private key is a file only you can read (not the Keychain) until SeamDrop has an Apple signature; on Windows it's protected by Windows (DPAPI).
- Project Send is hidden (a developer setting shows its "coming soon" tab): its scanner works (`portal-scan`) and its screens are designed in the lab; conversion, AI and accounts don't exist yet.

## For developers

**Status:** Mission 04 (solid desktop, many devices). See `docs/PROGRESS.md` for what's done and tested, `docs/PLAN.md` for the architecture, `docs/PROTOCOL.md` for the wire protocol (pairing by typed code over SPAKE2, transfers, resume, the clipboard), `docs/DECISIONS.md` for every decision, `docs/MANUAL-TESTS.md` for the Mac + PC checklist, `docs/RELEASING.md` and `docs/SIGNING.md` for releases. Development builds of any commit are under **Actions → build → a run → Artifacts**. The product name is set in one place: `productName` in `src-tauri/tauri.conf.json` (the UI, the Rust core and the scanner all read it from there).

### Develop

Prerequisites: Rust (stable, edition 2024), Node 24. macOS: Xcode Command Line Tools. Windows: Visual Studio Build Tools (C++), WebView2.

```bash
npm install
npm run tauri dev        # the real app (core + window)
npm run dev              # design canvas in a browser: both windows, simulated (http://localhost:5180)
```

In the browser, `http://localhost:5180/?mock=<scenario>&os=mac|win` runs the real app's screens against a fake core (`src/dev/mockCore.ts`; scenarios: searching, searching-long, nearby, revoked, pairing-incoming, online, online-ethernet, offline, disconnected, incompatible, incompatible-self, offer-in, instant, offer-out, sending, receiving, stalled, paused-out, paused-in, resuming, done-in, done-out, failed, cancelled, cancelled-in, blocked-peer, declined, refused). `&update=ready|available|waiting|downloading|failed|offline` shows the update states and `&clip=history|sync|asking|request|private|taken` the clipboard's. Add `&view=accept` for the compact accept window.

**Dev panel:** press <kbd>`</kbd> (backtick) in either the app or the browser. In the app it switches between the **real core** and **Demo mode** (the simulated design states, with jumps to every screen).

**A second device on one machine:** `cargo run -p portal-core --example peer -- --name "Test PC" --auto` starts a headless SeamDrop peer that finds the app, shows a pairing code when the app asks (printed as `PAIR CODE …`, to type in the app) and accepts files (`--send PATH` sends once paired, `--receive DIR` sets where files go, `--connect IP` dials by address).

#### Project scanner (`portal-scan`)

Reads a project and reports what would break on the other OS. It never runs the project's code, scripts or package managers, never reads secret values (key names only), and only goes online with `--online` (PyPI's JSON API, for package builds).

```bash
cargo run --release -p portal-scan -- ~/Projects/app                    # readable summary, checked for the other OS
cargo run --release -p portal-scan -- ~/Projects/app --target macos --json   # versioned JSON report
cargo run --release -p portal-scan -- ~/Projects/app --online           # also ask PyPI which packages must be compiled
```

The JSON is described by `docs/scan-report.schema.json` (generated from the Rust types; `UPDATE_GOLDEN=1 cargo test -p portal-scan --test report` refreshes it and the golden reports). Exit codes: 0 report written, 1 couldn't scan, 2 bad arguments. `--progress` writes one JSON line per step to stderr.

#### Design lab (Project Send)

```bash
npm run lab              # builds the scanner, then http://localhost:5190
npm run lab:demos        # re-scans the demo projects in design-lab/projects into design-lab/data
node design-lab/scripts/lab-shots.mjs   # screenshots of every screen and state, both window styles and themes
```

Every Project Send screen in real SeamDrop windows: the app's own components and tokens, imported, not copied. The panel on the left lists every screen and state, switches window style (Auto picks the computer that shows it in the story), theme and reduced motion, and picks the demo scenario. Press <kbd>\\</kbd> to hide it. **Scan a real folder** runs the real `portal-scan` through a dev-server bridge (`design-lab/server/scanBridge.ts`) and shows its live progress and report. Scans and reports are real; conversion, AI and accounts are designed with demo data and don't exist yet. Nothing in `design-lab/` ships in the app.

#### Build

```bash
npx tauri build --target universal-apple-darwin   # macOS, Apple Silicon + Intel
npx tauri build                                    # Windows (NSIS installer)
```

Mac builds are ad-hoc signed (`bundle.macOS.signingIdentity: "-"`): no Apple account or Xcode needed, and the app runs without administrator rights.

#### Tests

```bash
cargo test -p portal-sys -p portal-core      # core: TLS, pairing, discovery, transfers, file safety, Files from, doctor,
                                              # and tests/wifi.rs: new addresses, dead links, Wi-Fi-like latency and drop-outs
npm test                                      # UI unit tests (core snapshot → screens)
npm run check:bridge                          # every UI command matches a Rust command and its arguments
npm run verify                                # design demo: clicks through every rule (browser)
node scripts/real-shots.mjs <label>           # screenshots of every real-app screen (needs npm run dev)
PORTAL_BIG_MB=10240 cargo test --release -p portal-core --test transfer a_large_file -- --nocapture   # 10 GB over loopback
```

Scanner and lab:

```bash
cargo test -p portal-scan        # every planted problem found, clean projects raise nothing, safety, golden reports, speed
npx vitest run design-lab        # lab: every report matches the schema, every screen is there, the bridge's guards
```

CI: `check.yml` runs the core tests and clippy on Windows for every push that touches Rust; `scan.yml` tests and builds the scanner on macOS and Windows when it changes (the built CLIs are attached to the run); `ui.yml` type-checks, lints and tests the UI and the lab on Linux on every push; `build.yml` builds both apps on `main` when the app changes.

### Where things live

| | |
|---|---|
| `crates/portal-core` | Everything that matters, no Tauri: identity and store, TLS 1.3 with pinned keys (`tls.rs`), protocol (`proto.rs`), pairing (`pairing.rs`), discovery and links (`net/`), transfers (`transfer/`), file-name safety (`fsafe.rs`), connection doctor (`doctor.rs`), and the node that ties it together (`node/`) |
| `crates/portal-sys` | Small OS wrappers: no-replace rename, DPAPI, Windows Firewall / network profile / Controlled Folder Access probes |
| `src-tauri` | The desktop shell: commands and events (`bridge.rs`), tray (`tray.rs`), notifications, windows (`shell.rs`), installer hooks (`windows/hooks.nsh`), `Info.plist` |
| `src` | The UI. `core/` talks to Rust; `engine/` is the design's simulated world (Demo mode); `window/` and `components/` are the design |
| `src/styles/tokens.css` | Every colour, type, spacing, radius, shadow, key and glow token |
| `crates/portal-scan` | The project scanner: walk (`walk.rs`), tree-sitter parsing (`parse.rs`), snapshot of what's there (`snapshot.rs`), rules (`rules/`, every rule's words in `catalog.rs`), tier and headline (`summary.rs`), PyPI check (`online.rs`); fixtures in `tests/fixtures/{broken,clean}` |
| `design-lab` | The Project Send design lab: screens (`screens/`), demo scenarios (`scenarios.ts`, `projects/`, `data/`), the scanner bridge (`server/`) |
| `docs/` | Plans, decisions, progress, manual tests, the scan report schema, third-party notices |

Logs: the Details panel in the window, and `seamdrop.log` in `~/Library/Logs/com.seamdrop.desktop` (macOS) or `%LOCALAPPDATA%\com.seamdrop.desktop\logs` (Windows).

