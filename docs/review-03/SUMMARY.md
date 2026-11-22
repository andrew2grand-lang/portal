# Fresh-eyes review 03: summary

Four reviewers looked at SeamDrop with no mission files, plans or decision logs. Each was told only: "SeamDrop moves files and your clipboard between a Mac and a Windows PC." Each read the top of the README and used the real app's screens against the mock core (`http://localhost:5180/?mock=…&os=mac|win`, 552×780).

| Persona | Report |
|---|---|
| Beginner on a Windows gaming PC | [beginner-windows.md](beginner-windows.md) |
| Non-technical Mac user | [non-technical-mac.md](non-technical-mac.md) |
| Privacy-minded home user | [privacy.md](privacy.md) |
| Developer moving big files daily | [developer.md](developer.md) |

The reviewers' screenshots are in `shots/` here (kept out of git, like all regenerable screenshots). Screenshots after the fixes are in `docs/screens/m03-*.png`; `node scripts/real-shots.mjs <label>` makes the full set (every state, both windows, both themes) in `shots/real/`.

Severity, as the reviewers used it: **blocks use** (can't finish the task), **confuses** (finishes, but is misled or unsure), **annoys**, **polish**. Where reviewers disagreed, the higher rating is used. Mission 03 §8: everything ranked *blocks use* or *confuses* is fixed and re-checked below, except where noted; the rest is listed for the owner.

## Blocks use

| # | Finding | Who | Screen | Fix | Status |
|---|---|---|---|---|---|
| B1 | **The Mac install needs Terminal.** The README's only Mac route was `xattr` in Terminal; a home user stops there. | Mac, beginner | README → Install | README now leads with the route every Mac user knows (drag to Applications, then System Settings → Privacy & Security → **Open Anyway** with your own password) and keeps Terminal for work and school Macs, where Open Anyway needs an administrator. Signing removes the step for good (docs/SIGNING.md). | Fixed |
| B2 | **The Windows firewall fix meant deleting rules by hand.** A red ✗ said "delete: SeamDrop" in Advanced settings; a beginner won't touch firewall rules, and pressing Cancel on Windows' question is how most people get there. | Beginner | `searching-long&os=win` | **Fix it** in the connection check: one administrator question, then it removes inbound Block rules for SeamDrop and puts both Allow rules back (`portal-sys/src/firewall.rs`). The text says why it happened. Same button for a missing or partial rule. Says plainly if the question was answered No. Tested on real Windows Firewall rules in CI (`real_firewall_fix`). | Fixed |
| B3 | **Either computer could turn on automatic clipboard sync for both.** "One switch for both computers" meant whoever sat at the PC could start receiving everything copied on the Mac. | Privacy (blocks use), beginner (confuses) | Settings → Clipboard, `clip=sync` | Each computer now has to say yes. Turning it on asks the other ("MacBook Pro asks to sync the clipboard" with **Turn on** / **Not now**, also as a notification); nothing syncs until both agree. Turning it off on either stops it on both. Automatic copies are refused by a computer that hasn't agreed, even from its paired device. The pill says "Sync: waiting for Desk PC" until then; the tray says where it stands. | Fixed |

## Confuses

| # | Finding | Who | Fix | Status |
|---|---|---|---|---|
| C1 | **Cancel threw away a paused or running transfer at once**, and afterwards nothing said it was cancelled (the window jumped to "isn't reachable"). On a 12 GB transfer it's the only button, so it gets pressed by reflex. | All four | Cancel asks first and says what stays and what goes: "Stop sending? Your files stay on this Mac; the part that arrived on Desk PC is removed." with **Keep sending** / **Keep waiting** as the default. Afterwards a **Cancelled** screen says what happened on both sides. | Fixed |
| C2 | **"Transfer stopped… Nothing was saved there"** read as "my files are gone", and the only button was OK, so everything had to be picked again. | Mac, developer | "Your files are still on this Mac; nothing was saved on Desk PC." Stopped and cancelled sends get **Send again** (the same items, no picking). The developer's "failed vs paused, I can't predict which" came from the mock: a dropped connection now always pauses; the mock's failed state now shows a real failure (out of space). | Fixed |
| C3 | **Paused: "After that it's removed".** Removed from where, my Mac? The two computers worded the rule differently, nothing said where the partial data is, and nothing hinted at the likely cause. "Checking what already arrived" still said Paused. | Mac, beginner, privacy, developer | One rule, from each side's point of view. Sender: "Your files are safe on this Mac. If it can't finish within 24 hours, the part that arrived on Desk PC is removed." Receiver: "The 280 MB that arrived waits in a hidden folder in Downloads/SeamDrop for 24 hours… Cancel removes it now." The likely cause is named ("Is the Mac asleep, or SeamDrop closed there?"). While checking, the bar says **Checking…** and how much it's checking. | Fixed |
| C4 | **The PC never said how to get files from the Mac**, and "Choose a folder" looked like "where to save". | Beginner | Under the drop area: "Getting files from MacBook Pro? Send them from there. You're asked here first; they're saved in Downloads\SeamDrop." The key is now **Add a folder**. | Fixed |
| C5 | **The README talked to developers first** (`tauri.conf.json`, "Mission 03", Actions artifacts), told people to make a second Applications folder, and called a button "the key". | Mac, beginner | Rewritten: what it does, Install (per computer, in plain words, why Windows asks about Public networks), Using SeamDrop (send, receive, drops, clipboard, updates), then developers at the end. | Fixed |
| C6 | **The received-clipboard key was an icon and "4"** (read as a notification badge), it wasn't clear whether a received item was already on the clipboard, and nothing said how to paste. | All four | The key says **4 received**. The panel is "Clipboard from MacBook Pro": "The newest is already on your clipboard: paste it with ⌘V." An arrival also shows a line in the window: "Clipboard from MacBook Pro: '…' — ready to paste with Ctrl+V." After sending: "Sent to Desk PC (text, 1.2 KB). Paste it there with Ctrl+V." **Clear** is **Clear list**. | Fixed |
| C7 | **"What password managers copy is never sent" promised more than the app checks.** It depends on the app marking the copy. | Privacy | The real rule, everywhere (README, Settings, the panel): "Anything an app marks as private, as password managers do, is never sent. Everything else you copy can be." | Fixed |
| C8 | **History "kept until SeamDrop quits"** (which is never), not said to be memory only; "No longer kept" unexplained. | Privacy, developer | "Kept in memory only, never on disk, until SeamDrop quits or you clear the list." "Not kept (too big)" with the 64 MB memory limit in its tooltip. The limits are stated (text 8 MB, images 25 MB; bigger arrives as a file). | Fixed |
| C9 | **"Restart to update": restart what?** Read as rebooting the computer. | Mac, beginner | **Restart SeamDrop to update** (and "SeamDrop restarts to update when the transfer finishes"), in the window, Settings and the tray. | Fixed |
| C10 | **Update failed: "didn't pass its signature check"**, with only **Later**: sounded like a hacked download, with no next step. | Mac, beginner, developer | "The update didn't download correctly, so it was thrown away. Nothing on this computer changed; SeamDrop 0.3.0 keeps running and tries again later." Keys: **Try again** and **OK**. | Fixed |
| C11 | **"Update SeamDrop on this Mac" had no way to do it** (only Try again), even with an update ready. | Mac, developer | The screen offers **Restart SeamDrop to update** when one is ready, else **Check for updates**. | Fixed |
| C12 | **A stalled transfer said "Connected · Wi-Fi"** next to "Connection interrupted". | Mac, developer | The header says **Reconnecting…** and hides the link pill while nothing moves. | Fixed |
| C13 | **"Sent" didn't say where the files went** or that they were checked; resume and verification were invisible. | Mac, developer | Sender: "Saved in Downloads\SeamDrop on Desk PC; every file was checked there." (the receiver now reports its folder). Receiver: "Every file was checked on arrival." After a drop, both done screens say "Continued after the connection dropped: the 280 MB already on Desk PC wasn't sent again." | Fixed |
| C14 | **Pairing didn't say what the other computer can do afterwards**; an incoming request showed only the name the other computer chose. | Privacy | Nearby: "either one can offer files (you choose whether to accept) and send its clipboard." Incoming request: its address in the title, and "Only confirm if you started pairing on that computer yourself and it shows the same code." | Fixed |
| C15 | **Updates: which server, how often, does it restart by itself?** | Privacy | Settings → About: "SeamDrop checks the public SeamDrop releases page on GitHub when it opens and every 4 hours. The check sends nothing about you or your files, and an update is installed only if its signature is ours. … it restarts only when you press Restart." | Fixed |
| C16 | **"Still looking" led with "macOS may be blocking"**, Privacy settings and IP addresses; the offline screen offered **Disconnect** when it wasn't connected; the IP field's grey example looked filled in and sat above this computer's own address. | Mac, beginner | The screen starts with the simple checks ("First check that SeamDrop is open on your PC and both are on the same home network"); the Mac check reads "SeamDrop may need Local Network access on this Mac". **Stop trying** replaces Disconnect there. The field says "for example 192.168.1.20", is labelled "The other computer's IP address", and refuses this computer's own address with a clear message. | Fixed |
| C17 | **Details showed nothing about transfers.** | Developer | Mock only: the real core logs every step (offer, accept, pause with bytes, resume with the file and offset, renames old → new, `blake3 ok`, where it was saved, why it stopped). The mock now writes the same lines for each state. | Fixed (mock) |
| C18 | **The shortcut error didn't say which shortcut failed.** | Developer | "⌃⌥⌘V can't be used: another app has it, or the system keeps it. ⌃⌥⌘C still works." The clipboard panel says so too. On a Mac, Settings also spells it out ("Control + Option + Command + C"). | Fixed |
| C19 | **The update note contradicted the paused screen** ("now continue after the computer sleeps, too"). | Developer | Mock text only; the release notes come from CHANGELOG.md. Replaced in the mock. | Fixed (mock) |
| C20 | **A "Project · Paid" tab on every screen** made people think they'd be charged, or that their photos were a "project". | All four | **Not changed**: Mission 03 says Project Send stays as it is, beyond the rename. Listed for the owner below. | For the owner |
| C21 | **The firewall is open on public networks**, with no "only at home" option, and SeamDrop keeps running when the window is closed. | Privacy | **Not changed**: Mission 01 decided the installer allows Public networks (home Wi-Fi is often marked Public), and the README now explains why. A "receive only on these networks" setting and a pause switch are listed for the owner. | For the owner |
| C22 | **No way to check the first download.** | Privacy | Each release now publishes SHA-256 checksums (`SHA256SUMS.txt` and in the notes) and the update key's fingerprint; the README says how to check. | Fixed |

## Annoys

- Details text is always visible under every screen and looks like an error console (Mac, beginner). Suggested: show only "Details" until opened.
- The accept screen doesn't say where files go or whether they fit, and the compact accept window doesn't say "SeamDrop" (beginner, developer). Return / Esc / ⌘Return don't work (developer).
- No **Pause** button and no **Retry now** while paused (developer).
- The restart after an update clears the clipboard history (developer).
- **Send clipboard** isn't available while a transfer runs, and the next batch can't be queued (developer).
- Shortcut symbols ⌃⌥ aren't known to everyone (Mac): Settings now also spells it out; elsewhere still symbols.
- "Copies go to the other computer and back" sounded like overwriting (Mac): reworded with the consent change.

## Polish

- Settings' floating ✕ can cover text or **Change…** when scrolled, and is announced as "Clear" (Mac, beginner).
- Receiving lists items as "Sending" for screen readers (developer).
- "Ethernet here, Wi-Fi on MacBook Pro" would explain speeds better than "over Ethernet" (developer).
- Folder progress has no "140 of 212 files" count (developer).
- Esc doesn't close the clipboard panel (developer).
- The done screen counts renamed and skipped items but doesn't list them on the screen (Details does) (beginner, developer).
- ".DS_Store and Thumbs.db" is jargon (beginner).
- A "Sent" clipboard list, blurred previews and a retention choice (privacy).
- Delete Portal's old data folder from the app once migration worked (privacy); the README now says it holds the old keys.

## Independent code review (engineering)

Besides the personas, one reviewer read the resume, updater and clipboard code adversarially and probed it with wire-level tests. Fixed, each with a test that failed first:

| Finding | Fix | Test |
|---|---|---|
| A drop between "continue from 2 MiB" and the sender's next message forgot the partial file; the next resume re-sent it under a new name (`big (2).bin`) and delivered a truncated `big.bin`. | Checkpoints keep a resumed partial that hasn't continued yet. | `resume_wire::a_drop_before_the_resumed_file_continues_keeps_what_arrived` |
| After a crash the saved state can be a second behind what's on disk; a file already in staging then failed the transfer ("File exists") and everything was deleted. | Staging files are opened with truncate (staging is the transfer's own folder). | `resume_wire::a_saved_state_from_a_moment_before_a_crash_still_resumes` |
| A repeated question mid-run was answered with the old place, and the sender's continue then killed the live run. | Repeated only while nothing has arrived since; otherwise the link restarts and a fresh check answers. | `resume_wire::a_repeated_question_mid_run_is_never_answered_with_an_old_place` |
| Re-reading a big partial file before continuing could take longer than the 60 s stall limit, so the transfer paused and started the same check again, forever. | The two sides keep each other posted while it checks. | `resume::a_long_check_of_what_already_arrived_does_not_stall_the_transfer` |
| A source changed in place, keeping its size and time, failed the final check instead of starting that file again. | The receiver sends the hash of what it has; a mismatch starts that file again, with a note. | `resume::a_source_changed_in_place_while_paused_starts_that_file_again` |
| An old pause time survived a resume, so a crash long after it counted as "paused for over 24 hours" and removed everything. | Running again clears the pause time on both sides. | `resume::a_crash_long_after_a_resume_does_not_count_the_old_pause` |
| Small finished files weren't flushed before a checkpoint counted them; a power cut could leave a "done" file empty. | Each checkpoint flushes the files finished since the last one. | Not testable without pulling the power; ~0.15 ms per file measured here. |
| Clipboard sync used each computer's clock; with the other clock ahead, "off" here could count as older. | A change here is stamped after any change seen from either side. | `clipboard::a_change_here_is_newer_even_when_the_other_clock_runs_ahead` |
| The updater could start two installs, install after a transfer had started during the download, and on Windows exit without stopping the core. | One install at a time; checks for a transfer again just before installing; stops the core first on Windows. | Manual (§7b). |
| Cancel racing a drop kept the partial files. | Cancel removes them in that case too. | Race; not reproducible on demand. |
| macOS: the clipboard could change between checking its private mark and reading it. Windows: the clipboard stayed open while converting images. | Re-read if it changed; convert after closing. | Real-pasteboard test (Mac), real-clipboard test (Windows CI). |

Listed, not fixed: clipboard sending and sync assume one paired device (several devices are future work, §10); the old-app ("twin") warning can be triggered by a crafted reply on the local network (it only shows a hint); migration removes Portal's staging folder even if Portal is still running a transfer.

## For the owner

1. **Project · Paid tab (C20).** Every reviewer stumbled on it. Options: hide it until Project Send ships; move it into Settings; or keep it and drop the "Paid" badge. Mission 03 said not to change Project Send, so it's untouched.
2. **Public networks (C21).** The installer allows SeamDrop on Public networks so home Wi-Fi marked Public works. A privacy-minded user would like "receive only on my home network" and a "Pause SeamDrop" switch in the menu bar and tray.
3. **Annoys and polish lists above**: none block use; the Details strip, accept-screen destination and free space, and a Pause button were mentioned most.
