# Mission 01 plan: the working core

Spec: `MISSION-01-working-core.md` (Downloads; copied to `docs/MISSION-01.md`). Decisions that change or add to it: `docs/DECISIONS.md`. Status: `docs/PROGRESS.md`.

**Goal:** turn the design prototype into a Tauri v2 app for macOS (Apple Silicon + Intel) and Windows 10/11 x64 that finds the other machine, pairs once, and sends files and folders reliably over Wi-Fi or a direct Ethernet cable.

**Update 1 (2026-10-24):** Wi-Fi is the product and the cable is deprioritised: the direct-link parts below still run, silently, but nothing in the UI or docs offers or describes a cable. The work moved to the same-network path (PC wired to the router, Mac on its Wi-Fi): parallel dialling, network-change and sleep handling, a unicast sweep when the router filters multicast and broadcast, clean failure mid-transfer, keep-awake. See `DECISIONS.md` 58–73 and `PROGRESS.md` → Update 1.

**Execution:** native (I build every step myself in this session, TDD per module), one independent review at the end. The brief forbids stopping for questions, so the plan's handoff question is replaced by `DECISIONS.md`.

---

## 1. Architecture

```
┌──────────── React UI (existing design, src/) ────────────┐
│  PortalProvider ── one interface, two backends:           │
│    SimProvider   demo mode / browser canvas (unchanged)   │
│    CoreProvider  real mode: invoke() + listen()           │
└───────────────▲──────────────────────────┬───────────────┘
   events (≤10/s per transfer)             │ commands (paths only, never bytes)
┌───────────────┴──────── src-tauri (thin bridge) ─────────▼──────────────┐
│ commands.rs  events.rs  tray.rs  notify.rs  windows.rs (close→hide,      │
│ accept window)  single-instance  autostart  dialog  opener               │
└───────────────▲──────────────────────────┬──────────────────────────────┘
                │ CoreEvent (tokio mpsc)   │ Core API (async fns)
┌───────────────┴──── crates/portal-core (no Tauri) ───────▼──────────────┐
│ node      actor that owns state: peers, trust, settings, transfers      │
│ net/      iface (classify), discovery (mDNS + broadcast + known addrs), │
│           dial (scoped v6, bound v4), listener, link (persistent TLS)   │
│ tls       rustls 1.3 configs + pinned verifiers                         │
│ pairing   commit-then-reveal SAS, rate limits                           │
│ proto     frames + versioned JSON control messages                      │
│ transfer/ sender (walk, manifest, stream), receiver (stage, verify,     │
│           no-replace finalize)                                          │
│ fsafe     cross-OS name rules (one module, unit test per rule)          │
│ doctor    checklist from probes + heuristics                            │
│ store     identity, settings, trusted peers (app-data JSON)             │
└───────────────▲─────────────────────────────────────────────────────────┘
┌───────────────┴──── crates/portal-sys (OS glue, no C deps) ─────────────┐
│ rename_noreplace (renamex_np / MoveFileExW), DPAPI protect, 0600 files,  │
│ Windows firewall rules + network profiles (COM), CFA flag               │
└─────────────────────────────────────────────────────────────────────────┘
```

- **All networking and file I/O in Rust.** The webview only ever sends absolute paths and receives events.
- `portal-sys` has no C build dependencies, so `cargo check -p portal-sys --target x86_64-pc-windows-msvc` runs on the Mac and catches Windows FFI mistakes before CI.
- `portal-core` is tested headless: two `Node`s on one machine over loopback.

### Crates and packages (pinned, each ≥ 2 weeks old on 2026-10-24)

| Purpose | Crate |
|---|---|
| App shell | tauri 2.12.2, tauri-build 2.7.1, plugins: single-instance 2.5.2, autostart 2.7.0, notification 2.5.1, dialog 2.8.1, opener 2.7.0 |
| Async | tokio 1.53.2 |
| TLS 1.3 | rustls 0.23.45 (`ring` provider, **no `tls12` feature**), tokio-rustls 0.26.6, rcgen 0.14.10, ring 0.17.14, subtle 2.6.1 |
| Discovery | mdns-sd 0.21.5, netdev 0.46.3 (interface types incl. macOS SystemConfiguration), socket2 0.6.5 |
| Integrity | blake3 1.8.7 |
| Files | unicode-normalization 0.1.25, filetime 0.2.29, fs4 1.1.0 (free space), dirs 7.0.0 |
| OS glue | libc 0.2.190, windows 0.62.2 |
| Misc | serde 1.0.229, serde_json 1.0.151, thiserror 2.0.21, tracing 0.1.44 |
| JS | @tauri-apps/cli 2.12.1, @tauri-apps/api 2.12.2, matching plugin packages; vitest 5.0.3 |

## 2. Identity and storage

- `device_id`: 16 random bytes, lowercase hex (32 chars). Generated once.
- Key pair: ECDSA P-256 via rcgen; self-signed cert valid until 4096. `fingerprint = SHA-256(cert DER)`.
- Files in the OS app-data dir (`~/Library/Application Support/com.project1000.portal`, `%APPDATA%\com.project1000.portal`): `identity.json` (id, cert), `key.bin` (PKCS#8; macOS mode 0600; Windows DPAPI `CryptProtectData`, user scope), `settings.json`, `peers.json`. Writes are atomic (temp file + rename).
- Settings: device name (default: OS computer name), receive folder (default `Downloads/Portal`), role (default Both), skip junk (default on), open at login (default off; source of truth is the autostart plugin).
- Trusted peer: `{ id, name, os, fingerprint, addrs (last known, ≤ 8), trusted, always_accept (default off), paired_at }`.

## 3. Transport, links and protocol

- **TLS 1.3 only**, ALPN `portal/1`, mutual certificates. Custom verifiers on both sides always run `rustls::crypto::verify_tls13_signature` against the presented cert, and in pinned mode compare `SHA-256(cert)` with the pinned value using `subtle::ConstantTimeEq`. A forged cert carrying the right DER but the wrong key fails the signature check (unit test).
- **Persistent link per trusted peer.** Whichever side can dial does; Windows can always dial out, so a PC on a Public network still links to the Mac. Every message and transfer for that peer uses the link. Duplicate links (both dialled): the peer with the smaller device id decides which to keep and tells the other (`Superseded`), so the two sides never disagree.
- **Path upgrade:** when a strictly better path appears (e.g. cable), the link is re-dialled over it once the link is idle. Mid-transfer switching is out of scope for Mission 01 (see DECISIONS); the switch happens before the next transfer and the UI shows the new link.
- **Frames:** `[kind u8][len u32 BE][payload]`. `kind 1` = control (JSON, ≤ 4 MiB), `kind 2` = file data (≤ 1 MiB). One writer task per link with a biased select: control frames jump ahead of data.
- **Control messages** (`t` tag): `hello{proto, id, name, os, role, app}`, `error{code, msg}`, `pair_request`, `pair_commit{c}`, `pair_nonce{n}`, `pair_reveal{n}`, `pair_confirm{ok}`, `status{role, name}`, `ping`, `pong`, `superseded`, `offer{tid, items[], total_bytes, file_count, skipped[], sender_role}`, `accept{tid}`, `decline{tid, reason}`, `dirs{entries[]}`, `file{idx, path[], size, mtime}`, `file_end{idx, blake3}`, `done{tid}`, `complete{tid, summary}`, `progress{tid, bytes}`, `cancel{tid, reason}`.
- **Version:** `PROTOCOL_VERSION = 1` in every hello; a different major version gets `error{code: "incompatible_version"}` and the UI says which side to update.
- **Untrusted peers** may only send `hello` and pairing messages. Anything else closes the connection.

## 4. Pairing (numeric comparison, commit-then-reveal)

I = initiator (TLS client), R = responder (TLS server). `fp_I`, `fp_R` are taken from the TLS session on each side (own cert + `peer_certificates()`), never from messages. `X = TLS exporter("EXPORTER-portal-pairing-v1", no context, 32 bytes)`.

1. I → R `pair_request`.
2. R picks `Nr` (32 random bytes), sends `pair_commit{ C = SHA-256("portal/pair/commit/v1" ‖ Nr) }`.
3. I picks `Ni`, sends `pair_nonce{Ni}`.
4. R sends `pair_reveal{Nr}`; I checks the commitment or aborts.
5. Both: `SAS = SHA-256("portal/pair/sas/v1" ‖ X ‖ fp_I ‖ fp_R ‖ Ni ‖ Nr)`, first 8 bytes as big-endian u64 mod 10⁶, shown as `123 456`.
6. Each user confirms; each side sends `pair_confirm{ok}`. Trust is stored only after both confirmations.

Why it holds: R commits to `Nr` before seeing `Ni`, and I reveals `Ni` before seeing `Nr`, so a man in the middle running two sessions cannot steer either code; his chance of matching codes is 1 in 10⁶ per attempt, and attempts are rate-limited. The fingerprints bind the code to the certificates actually used (the TLS 1.3 exporter alone does not cover the client certificate, hence `fp_I` explicitly). Primitives: TLS 1.3 (rustls), SHA-256 and CSPRNG (ring). Nothing new.

Rate limits: one pairing at a time; per device id and per IP at most 3 attempts per 10 minutes; after a cancel or mismatch the same device cannot raise a prompt for 60 s.

## 5. Discovery (6.1)

- mDNS (`_portal._tcp.local.`), instance name = device id, host `portal-<id>.local.`, `enable_addr_auto()` on all interfaces (IPv4 + IPv6 incl. link-local; mdns-sd uses if-addrs with `link-local`). TXT: `id`, `name`, `os`, `proto`, `port`.
- UDP fallback on the same port number: JSON announce to every IPv4 interface's **directed** broadcast (incl. `169.254.255.255`), `255.255.255.255`, and IPv6 `ff02::1%scope` per interface; every 2 s for 10 s, then every 10 s and on interface changes; replies unicast.
- Known-address probing for trusted peers every 15 s while not linked; manual Connect by IP (`192.168.1.20`, `fe80::1%en7`, optional `:port`).
- Peer registry keyed by device id; several paths per peer (address, local interface, kind, source, last seen); own id ignored.
- 10 s with nothing found → the stage shows the doctor's suggestions, not an empty screen.

## 6. Link detection (6.6)

- Classify interfaces: Wi-Fi, Ethernet, Thunderbolt/USB4 bridge, VPN, other (netdev types; macOS VPN only if a `utun/ipsec/ppp` interface has IPv4 or the default route).
- Direct link = wired interface without a router + link-local peer address. Preference: direct IPv6 link-local > direct 169.254 > routed Ethernet > Wi-Fi > other > VPN; through a router IPv4 before IPv6 (Update 1).
- IPv6 link-local dials use our interface index as scope id. IPv4 169.254 dials bind the source address first (+ `IP_BOUND_IF` on macOS).
- UI link pill: `Wi-Fi`, `Ethernet`, or `Network`. Since Update 1 a direct link is also shown as `Ethernet` (see DECISIONS 58).

## 7. Transfer (6.4)

- Sender walks the inputs once (never follows symlinks; skips junk if the setting is on), builds the offer (top-level items, totals, skipped list).
- Receiver: role check → free-space check (total + 64 MiB margin) → prompt or auto-accept → stream.
- Stream: `dirs` batch, then per file `file` header, data frames, `file_end{blake3}` — no round trip per file. Sender reads 1 MiB chunks on a blocking thread, hashing as it reads; receiver hashes as it writes on a blocking thread. Bounded channels give backpressure end to end. `progress` from the receiver every 250 ms doubles as a heartbeat; 30 s of silence aborts.
- Safe writes: everything lands in `<receive>/.portal-incoming/<tid>/` (same volume). After the last `file_end` matches, each top-level item moves into place with a **no-replace rename**; on `AlreadyExists` the next `name (n).ext` is tried. Folder mtimes are set after their contents. Cancel, disconnect, disk-full or hash mismatch delete the staging folder, so nothing partial is ever visible.
- Done summary: items, bytes, seconds, average speed, renamed/skipped notes, long-path note, "Show in Finder / Show in File Explorer".

## 8. File safety (6.5) — `portal-core::fsafe`

Pure functions, a unit test per rule: wire-path validation (components; reject empty, `.`, `..`, NUL, `/`, `\`, drive letters, UNC), Windows forbidden chars and control chars → `_`, reserved names (before the first dot, case-insensitive, incl. `COM¹²³`/`LPT¹²³`), trailing dots/spaces, NFC, per-target name length (255 UTF-16 on NTFS, 255 UTF-8 bytes on APFS, char-boundary truncation keeping the extension, re-checked after ` (n)`), batch + on-disk collision detection under case-folding + NFC, junk list, symlink skip. Every change produces a `Rename { from, to, reason }` note for the log and summary.

## 9. Doctor (6.7)

Checks with status `pass | fail | likely | unknown`: on a network / same subnet as last-known peer address; macOS Local Network (likely denied after `EHOSTUNREACH` dialling local addresses); Windows network profile per interface (cable usually Public), our Allow rule present for all profiles, Block rules for our exe (INetFwPolicy2); Controlled Folder Access (after an access-denied write under Documents/Desktop); VPN; router isolation (gateway present, no peers after 10 s, known addresses silent). Actions: Try again, Connect by IP, Use a cable (how-to). Technical detail goes to Details.

## 10. App shell (7)

- Main window 552×780, min 420×640. macOS: overlay title bar with native traffic lights (design's drawn lights hidden). Windows: undecorated window with the design's own caption buttons wired to minimise/maximise/hide. Drag-drop enabled in `tauri.conf.json` only for `main`.
- Close hides; `Quit` in tray/menu bar is the only exit; single instance focuses the existing window.
- Tray/menu bar: status line, paired device line, `Open Portal`, `Quit`; icon shows a dot while an offer waits.
- Offer while hidden: native notification (macOS: tauri notification plugin + app-activation observer; Windows: WinRT toast with `on_activated`), clicking opens the compact accept window (`index.html?view=accept`).
- `Info.plist`: `NSLocalNetworkUsageDescription`, `NSBonjourServices = [_portal._tcp]`. Local-network prompt triggered at launch via a UDP connect to a link-local IPv6 address, retried.
- Windows NSIS per-machine install, hook script adds/removes program-based firewall Allow rules (TCP+UDP, all profiles). WebView2: embedded bootstrapper for Windows 10.

## 11. UI integration

- Restructure, not redesign: one `PortalContext` (`world`, `dispatch`, `progress`, `rate`) with two providers. The existing stage views keep their markup; `devices` moves into the world so real peers render through the same components.
- New screens from existing parts (Pane, Head, StateTile, Key, Checklist, Chip, Checkbox) plus one new `TextField` built from tokens: searching, nearby/pair, pairing code, offline, doctor, settings, Connect by IP, Project "coming soon", transfer failed.
- Real drag-drop via `getCurrentWebview().onDragDropEvent()` (paths only), keeping the hover/drop visuals; click opens the native file picker.
- Demo mode: dev panel toggle (backtick) swaps CoreProvider for SimProvider; browser `npm run dev` keeps the two-window canvas.

## 12. Testing

- Unit: every fsafe rule, protocol encode/decode, roles, SAS derivation + commitment, path traversal, forged certificate, no-replace rename race (threads racing to the same target: exactly one wins).
- Integration (`crates/portal-core/tests`): two nodes on loopback, different ports, temp dirs: pair, single large file (1 GiB in CI; 10 GiB opt-in), deep folder, nasty names, 5,000 small files; hashes verified. Faults: cancel by sender, cancel by receiver, disk full (tiny quota via injected writer limit), peer vanishes mid-transfer.
- UI: `npm run verify` / `shots` keep working in demo mode; vitest for the core→view mapping.
- CI: `check.yml` (Windows `cargo test` + clippy on every branch push), `build.yml` (tauri-action: `universal-apple-darwin` on `macos-latest`, Windows x64 on `windows-latest`; push to `main` + manual).

## 13. Steps (mission §11) and their verification

| # | Step | Done when |
|---|---|---|
| 1 | Plan + skeleton | `cargo test` green; `tauri build` produces a running `.app` showing the UI; Windows CI check green |
| 2 | Discovery + Connect by IP | two loopback nodes discover each other (mDNS + broadcast tests), manual connect returns peer identity; UI shows searching / nearby |
| 3 | Pairing, trust, settings | SAS tests, pairing integration test, restart keeps trust, removal revokes; pairing + settings screens |
| 4 | Transfer | integration tests for file, folder, 5,000 files, cancel, disk full, peer loss; UI states wired |
| 5 | File safety | rule tests; nasty-names integration test; renames in log + done screen |
| 6 | Roles | core rejects with reason codes; blocked views driven by real events |
| 7 | Link detection + cable | classification tests; scoped dial tests on loopback/link-local; pill shows cable |
| 8 | Doctor | check logic unit tests with fake probes; doctor screen |
| 9 | Background | close→hide, single instance, tray, notifications, accept window (manual run on Mac) |
| 10 | CI + README + checklist | both workflows green, artifacts downloadable, `docs/MANUAL-TESTS.md` |

## 14. Risks

| Risk | Mitigation |
|---|---|
| mdns-sd is beta; Windows' own mDNS on 5353 | Broadcast + known-address + manual fallbacks; SO_REUSEADDR in mdns-sd; manual check on the owner's PC |
| Can't run Windows locally | `portal-sys` cross-checks on the Mac; Windows CI runs the full core test suite |
| macOS Local Network privacy flakiness with ad-hoc signatures | Trigger early + retry; doctor heuristic; PC→Mac dial always works; Apple Development signing listed under Needs the owner |
| Windows notification click needs installed AUMID | WinRT toast with `on_activated` + tray fallback; test installed build only |
| 10 GB / 90 MB/s target | 1 MiB chunks, hashing on both ends off the async threads, biased writer; local loopback benchmark; real figures need the owner |
| Free CI minutes (macOS = 10×) | macOS only on `main`/manual, single universal job, rust-cache |
| Link-local IPv4 on macOS only on the primary interface | Prefer IPv6 link-local on cable; bind source + `IP_BOUND_IF` for 169.254 |
