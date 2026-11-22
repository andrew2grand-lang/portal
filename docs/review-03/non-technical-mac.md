# Review 03: non-technical Mac user

Persona: about 60, MacBook for email, photos and documents, Windows PC at home that a relative set up. Never opened Terminal. Reads every word literally. Goal: send a photo folder and a few documents to the PC; copy an address on the Mac and paste it on the PC.

Screens: `http://localhost:5180/?mock=<state>&os=mac` at 552×780. Screenshots: `shots/mac-user-*.png`.

## How the tasks went

| Task | Finished? | Steps | Stuck on |
|---|---|---|---|
| A. Install + pair | **No, not alone.** Stopped at README step 3 (Terminal). With help, pairing was 3 clicks plus a walk to the PC | 6 README steps, then Pair → Codes match (both sides) | Terminal, two Applications folders, "Project Paid" |
| B. Transfer drops | Yes, but anxious | Choose a folder → Add files → Send → Accept on PC | Were my photos safe after Cancel or "Transfer stopped"? |
| C. Update | Yes | 1 click | Restart *what*? "Signature check" scared me |
| D. Clipboard | Half | Copy → Send clipboard → ? | Nothing told me how to paste on the PC |

## Findings

### 1. The Mac install needs Terminal
- **Screen:** README → Install → macOS, step 3 · **Screenshot:** n/a
- **What happened:** "Open Terminal and run `xattr -dr com.apple.quarantine…`". I've never opened Terminal, and "quarantine" sounds like a virus. I'd stop and phone my nephew.
- **Why it matters:** Without this step there's no app.
- **Fix:** For home Macs (I'm the administrator), lead with the familiar route: drag to Applications, then System Settings → Privacy & Security → **Open Anyway** with *your own Mac password*, with pictures. Move the Terminal route under "work or school Mac". Best fix: sign the app.
- **Severity:** **blocks use**

### 2. The README talks to developers and managed Macs
- **Screen:** README intro and Install · **Screenshot:** n/a
- **What happened:** The opening lines mention `src-tauri/tauri.conf.json`, "Mission 03", "Actions → Artifacts", and "aren't signed yet". Step 2 says to make a *second* Applications folder and not to use the usual one, which is the opposite of every install I've done. It calls Send clipboard "the key under the drop area"; I looked at my keyboard.
- **Fix:** Put a short "For everyone" install first, with one download link per computer. Move developer notes to Develop. Say "button".
- **Severity:** **confuses**

### 3. "Project · Paid" tab on every screen
- **Screen:** all, e.g. `?mock=nearby&os=mac` · **Screenshot:** `mac-user-A1-nearby.png`, `mac-user-A-project-tab.png`
- **What happened:** My photos are a "project", so I clicked it. It says a "code project… arrives ready to run", "coming soon". "Paid" made me worry I'd be charged.
- **Fix:** Hide it until the feature ships, or move it to Settings.
- **Severity:** **confuses**

### 4. Technical text along the bottom of every screen
- **Screen:** all, the Details strip · **Screenshot:** `mac-user-B-details-log.png`
- **What happened:** `found Desk PC 192.168.1.31:47920 via mdns` sits under every screen. I wondered if it was an error.
- **Fix:** Show only "Details" until someone opens it.
- **Severity:** **annoys**

### 5. "Still looking" leads with "blocking", Privacy settings and IP addresses
- **Screen:** `?mock=searching-long`, `?mock=offline` · **Screenshot:** `mac-user-A-searching-long.png`, `mac-user-A-offline.png`, `mac-user-A-connect-by-ip.png`
- **What happened:** The first cause is "macOS may be **blocking** SeamDrop", with a route through Privacy & Security and an IP address. The offline screen offers **Disconnect**, but it already says it isn't connected. The list slides under the buttons. In Connect by IP, the grey `192.168.1.20` looks already filled in, so I didn't see why Connect was greyed out.
- **Fix:** Lead with the simple checks (Is SeamDrop open on the PC? Are both on home Wi-Fi?). Rename Disconnect to "Stop trying". Leave the IP field empty, or label the placeholder "e.g.".
- **Severity:** **confuses**

### 6. Stalled transfer says "Connected" and "interrupted" at once
- **Screen:** `?mock=stalled` · **Screenshot:** `mac-user-B-stalled.png`
- **What happened:** The header says "Windows · Connected" with a Wi-Fi badge. The card says "Connection interrupted… 0 MB/s".
- **Fix:** Make the header say "Reconnecting…" too.
- **Severity:** **confuses**

### 7. Cancel throws away a half-sent transfer without asking
- **Screen:** `?mock=paused-out` (also `sending`) → Cancel · **Screenshot:** `mac-user-B-paused-out.png` → `mac-user-B-paused-cancel.png`
- **What happened:** Cancel is the only button on the paused screen, so I pressed it to close the message. It ended the transfer with no question. I landed on "Desk PC isn't reachable", with no word on my photos.
- **Why it matters:** This is the "are my photos lost?" moment.
- **Fix:** Confirm first: "Stop sending? Your files stay on this Mac; what arrived on Desk PC is removed." Add a harmless "Hide" or "Keep waiting" button.
- **Severity:** **confuses**

### 8. "Transfer stopped… Nothing was saved there"
- **Screen:** `?mock=failed` · **Screenshot:** `mac-user-B-failed.png`
- **What happened:** I read "Nothing was saved" and panicked before I noticed "there". The only button is OK, so I'd have to pick the folder and documents again. Why didn't this one pause and resume like the other?
- **Fix:** "Your files are still on this Mac. Nothing arrived on Desk PC." Add **Send again** with the same list.
- **Severity:** **confuses**

### 9. "After that it's removed"
- **Screen:** `?mock=resuming`, `paused-out`; `paused-in&os=win` · **Screenshot:** `mac-user-B-resuming.png`, `mac-user-B-win-paused-in.png`
- **What happened:** "Kept for 24 hours… After that it's removed." Removed from my Mac? The two computers word the rule differently ("cancelled" and "removed"). Under "Desk PC is back", the meter still says "Paused".
- **Fix:** "Your files are safe on this Mac. If it can't finish in 24 hours, the half-copied files on Desk PC are cleaned up." Use the same sentence on both computers.
- **Severity:** **confuses**

### 10. "Sent" doesn't say where the files went, or what was left out
- **Screen:** `?mock=done-out&os=mac` vs `done-in&os=win` · **Screenshot:** `mac-user-B-done-out.png`, `mac-user-B-win-done-in.png`
- **What happened:** The PC says "saved to Downloads\SeamDrop" and "2 items left out". My Mac says neither, so I can't tell my nephew where to look. Which name was changed? "85 MB/s" means nothing to me.
- **Fix:** On the Mac, show "On Desk PC: Downloads › SeamDrop". List what was skipped or renamed. Move the speed to Details.
- **Severity:** **confuses**

### 11. "Restart to update" (restart what?)
- **Screen:** `?mock=online&update=ready`, Settings → About · **Screenshot:** `mac-user-C-online-update-ready.png`, `mac-user-C-settings-about.png`, `mac-user-C-after-restart-click.png`
- **What happened:** I thought the whole Mac would restart and close my email. Only after clicking did it say "SeamDrop restarts in a moment." "What's new" is cut off. One banner says "Not now", the others say "Later". Does the PC need updating too?
- **Fix:** Label the button **Restart SeamDrop**. Add "Takes a few seconds. The PC updates itself too."
- **Severity:** **annoys**

### 12. Update failed: "didn't pass its signature check"
- **Screen:** `?mock=online&update=failed` · **Screenshot:** `mac-user-C-online-update-failed.png`
- **What happened:** "Signature check" sounds like a hacker. The only button is **Later**. Later what? Do I have to do something?
- **Fix:** "The update didn't download correctly, so it was skipped. Nothing on your Mac changed; SeamDrop will try again." Change the button to **OK**.
- **Severity:** **confuses**

### 13. "Update SeamDrop on this Mac", but there's no way to do it
- **Screen:** `?mock=incompatible-self` · **Screenshot:** `mac-user-C-incompatible-self.png`
- **What happened:** The only button is Try again. "Update this Mac" sounds like a macOS update. A manual update would mean the Terminal install again.
- **Fix:** Add an **Update SeamDrop** button that runs the built-in updater.
- **Severity:** **confuses** (blocks use if auto-update didn't run)

### 14. "Copy here, paste there" is hidden behind an unlabeled "4"
- **Screen:** `?mock=online&clip=history` → the "4" key; Send clipboard · **Screenshot:** `mac-user-D-clip-history.png`, `mac-user-D-clip-history-open.png`, `mac-user-D-clip-taken-sent.png`
- **What happened:** "Send clipboard" doesn't say to copy first. The icon with "4" looked like a notification count, with no label or tooltip. The one clear sentence ("Copy here, paste on Desk PC…") is inside that panel. After sending: "Clipboard sent to Desk PC." Now what? Nothing told me it's **Ctrl+V** on the PC. A **Clear** button sits next to Send clipboard, and "rich text, 860 bytes" is jargon.
- **Fix:** Under the button: "Copy something (⌘C), press here, then paste on Desk PC with Ctrl+V." Label the key "Received (4)". Rename Clear to "Clear list".
- **Severity:** **confuses**

### 15. Shortcut symbols and sync wording
- **Screen:** Settings → Clipboard · **Screenshot:** `mac-user-D-settings-clipboard.png`
- **What happened:** I know ⌘, but not ⌃ or ⌥, and four keys at once is hard. "Copies go to the other computer **and back**" made me think the PC might overwrite my copy.
- **Fix:** Write "Control + Option + Command + C". Say "Anything you copy on either computer is ready to paste on the other."
- **Severity:** **annoys**

### 16. Settings ✕ covers "Change…"
- **Screen:** Settings, scrolled · **Screenshot:** `mac-user-D-settings-clipboard.png`
- **What happened:** The floating ✕ overlaps the shortcut's Change… button. Screen readers announce it as "Clear".
- **Fix:** Pin the ✕ in a header row and label it "Close".
- **Severity:** **polish**

## What worked well
- Pairing: "Desk PC is nearby / Pair once and they'll know each other", a big **482 913**, then **Codes match**. No account, no password.
- The send list ("Ready to send", Add files / Add a folder, Send) and the PC's Accept / Decline / "Always accept" are clear.
- "Paused… Will continue when it's back… also after a restart or sleep" kept me calm.
- **Declined:** "Nothing was sent. Everything is still on this Mac." That's the sentence I needed on the Cancel and Failed screens.
- Update during a transfer: "waits for the transfer… Nothing is lost."
- The PC's Received screen says where the files went and has **Show in File Explorer**.
- The private-clipboard message gives a plain, friendly reason.
