# Mission 01: The working core

**Product:** "Portal" (working name only, keep it in one config constant so it's easy to rename)
**Owner:** the owner
**Goal of this mission:** turn the existing UI design into a real app that installs on a Mac and a Windows PC, finds the other machine, pairs once, and sends files and folders reliably over Wi-Fi or a direct cable.

Read this whole file before doing anything.

---

## 0. How to work on this mission

1. **Skills first.** List every skill you have available. Load and use all that are relevant, especially anything for Tauri, Rust, frontend/UI design, design systems, accessibility, planning, test-driven development, debugging, verification and code review. List the ones you loaded at the top of `docs/PROGRESS.md`.
2. **Read the existing design project** in this folder (the localhost UI we built). Reuse its components, tokens and states. Do not redesign it in this mission. If its code structure makes integration awkward, restructure the code (not the look) and note why in `docs/DECISIONS.md`.
3. **Work autonomously. Do not stop to ask me questions.** I won't be watching. Make the best decision yourself, record it in `docs/DECISIONS.md` with a one-line reason, and keep going.
4. **Plan, then build.** Write your plan (architecture, crates and packages, the pairing scheme, risks) to `docs/PLAN.md`, then start building straight away. If you disagree with something in this brief, choose the better option, note why in `docs/DECISIONS.md`, and carry on.
5. **Small, verified steps.** Build in the order of section 11. Each step ends with tests passing. Commit after every step with clear messages. Keep `docs/PROGRESS.md` updated (section 13).
6. **If something truly needs me** (testing on my real hardware, a permission dialog, an account login), don't wait. Skip it, continue with everything else, and list it under "Needs the owner" in `docs/PROGRESS.md`.
7. **Be honest in reports.** Separate "tested and works" from "written but untested on real hardware". You can't test Mac ↔ Windows yourself, so give me an exact checklist.

### Decisions already made (don't ask about these)
- **Bundle identifier:** `com.project1000.portal`. Use it everywhere (Tauri `identifier`, Info.plist, mDNS service naming where relevant). Never change it.
- **Money:** spend nothing. Use only free tools, crates and services, and free GitHub Actions minutes.
- **GitHub:** if the `gh` CLI is logged in, create a **private** repo named `portal` and push to it. If not, use a local git repo and add "create the GitHub repo" to "Needs the owner".
- **Admin rights:** yes, the Windows installer may install per-machine (elevated) to add the firewall rule. I'll accept the Windows admin prompt when I install.
- **Start at login:** add an "Open at login" setting, **off by default**.
- **Design:** don't change the visual design. You may add new screens this mission needs (pairing, accept window, doctor, settings, Connect by IP), built from the existing components and tokens, in the same style.
- **Symlinks:** skip them and report them in Details and on the done screen.
- **Mac signing for local tests:** use an Apple Development identity if one is already in the keychain; otherwise ad-hoc sign and note it.

---

## 1. What the product is (short)

A Mac ↔ Windows app. Install it on both machines and they link over Wi-Fi or a direct cable. **Cable = works offline.**

- **Free tier (this mission):** send files and folders between a Mac and a Windows PC. Fast, direct, no account, no cloud.
- **Paid tier (later, not this mission):** "Project Send". A code project arrives converted and ready to run on the other OS, or with a clear report of what blocks it.

The product is **mostly about Windows × Mac.** Build for that pair first and best.

---

## 2. Who it's for and what they want

The free tier competes with free, established tools (LocalSend, PairDrop, Light Share and others). We win on **it just works, it's clear, and it's beautiful**, plus being expert at the Mac/Windows gap.

From competitor support docs and platform documentation, these are the main reasons transfers fail today. **Our app must handle every one of them gracefully:**

| Failure | What users experience | What we do |
|---|---|---|
| Router/guest Wi-Fi blocks devices from seeing each other ("AP/client isolation") | The other device never appears | Suggest it as a *likely* cause, plus a cable or Connect by IP |
| Windows network set to **Public** (including every direct cable, which Windows sees as an "Unidentified network") | Windows blocks incoming connections | Firewall rule that covers Public too; the PC dials out when it can't accept; explain in plain words |
| Windows Firewall **Block** rules (created if the user pressed Cancel on the first prompt, or isn't an admin) | Silent failure forever; the prompt never returns | Detect Block rules for our app and explain how to remove them |
| macOS **Local Network** permission denied (macOS 15+) | App silently can't reach anything | Trigger the prompt early, detect denial heuristically, deep-link to the setting |
| VPN active | Local devices unreachable | Detect a likely VPN and say so |
| Multicast discovery blocked | Device missing | Fall back to broadcast, last-known addresses, manual IP |

**What users expect** (built into this mission where listed in section 3):
- The other device just appears. Pair once, then it's remembered.
- Drag and drop onto the window. Big files and whole folders are fine.
- A clear accept/decline prompt, with "Always accept from this device".
- Real progress, speed, time left, and a clear "done".
- Files land in a sensible place and nothing is ever silently overwritten.
- It stays available to receive without the main window open (menu bar on Mac, system tray on Windows).
- When something's wrong, the app says **what** and **how to fix it**, inside the window.

---

## 3. Scope

### In scope (Mission 01)
1. Tauri v2 desktop app for **macOS (Apple Silicon and Intel)** and **Windows 10/11 (x64)**, using the existing React UI.
2. **Discovery** on the local network, with fallbacks (6.1).
3. **Pairing** with on-screen code confirmation, plus remembered **trusted devices** (6.2).
4. **Roles** Send / Both / Receive, enforced in the core, not just the UI (6.3).
5. **Transfer** of files and folders, encrypted, verified, with progress (6.4).
6. **Cross-OS file safety:** names, junk files, collisions, lengths (6.5).
7. **Link detection:** Wi-Fi vs direct cable, showing "Cable · offline" on a direct link (6.6).
8. **Connection doctor** inside the window (6.7).
9. **Runs in the background:** closing the window hides it; the app keeps receiving from the **menu bar (Mac) / system tray (Windows)**. Single instance only. Incoming offers raise a **native notification** and a badge or state on the tray/menu-bar icon. Clicking either opens a compact accept/decline window. (Accept/Decline buttons inside the notification itself are out of scope: Tauri's notification actions are mobile-only.)
10. **Settings:** device name, where received files go (default: Downloads/Portal), and the trusted devices list (rename and remove).
11. **Details log** in the UI, fed by real core events.
12. **CI builds** for both OSes via GitHub Actions (section 10).

### Out of scope (don't build yet; design the code so they fit later)
- Project Send / conversion engine. Keep the Project tab visible with a calm "Coming soon" state. Never fake it.
- **Choosing between several devices on the network.** Future feature (section 12). The data model must support many peers from day one; the UI shows one paired device for now.
- Login and accounts (not decided yet).
- Resume after interruption (Mission 02), auto-update, code signing and notarization, LocalSend compatibility, clipboard sharing, phones, Linux.

---

## 4. Definition of done

Mission 01 is done when I can do all of this on **my real Mac and Windows PC**:

1. Install from CI-built artifacts on both machines.
2. With both apps running, permissions granted, firewall allowed, and on a home network without client isolation, each device shows the other within **5 seconds**, in 5 out of 5 launches.
3. Pair once using the matching code on both screens. After restarting both apps, they're still paired with no code needed.
4. Send a **10 GB single file** and a **folder of 5,000 small files** in both directions. Both arrive complete and **BLAKE3-verified**, with correct folder structure.
5. Connect an **Ethernet cable directly** between the machines (adapters are fine) and transfer again, **once with Wi-Fi still on and once with Wi-Fi off** on both. The UI shows **"Cable · offline"**. Allow up to 60 seconds after plugging in for addresses to appear.
6. Set the PC to **Send** and try sending to it from the Mac. The transfer is refused, with a clear message on the Mac saying which side to switch.
7. Set Windows' Wi-Fi network to **Public**. The doctor explains the problem and how to fix it, and transfers still work where the PC can dial out.
8. Close the window on the receiver. The offer still arrives as a notification, and clicking it opens the accept window. (Test Windows notifications with the installed build only; in dev they show as PowerShell.)
9. A file named `Résumé: Q3?.pdf` sent Mac → Windows arrives with a safe name, and the Details log says what was renamed and why. Create the test file in Terminal with `touch 'Résumé: Q3?.pdf'` (Finder shows it as "Résumé/ Q3?.pdf").
10. Speed:
    - **Direct gigabit cable** (both adapters gigabit; USB 2 adapters cap far lower), SSDs on both machines: the median of 3 runs of the 10 GB file is **≥ 90 MB/s**, measured over the middle 80% of the transfer. Gigabit tops out around 118 MB/s.
    - **Wi-Fi:** the same file, machines in the same spots, alternating Portal and LocalSend, 3 runs each. Portal's median must be **≥ 90% of LocalSend's**.
    - The 5,000-file folder must not need a network round trip per file.

---

## 5. Architecture and stack

Proposed. Challenge anything you think is wrong, with reasons, in your plan.

- **App shell:** Tauri v2 (latest stable 2.x). The React UI is the front end; **all networking and file I/O live in Rust.** File bytes never pass through the webview. The UI receives throttled events (max ~10 per second per transfer).
- **Core as a separate Rust crate** (e.g. `portal-core`) with no Tauri dependency, so it can be tested headless and reused later. The Tauri layer is a thin bridge of commands and events.
- **Async runtime:** tokio.
- **Discovery:** mDNS/DNS-SD (e.g. the `mdns-sd` crate, pure Rust, no Bonjour install needed on Windows; its README calls it beta, so test it hard). Service type from one constant (e.g. `_portal._tcp`). TXT records: device id, name, OS, protocol version, port. Plus fallbacks (6.1).
- **Transport:** **TLS 1.3 only**, over TCP (`tokio-rustls`). Each install generates a long-lived key pair and self-signed certificate on first run (e.g. `rcgen`). After pairing, peers **pin each other's certificate fingerprint** (see section 8 for how verification must work). Keep transport behind a trait so QUIC (e.g. `quinn`) could be tried later if measurements justify it.
- **Connection direction:** either side can open the TCP connection, whichever way the files flow. Windows allows outbound by default, so when the PC can't accept inbound (Public network, Block rules), **the PC dials the Mac.** Listening on the Mac doesn't need the Local Network permission; dialling out does.
- **Protocol:** our own small, versioned protocol: a control channel with JSON or CBOR messages (hello, offer, accept, decline, cancel, progress, done, error), and data streamed in large chunks with backpressure. Pipeline folder metadata so many small files don't each cost a round trip. Include a protocol version in every hello and refuse incompatible versions with a clear message.
- **Integrity:** BLAKE3 per file, computed while streaming on both sides.
- **Storage (Mission 01):** config and trusted devices in the OS app-data directory. Private key: on macOS a file in app-data with permissions 0600; on Windows a DPAPI-protected file (CryptProtectData). Don't use the macOS Keychain yet: Keychain access is tied to the code signature, and ad-hoc signatures change every build. Move to the Keychain once we have a Developer ID.
- **Interfaces:** enumerate network interfaces (e.g. `if-addrs` or `netdev`) for link detection.
- **Port:** a fixed default port (not LocalSend's 53317), falling back to a free random port if taken. Always advertise the real port via discovery.
- **Versions:** use the latest stable Tauri 2.x and crate releases when you start. Avoid anything released in the last two weeks unless it fixes a bug we hit.

Alternative considered and rejected for now: forking LocalSend (Apache-2.0, Flutter). It would give proven transfers fast, but it means rebuilding our design in Flutter, and its protocol has no pairing step. We may add LocalSend compatibility later (section 12).

---

## 6. Detailed requirements

### 6.1 Discovery
- Register with a **unique mDNS host name** (e.g. `portal-<device-id>.local.`), never the machine's own hostname, to avoid name conflicts with macOS's mDNSResponder and Windows' built-in mDNS.
- Advertise and browse on **all suitable interfaces**, IPv4 and IPv6, explicitly including link-local addresses and the cable interface. Verify the cable interface really is included.
- Test that responses arrive on Windows while its built-in mDNS is also using port 5353.
- Fallbacks, in order: UDP broadcast to each interface's **directed** broadcast address (including 169.254.255.255 on link-local interfaces), not only 255.255.255.255; probing **last-known addresses** of trusted peers; **manual "Connect by IP"** in the UI.
- Show the device within 5 seconds on a normal network. If nothing is found after ~10 seconds, show the doctor's suggestions (6.7), not an empty screen.
- Ignore our own announcements. Treat the same peer seen on two interfaces (Wi-Fi and cable) as **one device with several paths**.

### 6.2 Pairing and trust
- First contact between two unknown devices requires pairing. Both screens show the **same 6-digit code**, and the user confirms on **both** devices.
- The code must be derived from: a commit-then-reveal exchange of fresh random values from each side (like Bluetooth's numeric comparison, so an attacker can't grind keys to produce a matching code), **plus the fingerprints of the certificates actually presented in this TLS session**, not fingerprints sent inside messages. For example: SAS = H(commitment transcript ‖ peer cert fingerprint from this session ‖ our fingerprint ‖ nonceA ‖ nonceB), or derive it from the TLS exporter (RFC 8446 §7.5).
- Don't invent new crypto: use well-reviewed primitives, and explain the scheme in your plan.
- Abort on mismatch. Rate-limit pairing attempts per peer, and don't let unpaired devices raise repeated prompts.
- After pairing, store the peer (id, name, OS, pinned fingerprint, last-known addresses, trusted flag).
- "Always accept from <device>" skips the accept prompt for that device. It's off by default and changeable in Settings.
- Unpaired devices can be seen but can't send without pairing. Removing a device in Settings revokes trust immediately.

### 6.3 Roles
- Send / Both / Receive per device, default Both, persisted.
- Enforced in the core. A device set to Receive can't start a send. A device set to Send rejects offers with a reason code, and the sender's UI shows "<Device> is set to Send. Switch it to Receive or Both."

### 6.4 Transfer
- **Input:** drag and drop onto the window, or a file/folder picker, with multiple items at once.
  - Use Tauri's native drag-drop event (`getCurrentWebview().onDragDropEvent()`, `dragDropEnabled` true) and pass only the absolute paths to Rust. Never read file contents in JS.
  - Replace any HTML5 drop handlers in the design with this event, keeping the existing hover and drop visuals.
  - Declare drag-enabled windows in `tauri.conf.json`. One developer reports the setting being dropped when set from Rust.
  - Don't change window focus during a drag.
- **Offer:** name(s), total size, file count and the sender's role. The receiver checks **free disk space** before accepting and refuses with a clear message if there isn't enough.
- **Progress:** bytes, percent, current speed, time left and the current file. Either side can cancel, which cleans up partial files.
- Stream large chunks from disk with backpressure. Never load whole files into memory.
- **Writing safely:**
  - Write each file into a hidden `.portal-incoming` folder **inside the receive folder**, so it's on the same volume.
  - After the hash matches, move it into place with a **no-replace rename**: `renamex_np(RENAME_EXCL)` on macOS, `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING` on Windows. Rust's `std::fs::rename` silently replaces existing files on both OSes.
  - If the name already exists, pick the next `name (n).ext` and retry.
- Preserve folder structure and modified times. Set folder times after their contents are written.
- **Done:** list of received items, total size, time, average speed and "Show in Finder / File Explorer".
- Every step emits events to the Details log (timestamp, tag, message), matching the design's log style.

### 6.5 Cross-OS file safety (this is our expertise; make it excellent)
Put all of this in one module with unit tests for every rule.

**Paths on the wire:** send relative paths as an **array of components**, never a joined string. Reject components that are empty, `.` or `..`, or that contain NUL, `/` or `\`, and anything that looks like a drive letter or UNC path.

**Going to Windows:**
- Replace forbidden characters: `< > : " / \ | ? *` and control characters.
- Rename reserved names, case-insensitively, checking the base name before the **first** dot (so `NUL.tar.gz` is caught): `CON, PRN, AUX, NUL, COM0–COM9, LPT0–LPT9`, plus the superscript forms `COM¹ COM² COM³ LPT¹ LPT² LPT³`.
- Remove trailing spaces and dots.
- Normalise names to **NFC** (macOS often stores NFD, which displays wrongly on Windows).
- **Long paths:** use std/tokio file APIs with absolute paths (Rust std adds the `\\?\` prefix automatically). Don't build `\\?\` paths by hand, and sanitise names before any path is built. When a received path exceeds 260 characters, log it and note on the done screen that some Windows apps may not open those files.

**Both directions:**
- **Per-name length limits:** 255 UTF-16 units on NTFS, 255 UTF-8 bytes on APFS (a long CJK name from Windows can be too long for the Mac). Truncate on a character boundary, keep the extension, and re-check after adding " (2)".
- **Collision detection after sanitising**, using the target's case-insensitivity and NFC (e.g. `a:b` and `a?b` both become `a_b`), across the whole batch and against existing files.
- **Never overwrite** (see 6.4).
- Skip OS junk by default (a setting, on by default): `.DS_Store`, `._*`, `Icon\r`, `.Spotlight-V100`, `.Trashes`, `.fseventsd`, `.TemporaryItems`, `Thumbs.db`, `desktop.ini`, `$RECYCLE.BIN`, `System Volume Information`.
- Symlinks: never follow them. Skip them and report them.
- Every rename or skip is listed in the Details log and summarised on the done screen ("3 names changed for Windows").

### 6.6 Link detection: Wi-Fi vs cable
- For each peer, keep all working paths (address and interface) and **prefer the fastest**. A direct wired link beats Wi-Fi.
- Classify interfaces as Wi-Fi, Ethernet, Thunderbolt/USB4 bridge, VPN or other. A path counts as a "direct cable" when it reaches the peer over a wired interface with no router (link-local addresses).
- **On a direct cable, prefer IPv6 link-local** (`fe80::…%scope_id`). Every interface gets one within seconds on both OSes. Advertise and connect with the scope ID.
- **For IPv4 169.254 paths,** bind the socket's source address to that interface before connecting (or `IP_BOUND_IF` on macOS). Never rely on the routing table: on macOS, link-local IPv4 is only active on the primary interface, so with Wi-Fi up, traffic can go out the wrong interface. Windows only assigns a 169.254 address after DHCP times out, which can take tens of seconds.
- The UI shows **"Wi-Fi"** or **"Cable · offline"** (direct cable, whether or not internet is available elsewhere).
- If a faster path appears mid-transfer (cable plugged in), switch new chunks to it if practical and show "Switched to the faster link. Works offline." If that's too complex for this mission, use it from the next transfer and tell me.
- **Ethernet cable** (with USB-C adapters if needed) is the **supported** offline path for this mission.
- **Thunderbolt/USB4 between Mac and Windows is an experiment only.** Windows documents USB4 host-to-host networking with link-local IPv4 between PCs, and Macs have Thunderbolt Bridge, but I haven't found confirmation that Mac ↔ Windows works. Make the code ready for it, but don't promise it in the UI.

### 6.7 Connection doctor (inside the window)
When discovery or a connection fails, show a short checklist with plain fixes. Each check shows passed, failed, **likely** or unknown. Never claim certainty you don't have.
- Both devices on a network? Same subnet?
- **macOS Local Network permission:** there's no API to read it. Show "likely denied" when outgoing connections to local addresses fail with "No route to host" (EHOSTUNREACH), or `kDNSServiceErr_PolicyDenied` if using dns_sd. Link to System Settings → Privacy & Security → Local Network. Tip: even when denied, the Mac can still *accept* connections, so "Connect by IP" from the PC to the Mac still works.
- **Windows:** network profile Private or Public (check the cable interface separately; it will usually be Public / "Unidentified network"). Check our firewall Allow rule, and detect **Block** rules for our exe (INetFwPolicy2). Explain that Block rules override Allow and must be removed.
- **Windows Controlled Folder Access:** if the receive folder is in Documents or Desktop and Windows blocks the write, say so.
- **VPN:** on macOS don't treat every `utun*` as a VPN (macOS always has several). Flag a VPN only when a utun/ipsec/ppp interface has an IPv4 address or carries the default route.
- **Router isolation:** if we're on a network with a gateway, see no peers after 10 seconds, and last-known addresses don't answer, say "Your network may be blocking devices from seeing each other", as a likely cause, not a certainty. Suggest a cable or Connect by IP.
- Buttons: **Try again**, **Connect by IP**, **Use a cable (how-to)**.

Write it in the same calm voice as the design. No jargon in the main text; put technical details in Details.

---

## 7. Platform specifics

### macOS
- **Local Network privacy (macOS 15+), per Apple's TN3179:**
  - Ship `NSLocalNetworkUsageDescription` (friendly text) and `NSBonjourServices` (`_portal._tcp`) via `src-tauri/Info.plist`. Verify in CI that both appear in the built app's Info.plist.
  - **Trigger the prompt early** by connecting a UDP socket to a link-local IPv6 address (TN3179's technique). Retry, because the first operation can be denied before the user has answered.
  - The multicast entitlement isn't required on macOS.
  - Apps run from Terminal get access automatically, so **test the bundled `.app`, not only `tauri dev`.**
  - Ad-hoc-signed builds are tracked unreliably by this privacy system. **Sign local Mac test builds with a free Apple Development identity** (Xcode personal team) where possible, and check the main executable has a unique LC_UUID.
- Builds downloaded from GitHub are quarantined. On macOS 15, an unsigned/unnotarised app can only be opened via System Settings → Privacy & Security → **Open Anyway** (Control-click no longer bypasses this). Ad-hoc sign CI builds (`bundle.macOS.signingIdentity: "-"`) so Apple Silicon builds aren't reported as "damaged". Write me the exact steps to open a test build.
- Menu bar item with: status, paired device, "Open Portal", "Quit".

### Windows
- **Firewall:** the installer adds a program-based Allow rule (TCP and UDP, including mDNS on 5353) for **Domain, Private and Public** profiles. This needs a per-machine (elevated) install, which is approved. Use Tauri's NSIS installer hook scripts, and remove the rule on uninstall. If the rule can't be added, the doctor guides the user.
- **Assume the cable network is Public.** A switch to Private may not survive a reboot, so the design must work without it (Allow rule on Public, plus the PC dialling out).
- Unsigned builds trigger SmartScreen ("More info → Run anyway"). That's fine for testing; write me the steps.
- WebView2 is built into Windows 11. Make sure the installer handles Windows 10 machines without it.
- System tray icon with the same menu as the Mac menu bar.

### Both
- Closing the window hides it (intercept close, prevent exit when the last window closes). "Quit" in the tray/menu bar is the only real exit.
- `tauri-plugin-single-instance`: a second launch focuses the existing window instead of starting a second core.
- Starting at login (`tauri-plugin-autostart`): an "Open at login" setting, off by default.

---

## 8. Security
- TLS 1.3 only. Peers are authenticated by pinned fingerprints after pairing.
- **Custom certificate verifiers** (server and client) must:
  - compare the pinned fingerprint in constant time, **and**
  - verify the handshake signature with the crypto provider's `verify_tls13_signature`.
  
  Checking only the fingerprint would let anyone who replays the peer's public certificate pass without its private key. Include a unit test where a forged certificate with the right fingerprint but the wrong key is rejected.
- Never accept data from an unpaired device. Validate every message and size. Reject path traversal as described in 6.5, so nothing can be written outside the receive folder.
- No telemetry and no internet calls in this mission.

---

## 9. Testing

**Automated (you run these):**
- Unit tests:
  - every filename rule in 6.5
  - protocol encode/decode
  - roles
  - pairing code derivation
  - path-traversal rejection
  - the forged-certificate test
  - no-replace rename under a race
- Integration test: two core instances on one machine over loopback with different ports. Send a single large file, a deep folder, a folder with nasty names, and 5,000 small files. Verify hashes and results.
- Fault tests: cancel mid-transfer on either side; disk full; peer disappears mid-transfer. Each must clean up and show a clear error.

**Manual (I run these on my Mac and PC):** give me a checklist matching section 4, with exact steps, what I should see, and how to collect logs if something fails.

---

## 10. Build and workflow
- Code lives in a **private GitHub repo** (single source of truth). I develop on the Mac with Claude Code.
- Add a **GitHub Actions workflow using `tauri-action`** that builds macOS Apple Silicon, macOS Intel and Windows x64, on every push to `main` and on demand. Artifacts must be downloadable, so I can install the Windows build on the PC without a Windows dev setup.
  - GitHub retired the `macos-13` Intel runner in December 2025. Build Intel Macs by cross-compiling (`--target x86_64-apple-darwin`) on an Apple Silicon runner, or ship a universal binary (`universal-apple-darwin`). Check GitHub's current runner labels rather than assuming.
- No signing secrets yet. Ad-hoc sign macOS builds as Tauri's docs recommend.
- README: how to run in dev, how to build, how to install test builds on each OS, and known limitations.

---

## 11. Build order inside this mission
1. Plan + skeleton: Tauri app running the existing UI on both OSes, plus the `portal-core` crate with tests.
2. Discovery on Wi-Fi + "Connect by IP".
3. Pairing + trusted devices + settings.
4. Transfer of a single file, then folders, with hashing, safe writes and progress, wired to the existing UI states.
5. Cross-OS file safety module.
6. Roles enforced end to end.
7. Link detection + direct Ethernet cable (IPv6 link-local first).
8. Connection doctor.
9. Background running: hide-on-close, single instance, menu bar/tray, notifications, compact accept window.
10. CI builds + README + my manual test checklist.

Keep the design's hidden **dev panel** working with a **Demo mode** that replays mock states, so design work can continue without two machines.

---

## 12. Future notes (don't build now; don't block them)
- **Choose a device on the network:** more than one paired device, with a picker. The product stays mostly about Windows × Mac. The core must already treat peers as a list.
- **Project Send:** the conversion engine (Python first), blocker reports and hand-off plans. Keep a clean boundary so the engine plugs into the pipeline after a project is received.
- **Resume** interrupted transfers (Mission 02).
- **Signing and notarisation** for launch: Apple Developer Program ($99/year) for Developer ID and notarisation, then move the key to the Keychain. Windows signing, e.g. Azure Artifact Signing (per recent reporting: organisations in the UK/EU, individuals only in the US/Canada).
- Auto-update, LocalSend protocol compatibility (to receive from phones), clipboard sharing, login/accounts (undecided).

---

## 13. Reporting
Keep `docs/PROGRESS.md` updated after every build-order step with:
- What works and how you verified it
- What's written but untested on real hardware
- **Needs the owner:** things only I can do (with exact steps)
- Next step

When the mission is finished, or you can't go further without me, end with a final summary: what's done, the CI build links, my manual test checklist (section 4), and the "Needs the owner" list.

---

## Sources behind this brief (checked October 2026)
- LocalSend repo (license, ports, troubleshooting): https://github.com/localsend/localsend
- LocalSend protocol v2: https://github.com/localsend/protocol
- Apple TN3179, Local Network privacy: https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy
- macOS Local Network privacy overview: https://eclecticlight.co/2025/03/10/manage-privacy-protection-for-network-devices-and-others/
- Apple forums, Sequoia local network and multicast: https://developer.apple.com/forums/thread/757824
- Apple forums, link-local IPv4 with Wi-Fi up: https://developer.apple.com/forums/thread/725414
- Windows Firewall rules (Block overrides Allow): https://learn.microsoft.com/windows/security/operating-system-security/network-security/windows-firewall/rules
- Firewall prompt "Cancel" creates Block rules: https://blog.rebex.net/beware-pressing-cancel-on-windows-firewall-security-alert
- Direct Ethernet treated as Public / Unidentified network: https://yaleits.atlassian.net/wiki/spaces/~howard.gilbert/pages/5165842453/Unidentified+Network+private+Ethernet+treated+as+Public
- Windows file naming rules: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
- Rust `std::fs::rename` replaces existing files: https://doc.rust-lang.org/std/fs/fn.rename.html
- Rust std verbatim paths on Windows: https://rust.googlesource.com/rust/+/HEAD/library/std/src/sys/path/windows.rs
- Windows USB4 host-to-host networking: https://learn.microsoft.com/en-us/windows-hardware/design/component-guidelines/usb4-interdomain-connections
- Intel Thunderbolt Share (no Mac support): https://9to5mac.com/2024/05/16/intel-thunderbolt-share-mac/
- Tauri releases: https://v2.tauri.app/release/tauri/
- Tauri notification plugin (actions are mobile-only): https://v2.tauri.app/plugin/notification/
- Tauri macOS signing (ad-hoc): https://v2.tauri.app/distribute/sign/macos/
- Tauri GitHub Actions pipeline: https://v2.tauri.app/distribute/pipelines/github
- Tauri NSIS installer hooks: https://v2.tauri.app/release/tauri-bundler/v2.0.1-beta.15/
- Tauri drag-and-drop (native event): https://react-dropzone.js.org/guide/tauri/
- Tauri drag-and-drop lessons on Windows: https://dev.to/devrayat000/building-a-transparent-drag-and-drop-shelf-for-windows-with-tauri-the-engineering-war-stories-3c8c
- GitHub macos-13 runner retirement: https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/
- mdns-sd crate: https://github.com/keepsimple1/mdns-sd
- macOS Sequoia Gatekeeper "Open Anyway": https://www.macworld.com/article/2457844/what-to-do-when-you-cant-open-an-app-you-just-installed-in-macos-sequoia.html
- Apple Developer Program cost: https://flaviocopes.com/apple-developer-free-vs-paid/
- Azure Artifact Signing price and eligibility: https://devclass.com/2026/01/14/code-signing-windows-apps-may-be-easier-and-more-secure-with-new-azure-artifact-service/
- Unicode NFD/NFC filename issue Mac → Windows: https://br.bandisoft.com/bandinamer/
