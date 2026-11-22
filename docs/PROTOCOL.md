# SeamDrop wire protocol (version 4)

What two SeamDrop devices say to each other, written so another implementation (a phone, say) can be built from it. The reference implementation is `crates/portal-core`; where this document and the code disagree, the code is wrong or this document is, and either is a bug.

Everything here is implemented in `crates/portal-core`; a phone app can implement all of it from this document (nothing requires linking `portal-core`). §9 says what a relay would change.

## 1. Connection

- **Transport:** TCP, default port 47920 (any port works; the hello says which one to dial back).
- **TLS 1.3 only**, ALPN `portal/1`, SNI `portal` (names aren't checked).
- **Both sides present a certificate** (mutual TLS). Each device has one long-lived key pair (ECDSA P-256, self-signed certificate). Its **fingerprint** is SHA-256 of the certificate's DER encoding. A device is identified by that key, never by its name or device id (both are just claims).
- **First contact** (no pairing yet): any certificate whose CertificateVerify signature checks out is accepted; trust comes from pairing. **A paired device** is pinned: its certificate's SHA-256 must equal the stored fingerprint (constant-time comparison), as well as the signature checking out.
- **Session binding:** after the handshake, both sides compute a 32-byte TLS exporter: RFC 8446 §7.5 exporter with label `EXPORTER-portal-pairing-v1`, no context. Pairing uses it (§3).

## 2. Frames and hello

Everything after the TLS handshake is frames: `[kind: u8][len: u32 big-endian][payload]`.

| kind | payload | limit |
|---|---|---|
| 1 | a control message, JSON (UTF-8) | 4 MiB |
| 2 | file data for the file being streamed | 1 MiB |

A control message is a JSON object with a `t` field naming its type (snake_case) and its fields beside it, e.g. `{"t":"pair_wrong","tries_left":2}`. A message type a device doesn't know decodes as "unknown" and is ignored (forward compatibility). 32-byte values travel as 64 lowercase hex digits; a SPAKE2 share as 130.

**Hello.** Both sides send `hello` first:

```json
{"t":"hello","proto":4,"id":"<32 hex>","name":"MacBook Pro","os":"macos","role":"both","app":"0.4.0","port":47920}
```

A device that speaks another `proto` gets `{"t":"error","code":"incompatible_version","msg":…}` and is disconnected. Version 4 is this document (Mission 05 added §5's liveness, §7's `hold` and `kept`, §6's `queue_head`, §11 the Shelf and §12 pictures). There is no negotiation and no fallback.

## 3. Pairing by typed code

Two devices that don't trust each other yet become paired: each stores the other's fingerprint (from the TLS session, never from a message). One device (the **asker**: where the person pressed Pair) asks the other (the **shower**); the shower's person agrees and the shower shows a fresh random 6-digit code; the person types it on the asker. The code goes in only through a PAKE: **SPAKE2 as specified in RFC 9382**, ciphersuite SPAKE2-P256-SHA256-HKDF-HMAC. Nothing derived from the code travels, so it can't be tested offline: each guess costs one exchange with the shower, and the shower limits them.

### 3.1 Roles

- SPAKE2 **A is the TLS client, B is the TLS server**, whichever device asked. There is no symmetric mode.
- A uses M and sends cA; B uses N and sends cB (RFC 9382 §3.3, §4).

### 3.2 Inputs

- **Code:** 6 decimal digits, `000000`–`999999`, each equally likely (rejection sampling from a CSPRNG). Shown as two groups of three ("048 213"). When typed, spaces and hyphens are removed; anything but exactly six ASCII digits is refused before any message is sent (and isn't a try).
- **w** (RFC 9382 §3.2 "the method used to compute w"):
  `w = OS2IP(HKDF-SHA256(salt = "SeamDrop pairing v1", IKM = the six ASCII digits, info = "w", L = 48)) mod p`
  where p is the order of the P-256 group. 48 bytes is 128 bits longer than p, so the bias is negligible. There is no memory-hard function: the code is used for a single exchange and never travels in any form.
- **Identities and AAD** (both from the TLS session):
  - `A = "SeamDrop pairing v1" ‖ " " ‖ "A" ‖ fp_client ‖ exporter`
  - `B = "SeamDrop pairing v1" ‖ " " ‖ "B" ‖ fp_server ‖ exporter`
  - `AAD = "SeamDrop pairing v1" ‖ exporter ‖ fp_client ‖ fp_server`

  `fp_client` / `fp_server` are the 32-byte fingerprints of the TLS client's and server's certificates in this session; `exporter` is §1's 32 bytes. Both fingerprints and the exporter are in the identities, so in the transcript TT and Ke/Ka, and in AAD, so in KcA/KcB. A man in the middle has two TLS sessions (other fingerprints, other exporters), so both sides' keys differ and both confirmations fail.
- **Encoding:** points as uncompressed SEC1 (65 bytes, `04 ‖ x ‖ y`). A received share must be exactly that, on the curve, and not the identity; otherwise the exchange stops. K must not be the identity either.

Everything else is RFC 9382: `pA = w·M + x·P`, `pB = w·N + y·P` with x, y uniform in [1, p); `K = x·(pB − w·N)` / `y·(pA − w·M)` (cofactor 1); `TT = len(A)‖A‖len(B)‖B‖len(pA)‖pA‖len(pB)‖pB‖len(K)‖K‖len(w)‖w` with 8-byte little-endian lengths and w as 32 bytes big-endian; `Ke ‖ Ka = SHA-256(TT)`; `KcA ‖ KcB = HKDF-SHA256(IKM = Ka, salt = empty, info = "ConfirmationKeys" ‖ AAD, L = 32)`; `cA = HMAC-SHA256(KcA, TT)`, `cB = HMAC-SHA256(KcB, TT)`. Confirmations are compared in constant time. Because the keys differ per role, a device's own confirmation sent back to it fails. `crates/portal-core/tests/spake2.rs` checks the implementation against all four RFC 9382 test vectors (`tests/fixtures/rfc9382-vectors.json`).

### 3.3 Messages

| message | from | fields |
|---|---|---|
| `pair_ask` | asker | |
| `pair_ready` | shower | (the code is on screen) |
| `pair_key` | both | `share`: 65-byte SPAKE2 share, hex |
| `pair_check` | both | `mac`: 32-byte confirmation, hex |
| `pair_wrong` | shower | `tries_left`: wrong codes still allowed (0: over) |
| `pair_done` | asker | (the shower's confirmation checked out) |
| `pair_stop` | either | (it's over; no reason given) |

### 3.4 Flow

```
asker                                   shower
  ── pair_ask ──────────────────────────▶   asks its person: "Desk PC (192.168.1.20) wants to pair"
                                            Show code → new code, w
  ◀────────────────────────── pair_ready ─   (Ignore: nothing is sent, ever)
  person types the code → w, x
  ── pair_key {pA or pB} ──────────────▶   new secret, its share, the keys
  ◀──────────────────── pair_key {share} ─
  keys
  ── pair_check {own confirmation} ────▶   checks it
                     wrong: ◀── pair_wrong {tries_left} ─   (same code stays on screen;
                                                              the asker types again from pair_key)
                     right: ◀── pair_check {own confirmation} ─
  checks it; stores the shower's fingerprint
  ── pair_done ─────────────────────────▶   stores the asker's fingerprint
```

- Each try uses fresh SPAKE2 secrets on both sides; the code (so w) stays the same within one pairing.
- Any message out of order on the pairing's connection ends the pairing with `pair_stop`, as does an invalid share or a confirmation that doesn't check out at the asker (that only happens with an attacker in the middle).
- The asker's own confirmation is sent before the shower's, so the shower is the one that counts wrong codes.
- The old numeric-comparison messages of version 2 (`pair_request`, `pair_commit`, `pair_nonce`, `pair_reveal`, `pair_confirm`) don't exist in version 3: they decode as unknown and are ignored. There's no fallback to them.

### 3.5 Limits (shower)

- **Asks:** at most 3 `pair_ask` per requesting key and per address within 10 minutes; more are ignored silently. After Ignore, or a cancel, the same key and address can't raise a prompt for a minute (silently too). A shower that's already pairing answers another device's ask with `{"t":"error","code":"rate_limited"}`.
- **Ignore gives the asker no hint:** nothing is sent; the asker gives up after the timeout, exactly as if nobody had seen the question.
- **Tries:** 3 wrong codes per pairing; the third ends it (`pair_wrong {tries_left: 0}`, then `pair_stop`).
- **Across everyone:** after 10 wrong codes in all (any requesters, any keys, any addresses), pairing is **locked**: further asks get `rate_limited` until the person on the shower presses "Allow pairing again". A per-device or per-address limit alone isn't enough: keys cost nothing and IPv6 addresses change.
- **Time:** a pairing that isn't finished 2 minutes after the ask (and again 2 minutes after the code appears) ends with `pair_stop`.

### 3.6 After pairing

Both devices store the other's id, name, OS and fingerprint, and connect to each other from then on with the fingerprint pinned (§1). Pairing again (after one side removed the other) runs the same exchange. Pairings made by version 2 (numeric comparison) stay valid: they're the same stored fingerprints.

## 4. Discovery

Sightings are hints only: who a device is gets proven by TLS afterwards (§1). Three ways, all at once:

1. **mDNS / DNS-SD:** service `_seamdrop._tcp.local.`, instance name = the device id, host name `seamdrop-<id>.local.` (never the machine's own host name). TXT keys: `id` (32 hex), `name`, `os` (`macos`, `windows`, `linux`, `other`), `proto` (the protocol version), `port` (TCP).
2. **UDP announce** on port 47920 (the default; any port works), sent to every interface's directed broadcast address (including 169.254.255.255 on link-local interfaces), 255.255.255.255, and IPv6 `ff02::1` per interface. The datagram is `PORTAL1\n` followed by JSON (at most 1200 bytes in all):
   ```json
   {"proto":4,"id":"<32 hex>","name":"Desk PC","os":"windows","port":47920,"uport":47920,"reply":false}
   ```
   `port` is the TCP port to dial, `uport` the UDP port to answer to. A device that hears an announce from a device it hasn't seen recently answers by unicast at once, with `"reply": true` (never answering a reply).
3. **Sweep:** the same announce sent by unicast to every address of the local network (a /24 at most), while a device is alone; some routers pass neither multicast nor broadcast between their wired and wireless sides, but unicast gets through. Announces go out in a burst (every 2 s for 30 s) after start, wake, a network change or Try again; sweeps at 2, 6, 12 and 20 s, then 30 s, 1, 2, 4 and 5 minutes.

A paired device's last-known addresses are dialled too (with its fingerprint pinned), and a person can type an address ("Connect by IP"). A device seen with another protocol version (in its TXT record, its announce or its hello) is shown as needing an update.

## 5. After the hello

- `{"t":"welcome","paired":true|false}`: sent once, saying whether this side trusts the other (by its pinned key). A device that receives `paired: false` from a device it trusts knows it was removed there: it stops trusting it too and asks its person to pair again.
- `{"t":"status","role":"both","name":"Desk PC","accepts":"ask"|"always"|"never"}`: the sender's name, and what it does with files from the device it's telling ("Files from …" on that side). `role` is always `both` (version 1 builds read it). Sent after the welcome and whenever either changes.
- **Liveness:** each side sends `{"t":"ping"}` every second, whatever else it's sending (answered with `{"t":"pong"}`). **Any bytes read count as alive** (a 1 MiB data frame takes over 3 s on a 300 KB/s link, so whole frames can't be the measure). A link that receives no bytes at all for 2.5 s is closed, so a transfer pauses within about 3 s of the other device really disappearing; time a device spends waiting on its own disk doesn't count, and nor does a stall of its own (a watchdog over 1 s late counts the quiet only from when it woke).
- `{"t":"superseded"}`: "keep your other link to me; this one is closing". When two links with the same key come up between two devices (both dialled, or a faster path), the device with the smaller id decides which stays: the better path (a direct cable, then wired, then Wi-Fi), the newer one on a tie, never one a transfer is running on.
- `{"t":"bye"}`: closing on purpose.
- `{"t":"error","code":…,"msg":…}`: `incompatible_version`, `not_paired` (pair first, or again), `key_changed` (a different key than the one pinned for that id), `rate_limited` (pairing: busy or locked, §3.5), `protocol` (malformed or out of order), `disconnected` (still paired, but the person pressed Disconnect there: don't redial until they Connect).

Everything below (transfers, the clipboard) is accepted only on a link whose key is trusted; anything else from an unpaired device gets `not_paired` (an offer gets a decline with `not_paired`).

## 6. Transfers

One link carries at most one transfer at a time (its data frames belong to the transfer that's streaming). A computer runs at most one active transfer (offered, running, paused) per device, in either direction, and any number across devices.

1. **Offer** (sender → receiver):
   ```json
   {"t":"offer","tid":12345,"items":[{"name":"Photos","kind":"folder","bytes":640000000,"files":212}],
    "total_bytes":640000000,"file_count":212,"dir_count":4,
    "skipped":[{"path":["Photos","link"],"reason":"symlink"}],"sender_role":"both"}
   ```
   `tid` is a random 64-bit number chosen by the sender. `skipped` lists what the sender left out (`symlink`, `junk` such as `.DS_Store`, `unreadable`, `special`).
2. The receiver answers `{"t":"accept","tid":…}` or `{"t":"decline","tid":…,"reason":{"why":…}}` with `why` = `user`, `role_send` (its "Files from …" is Never; the old name stays for version 1 builds), `busy` (another transfer with this device is running), `no_space` (`needed`, `available`: the offer plus a 64 MiB margin, counting every other incoming transfer's remaining bytes), `not_paired`, or anything else (`other`).
3. **Folders first:** `{"t":"dirs","tid":…,"entries":[{"path":["Photos","2026"],"mtime":{"s":…,"ns":…}}]}` (up to 500 per message), so empty folders arrive and folder times can be set at the end.
4. **Each file:** `{"t":"file","tid":…,"idx":0,"path":["Photos","é.jpg"],"size":7,"mtime":{"s":…,"ns":…},"from":0}`, then its bytes as data frames (kind 2, at most 1 MiB each, exactly `size − from` bytes in all), then `{"t":"file_end","tid":…,"idx":0,"blake3":"<64 hex>"}`, the BLAKE3 of the whole file. `path` is a list of names, never a joined string. The receiver refuses any component that is empty, `.` or `..`, or contains a separator or NUL, and makes names safe for its own system (Windows' forbidden characters and reserved names, NFC, length), numbering a name that already exists ("Report (2).pdf").
5. `{"t":"done","tid":…}`: everything was sent. The receiver checks every file's BLAKE3 as it arrives, moves the whole transfer from its hidden staging folder into place, sets the folders' times, and answers `{"t":"complete","tid":…,"summary":{"files":…,"bytes":…,"renamed":…,"skipped":…,"long_paths":…,"saved_in":"C:\\Users\\Sam\\Downloads\\SeamDrop","renames":[{"from":["Docs","a:b.txt"],"to":["Docs","a_b.txt"],"reason":"‘:’ isn’t allowed in Windows names"}]}}`. `renames` lists the first 50 names the receiver changed, so the sender can say which (absent when there are none; older senders ignore it). A file whose hash doesn't match fails the transfer; nothing partial is left in the receive folder.
6. **Progress:** the receiver sends `{"t":"progress","tid":…,"bytes":…}` (bytes received so far) every 250 ms, at each file end and at the end. Senders show these confirmed bytes, not what they've queued. While a sender re-reads a partial file before resuming (§7), it sends `progress` too, so neither side takes the quiet for a stall.
7. **Cancel:** `{"t":"cancel","tid":…,"reason":{"why":"user"}}` or `{"why":"error","code":…,"msg":…}`, from either side, at any point. The receiver removes the partial files.

8. **What goes next to a busy device:** `{"t":"queue_head","at":<unix ms>|null}` tells the other device when the earliest of what this one has waiting for it was queued (null: nothing). When the transfer between them ends, the one queued earliest offers first (the lower device id on a tie); the other waits for the next free slot. A device sends its new head only after its own offer is on the wire. An offer that still crosses and is declined as `busy` goes back to the front of its queue.

## 7. Resume

When a link drops mid-transfer, both sides keep the transfer for 24 hours (the receiver keeps what arrived in its staging folder, with a checkpoint of whole files and the bytes of the current one) and wait for the device to come back.

- When a trusted link comes back, the **sender** asks `{"t":"resume_ask","tid":…}` (again until answered).
- The **receiver** checks what it has (re-reading the current file's bytes) and answers `{"t":"resume_from","tid":…,"dirs":…,"file":…,"offset":…,"bytes":…,"prefix":"<64 hex>"}`: folder entries already received, whole files done, bytes of the next file it already has, all bytes so far, and the BLAKE3 of those `offset` bytes.
- The **sender** compares `prefix` with the same bytes of its file: if they match, it continues from file `file` at `offset` (`"from": offset` in that file's `file` message); if not (the file changed while paused), it sends that file again from the start and says so on both done screens. Nothing already received is sent again.
- **Stopped on purpose (version 4):** a device that was quit or crashed mid-transfer asks its person before continuing. Until they press Continue it sends `{"t":"hold","tid":…,"on":true}` instead of resuming (a sender instead of `resume_ask`; a receiver in answer to one), and a receiver adds `{"t":"kept","tid":…,"bytes":…}` (what it has checked and on disk, so a sender after a crash shows a percentage it knows). The other side shows "Paused: waiting for <device> to continue", stops asking, and keeps its own Cancel. Continue sends `{"t":"hold","tid":…,"on":false}` and the resume goes ahead as above. A held sender answers a `resume_from` with `hold`; a receiver that had already started again stops quietly (no `cancel`). A device that gets `hold` for a transfer that ended there answers with how it ended (below). Neither message is transfer traffic: both reach the device even while another transfer runs on the link. A dropped network or a restart to update never holds.
- A `tid` that isn't under way any more is answered from how it ended: `complete` (with its summary) if it finished, `cancel` with `user` if it was cancelled, `cancel` with `{"why":"error","code":"expired"}` after 24 hours, `{"code":"unknown"}` otherwise. The transfer ends on the asking side too.

## 8. Clipboard

Clipboard contents travel on the control channel (no data frames), only between paired devices.

- **Contents** are encoded as consecutive parts, each `tag (1 byte) ‖ length (u64 big-endian) ‖ bytes`: `T` plain text (UTF-8), `R` RTF, `H` an HTML fragment (UTF-8), `P` a PNG image. Any of them may be missing. Limits: text, RTF and HTML together up to 8 MiB, images up to 25 MiB; anything bigger goes as an ordinary file transfer instead ("Clipboard text.txt", "Clipboard image.png").
- The encoded bytes are split into parts of 768 KiB, each sent as `{"t":"clip","id":…,"n":0,"of":3,"auto":false,"data":"<base64>"}` in order. `id` is chosen by the sender, unique per link. `auto` is true when automatic sync sent it (no notification on arrival). `"private":true` (absent otherwise) marks text sent as private on purpose from the Shelf's text box: the receiver puts it on its clipboard with the markers password managers use (macOS `org.nspasteboard.ConcealedType` and `TransientType`; Windows `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory` = 0 and `CanUploadToCloudClipboard` = 0), shows no preview of it, and so never sends it on. Private text over the clipboard limit is refused by the sender, never sent as a file.
- **Automatic sync is agreed per pair:** `{"t":"clip_sync","on":true|false,"at":<unix ms>}` says where this device stands with the one it's telling, sent when a trusted link comes up and whenever it changes. Copies flow automatically only when both have said `on`. `off` from either side turns it off for the pair on both (the newest `at` wins, and a device's own change is always stamped later than anything it has heard). A device that receives an automatic copy without having agreed drops it.
- **Never sent:** anything the copying app marked as private (macOS: the nspasteboard.org types such as `org.nspasteboard.ConcealedType`; Windows: `ExcludeClipboardContentFromMonitorProcessing`, `Clipboard Viewer Ignore`, or a DWORD 0 in `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard`). The content isn't read at all. Copied files go as a transfer, not as a clipboard.
- A device marks what it writes to its own clipboard (macOS type `com.seamdrop.desktop.received`, Windows format `SeamDropReceived`) and never sends that back automatically, so copies don't bounce between devices.

## 9. A relay later

Nothing here assumes a direct LAN connection beyond discovery (§4). The trust is the pinned key pair, and the TLS session between two devices is end to end. A relay ("SeamDrop Anywhere") would carry the same TLS byte stream between two devices that both connect out to it, without terminating TLS: it would see only encrypted bytes and which two devices are talking. The frames, the hello, pairing (§3: the exporter and both fingerprints are those of the end-to-end TLS session, so a relay can't sit in the middle), transfers, resume and the clipboard would be unchanged. What would change: how devices find each other (an account or a pairing-time rendezvous id instead of §4) and how they reach each other (a relay address instead of a LAN address). Not built.

## 10. Phones

`portal-core` checks for Android (`aarch64-linux-android`) and iOS (`aarch64-apple-ios`) in CI on `main`. What a phone app would change is in `docs/DECISIONS.md` (background limits, no global shortcut, the share sheet).

## 11. The Shelf (version 4)

Text a person keeps on purpose, shared with paired devices only when **both** agree: `{"t":"shelf_share","on":true|false,"at":<unix ms>}`, sent when a trusted link comes up and whenever it changes (the newest `at` wins). Records travel as `{"t":"shelf","items":[…]}`, each

```json
{"id":"<32 hex>","origin":"<device id>","name":"Vercel key","text":"…","private":true,"stamp":1763740000123,"by":"<device id>","added":1763740000123,"removed":false}
```

- `stamp` is `max(now, last + 1)` in ms (newer than anything the device has seen); the newest version of a record wins everywhere, the larger `by` on a tie. A removal is a version with `removed: true` and empty `name` and `text`; devices keep removal markers for 400 days (at most 4,000).
- **No relaying:** a device sends only records whose `origin` is itself or the device it's talking to (an edit of that device's item goes back to it). On a fresh link, all such records; then each change. A device refuses any record whose `origin` is neither the sender nor itself.
- **Checks:** id 32 lowercase hex, text up to 64 KiB, name up to 80 characters without line breaks, `stamp` and `added` at most a day ahead, removals without text. At most 200 live items: beyond that, records from other devices are refused.
- Unsharing (either side) or unpairing: each side forgets the other's records. Private items travel like the rest (inside TLS); a receiving device never shows their text in a preview or notification.

## 12. Device pictures (version 4)

`{"t":"picture","picture":null|{"kind":"photo","png":"<base64>"}|{"kind":"mark","color":<0xRRGGBB>,"icon":"laptop"|"desktop"|"phone"|"tablet"}}`, sent to trusted links when they come up and whenever it changes. A receiver checks a photo before using it: PNG signature, the IHDR dimensions (at most 256 × 256) and size (at most 64 KiB) before decoding, then one full decode; anything else falls back to the default (a colour from the device id and the system's icon).
