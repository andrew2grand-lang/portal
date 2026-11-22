# Review 03: developer moving big files daily

Persona: MacBook Pro on Wi-Fi plus a wired Windows workstation, moving 5–60 GB artefacts, VM images and huge folders daily, plus stack traces and screenshots via the clipboard. Today I use rsync, a NAS or LocalSend. Screens are at `http://localhost:5180/?…`, and screenshots are in `shots/`.

Bottom line: setting it up and pairing is quick. For a big transfer that drops, I can't yet trust it the way I trust `rsync --partial` plus a checksum.

## Findings

**1. A dropped transfer can still mean "start over", and I can't tell when.** *confuses*
Screen `mock=failed&os=mac` vs `mock=paused-out&os=mac` · `dev-b-failed-mac.png`, `dev-b-paused-out-mac.png`
What happened: one screen says "The connection to Desk PC dropped at 19%. Nothing was saved there. Send it again", and offers only **OK**, while the header already says Connected. Another drop leads to "Paused … picks up where it stopped".
Why it matters: losing 11 GB of a 60 GB image is what I'm switching tools to avoid, and I can't predict which outcome I'll get. I also have to pick all the files again.
Fix: say why this one couldn't resume (for example "files changed on this Mac" or "24 h passed"), keep partial data whenever possible, and add **Send again** that queues the same items again.

**2. I can't confirm it resumed rather than restarted.** *confuses*
Screen `mock=resuming&os=mac`, `mock=done-out&os=mac` · `dev-b-resuming-mac.png`, `dev-b-done-out-mac.png`
What happened: "Checking what already arrived" shows no progress, and the status still reads "Paused". The done summary never mentions the drop.
Why it matters: checking 30 GB that already arrived takes minutes. I need to see that it's hashing and not stuck, and that only the missing part went over.
Fix: show "Checking 280 MB already on Desk PC… 60%", then "Resumed at 44%: 280 MB kept, 353 MB sent" in progress and in the done line.

**3. Verification is invisible.** *confuses*
Screen `mock=sending&os=mac` (let it reach 100%), `mock=done-in&os=mac` · `dev-b1b-sending-finished-mac.png`, `dev-b-done-in-mac.png`
What happened: at 100% it says "Finishing" and still shows "64 MB/s over Wi-Fi". No screen says the files were checked.
Why it matters: without "verified" I'll keep running `shasum` on both sides by hand.
Fix: show "Verifying… 12 of 40 GB" while it finishes and "All files verified" in the done line, and drop the speed once it reaches 100%.

**4. Details says nothing about transfers.** *confuses*
Screen any transfer state, then **Details** · `dev-b2-sending-details-mac.png`, `dev-b3-done-in-details-mac.png`, `dev-b5-done-out-details-mac.png`
What happened: in every transfer state the log has only `start … port 47920` and `found Desk PC … via mdns`. It has no bytes, speed, path, verification, renames, skips or reason for the drop.
Why it matters: this is the first place I'd look after a failure, and it's empty.
Fix: log each transfer, with items and bytes, path, drops and resumes with the offset, verification, renames (old → new), skips and the reason it failed. Add **Copy log**.

**5. Renamed and skipped items are counted but never named.** *confuses*
Screen `mock=done-in&os=mac`, `mock=done-out&os=mac` · `dev-b-done-in-mac.png`
What happened: "1 name changed so it works on macOS." and "2 items left out on Desk PC: links and system files aren't sent." Neither one is clickable, and the log doesn't list them.
Why it matters: a tree with symlinks (`node_modules/.bin`, venvs) or names Windows doesn't allow arrives subtly broken, and I can't tell which files.
Fix: add a list I can expand ("`Résumé: Q3?.pdf` → `Résumé_ Q3_.pdf`", "link `bin/python` skipped"). Warn before sending, not only after. Offer to follow links.

**6. No Pause button and no "Retry now".** *annoys*
Screen `mock=sending&os=mac`, `mock=paused-out&os=mac` · `dev-b1-sending-mac.png`, `dev-b-paused-out-mac.png`
What happened: the only control is **Cancel**. Pausing only happens when the connection drops, and the paused screen has no way to retry now.
Why it matters: I want to pause cleanly before I close the lid or join a call, and to get it going again as soon as Wi-Fi is back.
Fix: add **Pause**/**Resume** (with a key) and **Retry now** in the paused state.

**7. The resume rules are vague, and the update note contradicts them.** *confuses*
Screen `mock=paused-out&os=mac`, `mock=online&os=mac&update=ready`, Settings · `dev-b-paused-out-mac.png`, `dev-c-update-ready-mac.png`, `dev-d0-settings-top-mac.png`
What happened: the paused screen says it continues "also after a restart or sleep, while SeamDrop is open on both computers. After 24 hours it's cancelled." The 0.4.0 note says "Paused transfers now continue after the computer sleeps, too", which suggests 0.3.0 doesn't. **Open at login** is off by default, so nothing resumes after a reboot until I open the app. No deadline time is shown.
Why it matters: closing the lid is my everyday case.
Fix: make it one true statement, show "Cancelled at 14:05 tomorrow", and warn "Open at login is off" with a one-click fix. Say whether it resumes in the middle of a file.

**8. The pair can split after an automatic update, and the screen that says so can't fix it.** *confuses*
Screen `mock=incompatible-self&os=mac&update=ready` · `dev-c-incompatible-self-mac.png`, `dev-d1-settings-clipboard-mac.png`
What happened: **Install updates automatically** is on by default. When one side is newer: "can't talk to this one. Update this Mac". The only button is **Try again**, even with the "0.4.0 is ready" banner on screen.
Why it matters: if the PC updates overnight, my first transfer in the morning is blocked.
Fix: add **Update now** on that screen, keep compatibility with the previous version, and when one side updates, tell the other.

**9. The automatic restart after a transfer wipes clipboard history.** *annoys*
Screen `mock=sending&os=mac&update=waiting`, `clip=history` · `dev-c-update-waiting-sending-mac.png`, `dev-d4-clip-history-open-mac.png`
What happened: the banner says "Restarts to update when the transfer finishes … Nothing is lost." Clipboard history is "kept until SeamDrop quits". The banner also pushes the file list out of view.
Why it matters: the transfer finishes, I go to paste the stack trace from history, and it's gone.
Fix: restart only after a few idle minutes and while the window isn't in front, or keep history across the update restart.

**10. A long transfer locks the window.** *annoys*
Screen `mock=sending&os=mac` · `dev-b1-sending-mac.png`
What happened: while sending, there's no **Send clipboard** button and no way to add the next batch.
Why it matters: 60 GB at about 60 MB/s takes roughly 17 minutes.
Fix: keep **Send clipboard** in view and let me queue the next batch.

**11. The accept prompt shows no destination or free space and doesn't respond to keys.** *annoys*
Screen `mock=offer-in&os=mac&view=accept` · `dev-b-accept-view-mac.png`, `dev-b4-send-list-mac.png`
What happened: it shows "3 items, 633 MB." but not where they'll go or whether they fit. Return and Esc do nothing, and the send list ignores ⌘Return. No shortcuts are shown anywhere.
Why it matters: 60 GB arriving on a nearly full SSD, and every send takes extra mouse trips.
Fix: show "Saves to Downloads/SeamDrop · 212 GB free" with a warning when it won't fit. Return to accept, Esc to decline, ⌘Return to send.

**12. When the clipboard shortcut fails, nothing on the main screen says so.** *confuses*
Screen `mock=online&os=mac&clip=taken`, then Settings → Clipboard · `dev-d5-shortcut-taken-mac.png`, `dev-d6-shortcut-recording-mac.png`, `dev-d2-send-clipboard-hover-mac.png`
What happened: the main screen gives no sign. Settings shows plain text: "Another app uses that shortcut, or it can't be used." I recorded ⌃⌥⌘V, and the row went back to ⌃⌥⌘C with the same message, so I can't tell which one is dead. The button doesn't show its shortcut.
Fix: show a warning on the main screen, name the combination that was rejected, show it in an error state, and show the shortcut on the button.

**13. Clipboard limits aren't explained.** *confuses*
Screen `mock=online&os=mac&clip=history` (click the clipboard key) · `dev-d4-clip-history-open-mac.png`, `dev-d3-send-clipboard-clicked-mac.png`
What happened: "An image, 6000 × 4000 · 22.4 MB · No longer kept", with no reason given, and no size limits stated. "Clipboard sent to Desk PC." doesn't say what was sent. Previews cut off around 30 characters, and only received items are listed.
Why it matters: long logs and Retina screenshots are my everyday clipboard, and I need to know whether a 15 MB log arrived whole.
Fix: state the limits and send larger items as a file automatically. Say "Sent text, 14 KB, 312 lines". Let me expand a preview, and list sent items too.

**14. Polish**
- Receiving labels items "Sending" (`mock=receiving`, `dev-b-receiving-mac.png`). Fix: say "Receiving". *polish*
- The PC says "over Ethernet" even though the Mac's Wi-Fi sets the speed (`mock=online-ethernet&os=win`, `dev-a5-online-ethernet-win.png`). Fix: "Ethernet here, Wi-Fi on MacBook Pro". *polish*
- A stalled transfer still shows "Connected · Wi-Fi" next to "Connection interrupted", with no time waited (`mock=stalled`, `dev-b-stalled-mac.png`). Fix: "Reconnecting… 40 s". *polish*
- A failed update offers only **Later** (`update=failed`, `dev-c-update-failed-mac.png`). Fix: add **Try again** and a link to the releases page. *polish*
- Folder progress has no "140 of 212 files" count (`dev-b1-sending-mac.png`). *polish*
- The clipboard key is a bare "4", and Esc doesn't close its panel (`clip=history`, `dev-d-clip-history-mac.png`). Fix: "4 received". *polish*
- The empty screen lacks the **Add files** button the README promises (`dev-a4-paired-mac.png`). *polish*
- A "Project · Paid · coming soon" tab takes up main-window space (`dev-e1-project-tab-mac.png`). *polish*

## What worked

- **Install:** one `xattr` command on the Mac and SmartScreen plus an admin prompt on Windows are fine for me.
- **Pairing:** **Pair**, then a 6-digit code, then **Codes match**: three clicks (`dev-a3-pair-code-mac.png`). "Still looking" lists real causes and shows the Mac's IP for **Connect by IP** (`dev-a1-searching-long-mac.png`).
- **Progress:** shows MB of the total, MB/s, time left and the current file. A stall honestly shows 0 MB/s.
- **Done summary:** size, time, average speed, destination path and **Show in Finder**/**Show in File Explorer**.
- **Updates:** they wait for transfers, and both "Not now" and the signature check work.
- **Clipboard:** "Always accept" per device. Copies that a password manager marks private are refused with a clear message.

