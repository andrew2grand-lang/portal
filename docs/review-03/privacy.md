# Privacy review: SeamDrop

Persona: a privacy-minded home user who copies passwords and 2FA codes all day. Sources: the top part of the README and the screens at `http://localhost:5180`, running against a fake core in Mac and Windows windows at 552×780. Screenshots are in `shots/privacy-*.png`.

## Findings

### 1. "Never sent" promises more than the app checks: **confuses**
- **Screen:** `?mock=online&clip=private&os=mac`, then Send clipboard. Also Settings → Clipboard.
- **Shots:** `privacy-d6-private-send.png`, `privacy-d1-settings-clipboard.png`
- **What happened:** The README and Settings both say "What password managers copy is never sent." When I sent the clipboard, the actual rule appeared: "Not sent: the app you copied from marked it private, as password managers do." So it depends on the source app marking the item.
- **Why it matters:** I also copy 2FA codes from an authenticator, passwords I reveal in a browser and recovery codes from a text file. If those apps don't mark them, they go across. The blanket promise made me think I was covered.
- **Fix:** State the real rule everywhere: "Anything an app marks as private is never sent. Everything else you copy can be." List the password managers you've tested, and consider a "never send from these apps" list.

### 2. Either computer turns on automatic sync for both, and nothing records what left: **blocks use** (of automatic sync)
- **Screen:** `?mock=online&clip=sync` (Mac and Windows), `?mock=offline&clip=sync&os=mac`, Settings → Clipboard.
- **Shots:** `privacy-d9-sync-main-mac.png`, `privacy-d13-sync-main-win.png`, `privacy-d14-sync-offline-mac.png`, `privacy-d11-sync-toggle-settings.png`
- **What happened:** The setting says "One switch for both computers." Ticking it takes effect immediately: no confirmation, no explanation of what it covers, and no line in Details. The only indicator is a pill inside the window, and the window is usually closed because the app runs from the menu bar or tray. The pill still says "on" while the PC is unreachable. The clipboard panel lists only what arrived. Nothing shows what my Mac sent.
- **Why it matters:** Whoever sits at the PC can start receiving everything I copy on my Mac, and afterwards I can't check whether a password went across.
- **Fix:** Make each computer opt in ("Desk PC wants to sync clipboards: Allow?"). Show the sync state on the menu bar and tray icon. Add a "Sent" list (type, size and time, never the content) and log on/off changes in Details. Say what happens to copies made while disconnected.

### 3. Received clipboard is shown in plain text "until SeamDrop quits", which in practice is never: **confuses**
- **Screen:** `?mock=online&clip=history&os=mac`, then the "4" key next to Send clipboard.
- **Shots:** `privacy-d2-history-main.png`, `privacy-d3-history-open.png`, `privacy-d4-history-scrolled.png`
- **What happened:** The key is a clipboard icon and "4", with no label or tooltip. It opens text previews ("Meeting notes, Thursday: budg…") that are "kept until SeamDrop quits". Closing the window doesn't quit SeamDrop, so these stay for days. A 22.4 MB image says "No longer kept" without saying why, yet still counts in the 4. The line "What password managers copy is never sent" is half hidden under the buttons, even scrolled to the bottom.
- **Why it matters:** Anyone who opens the window can read what I copied on the other computer. I can't tell whether this is kept only in memory.
- **Fix:** Say "Kept in memory only, never written to disk." Add a retention choice: off, 1 hour, or until quit. Blur previews until hovered. Label the key "4 received" and explain "No longer kept". Stop the buttons covering the text.

### 4. Pairing doesn't say what the other computer can do afterwards: **confuses**
- **Screen:** `?mock=nearby&os=mac`, then Pair → Codes match. Also `?mock=pairing-incoming&os=win`.
- **Shots:** `privacy-a2-pair-code-mac.png`, `privacy-a3-paired-mac.png`, `privacy-a5-pairing-incoming-win.png`, `privacy-a4-details-after-pair.png`
- **What happened:** The 6-digit code check is clear, but no screen says what pairing allows. Files ask every time, but clipboard items land without asking, and the PC can turn on sync (finding 2). An incoming request shows only the name the sender chose ("MacBook Pro"), with its address only in the Details strip. Nothing says "only confirm if you started this". The pairing isn't logged, and Paired devices shows no key or pairing date.
- **Why it matters:** On a shared network, a stranger can name their laptop "MacBook Pro". One careless "Codes match" lets them reach my clipboard.
- **Fix:** Show the sender's address with "Only confirm if you started pairing on that computer yourself." After pairing, state in one line what the other computer can now do. Log the pairing with a key fingerprint, and show the fingerprint and pairing date under Paired devices.

### 5. Updates are on by default, the server isn't named, and restarts are unclear: **confuses**
- **Screen:** `?mock=online&update=available|ready|waiting|failed|offline`, Settings → About.
- **Shots:** `privacy-c-waiting-main.png`, `privacy-c-offline-settings-win.png`, `privacy-c1-auto-update-off.png`, `privacy-c-failed-settings-win.png`
- **What happened:** "Install updates automatically" is ticked by default. The app contacts "the update server" without naming it (the README points to GitHub, About to seamdrop.app) and "tries again later" without saying how often. "Restarts to update when the transfer finishes" offers only "Not now", so it seems to restart by itself. Unticking it doesn't say whether checks or downloads continue. Checks never appear in Details.
- **Why it matters:** This is the only internet traffic the app admits to. On by default is acceptable to me, given the signature check, but only if it says what it sends and never restarts without asking.
- **Fix:** Under the checkbox, add: "Checks <host> once a day; sends only the version and OS. Never restarts without asking." Split it into "Check for updates" and "Install automatically". Log each check.

### 6. Partial files: how long is stated, but not where, or what Cancel does: **confuses**
- **Screen:** `?mock=paused-in&os=mac`, then Cancel. Also `?mock=paused-out&os=mac`.
- **Shots:** `privacy-b-paused-in.png`, `privacy-b1-paused-in-cancel.png`, `privacy-b2-cancel-details.png`, `privacy-b-paused-out.png`
- **What happened:** The screen says "What has arrived is kept for 24 hours … After that it's removed", but not where it's kept. Cancel jumps straight to "Desk PC isn't reachable", with no word that the 280 MB received was deleted and nothing in Details. The sender's side adds "while SeamDrop is open on both computers", so it's unclear whether data is removed if SeamDrop is closed when the 24 hours run out.
- **Why it matters:** These are work documents. Half a file in an unknown place is still my data on disk.
- **Fix:** Say where it is: "280 MB in a hidden folder in Downloads/SeamDrop. Cancel deletes it." Confirm after Cancel, delete expired data on the next start, and log it.

### 7. The firewall is open on public networks, and nothing limits SeamDrop to home: **confuses**
- **Screen:** README Install, Windows step 3. Settings at `?mock=online-ethernet&os=win`.
- **Shot:** `privacy-s2-settings-win.png`
- **What happened:** The Windows installer allows incoming connections on public networks too. The README explains why, which is fair. But there's no "only on my home network" setting, and SeamDrop keeps running when the window is closed. I can't tell whether my MacBook announces itself and accepts pairing requests on café Wi-Fi.
- **Fix:** Add "Receive only on these networks" (remember the home network at pairing) and "Pause SeamDrop" in the menu bar and tray.

### 8. There's no way to verify the first download: **confuses**
- **Screen:** README Install.
- **What happened:** The builds are unsigned. On the Mac I have to clear the quarantine flag with `xattr`, and on Windows I have to click past SmartScreen. No checksums are published, and dev builds from any commit are offered too. The update signature check helps only after install.
- **Fix:** Publish SHA-256 checksums and the update key's fingerprint with each release.

### 9. Portal's keys stay on disk: **polish**
- **Screen:** README, Coming from Portal.
- **What happened:** Portal's data folder, which held the pairing keys, is left behind for me to delete by hand.
- **Fix:** Offer "Delete Portal's old data" once the migration has worked.

### 10. "Clipboard sent" doesn't say what was sent: **polish**
- **Screen:** `?mock=online&os=mac`, then Send clipboard.
- **Shot:** `privacy-d8-send-clipboard-details.png`
- **What happened:** It says "Clipboard sent to Desk PC." and nothing more, and Details stays empty.
- **Fix:** "Sent text, 24 characters" or "Sent an image", without the content, and the same line in Details.

## What the screens say, and what I can't verify

**The screens say:** everything under "What reassured me", plus: received clipboard is kept until SeamDrop quits, and partial transfers for 24 hours.

**I can't verify:** whether connections are encrypted. What the real Details log records, and whether it's written to disk: the fake core logs only "start" and "found". What notifications show (file names, or clipboard text on the lock screen). Whether clipboard history or partial files touch disk. What the update check sends.

## Questions before I'd trust it

1. Is every connection encrypted with the key from pairing? What can an unpaired device on the network do?
2. How is "private" detected on each OS? Which password managers and authenticator apps have you tested?
3. Is received text marked so that Windows clipboard history and cloud sync, and Apple's Universal Clipboard, don't pass it on to Microsoft or iCloud?
4. Do logs or notifications ever contain clipboard text or file names? Where is the log kept, and for how long?
5. Am I told when the other computer turns sync on?
6. Is anything copied while disconnected queued and sent later?
7. Which update host, how often, and what's sent? Will the "Paid" Project mode need an account or licence check?
8. What does SeamDrop announce on the network: the computer name, my user name?

## What reassured me

- There's no account and no cloud. The Mac install needs no administrator, and the README explains each permission.
- Pairing uses a code on both screens. Remove takes effect at once, and the other side sees "removed this Mac".
- Files ask every time by default, with "Never" available. Sync and Open at login are off.
- When I copied from a password manager, the app refused to send it and said why.
- The 24-hour limit for partial transfers is stated, and a failed transfer says "Nothing was saved there."
- Updates that fail the signature check aren't installed, and the app says so.
- "Disconnect stops it; you stay paired" is a per-device off switch.
