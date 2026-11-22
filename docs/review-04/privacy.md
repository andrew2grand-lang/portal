# SeamDrop review 04: privacy-minded user

**Who I am.** I share a flat, and my housemates are on the same Wi-Fi. I use a password manager and private browser windows every day, I read every permission prompt, and I assume any "sync" feature leaks until it shows me otherwise. I want to move files between my own Mac and my own Windows PC with nobody else on the network able to get in. I also want to know exactly where my clipboard goes and how long it stays there. I judged SeamDrop only from its screens (the simulated app at `localhost:5180`, mostly 552×780, Mac and Windows windows) and the user part of the README.

## Tasks I tried

| # | Task | Finished? | Steps | Verdict |
|---|---|---|---|---|
| 1 | Deal with a pairing request from a computer I don't trust (Ignore) | Yes | 1 | The request screen is good. After Ignore, nothing confirms what happened, nothing is recorded, I can't block the computer, and the screen invites me to pair with it (F1, F7) |
| 2 | Pair my Mac and PC by typed code | Yes | 5 (Pair → Show code there → type 6 digits → Pair → OK) | Works well. The code field takes digits only, and three wrong codes end the attempt |
| 3 | See what wrong codes do, on both computers | Yes | 3 wrong codes | Clear on the typing side. On the side showing the code, nothing tells me to cancel if I'm not the one typing (F3) |
| 4 | Find out that pairing was locked after wrong codes, and unlock it | Only by chance | 3 (gear → scroll → Allow pairing again) | The main window says nothing. The notice sits in Settings with no who, when or how long (F2) |
| 5 | Make sure I'm pairing *my* PC and not a housemate's computer with the same name | No, I couldn't check | n/a | The list of nearby computers shows only a name and an OS (F5) |
| 6 | Send something a password manager marked private | Refused, correctly | 1 | Right behaviour and a clear reason, but the message is small and only in the footer (F15) |
| 7 | Send a secret on purpose through the Shelf's text box | Yes | 2 | It sends, but the secret is shown in plain text, spell-checked, and arrives as an ordinary readable item (F8) |
| 8 | Find out what the Shelf keeps and for how long | Partly | 1 | "In memory until SeamDrop quits": it never quits on its own. There's no per-item remove, and one size limit contradicts another (F9, F10) |
| 9 | Accept a sync request, pause sync, turn it off | Yes | 1 each | Good: both computers must opt in, a chip shows it's on, the pause shows its end time, turning off is immediate. The off button is cut off (F14) |
| 10 | Read Details for anything it shouldn't reveal, or should record | Yes | 1 | Clipboard entries show type and size only (good). It shows file names and my home-folder path, and records none of the trust events (F7, F13) |
| 11 | Read Settings for where my data goes | Yes | 1 + scrolling | Mostly honest and specific. The name field understates who sees my name (F6), and arrivals go straight to the clipboard by default (F11) |
| 12 | Remove a paired computer, and see the other side | Yes | 3 (gear → Remove → Remove) | The confirmation is clear. What happens to that computer's Shelf items isn't said (F12) |
| 13 | Send feedback | No (feedback isn't set up yet) | 1 | It silently saved a log file in my Downloads instead (F13) |

## What worked well

- **The pairing request is honest.** "Desk PC (192.168.1.31) wants to pair. Show a code only if you're pairing this computer with Desk PC yourself. Once paired, it can offer you files and send its clipboard." It shows the requester's address, warns me, and says exactly what pairing grants. The README's promise also holds up on screen: the code shows on the computer being asked and is typed on the one asking, so a housemate on another computer can't see it.
- **The code entry is careful.** It works once and only on that computer, and after 3 wrong codes "Nothing was paired. Pair again for a new code." The field ignores letters and is marked as a one-time code.
- **Private clipboard content is refused**, with the reason and a deliberate way round it: "The app you copied this from marked it as private, so it wasn't sent. To send it anyway, paste it into the Shelf's text box."
- **Automatic sync is opt-in on both computers.** The request says "Nothing syncs unless you turn it on." A "Clipboard sync on" or "Clipboard sync paused" chip stays visible in the main window. "Pause for 1 hour" gives an end time ("Paused here until 14:58") and says the pause is local, and "Turn off sync" works at once.
- **Copied files aren't sent silently.** "2 files on the clipboard · Send them to Desk PC" waits for me.
- **Details never shows clipboard content**, only "Clipboard sent to Desk PC ×3, text, 3.2 KB".
- **Per-device controls exist and are plain.** Files from Desk PC: Ask every time / Always accept / Never. Clipboard from Desk PC: Straight to the clipboard / Shelf only. Remove explains its consequences before it acts.
- **The update check says what it sends** ("nothing about you or your files") and that only signed updates install. The README is upfront: no account, no cloud, encrypted, and which firewall rules get added.

## Findings

### F1. After I press Ignore, the app invites me to pair with the computer I just refused
- **Screen:** `http://localhost:5180/?mock=pairing-incoming&os=mac`, then **Ignore**
- **Screenshots:** `shots/privacy-01-request.png` → `shots/privacy-02-after-ignore.png`
- **What happened:** The request disappears. In its place: "Desk PC is nearby. Pair once and they'll know each other from then on…" with **Pair** as the dark main button. Nothing says "Ignored", and there's no way to stop that computer from asking again. Details doesn't record that a request came or that I ignored it.
- **Why it matters to me:** If a housemate's laptop asks to pair, I want my "no" to stick and to be remembered. Instead the very next screen pushes me toward pairing with them. If they ask ten times, I get ten cards and no record.
- **Suggested fix:** After Ignore, show a short confirmation: "Ignored Desk PC (192.168.1.31). Nothing was shared." Offer **Don't show requests from this computer**. Write the request and the Ignore into Details. For a computer I've ignored, use a quieter screen without the main Pair button.
- **Severity:** confuses

### F2. The lockout after wrong codes is hidden in Settings and doesn't say who, when or how long
- **Screen:** `http://localhost:5180/?mock=pairing-locked&os=mac`. The main window looks exactly like the ordinary "Desk PC is nearby" screen (as in privacy-02). The notice is under gear → Paired devices.
- **Screenshot:** `shots/privacy-05-locked-settings.png`
- **What happened:** "A wrong pairing code was typed too many times, so requests to pair with this computer are turned down for now. [Allow pairing again]". Nothing shows on the main window, and nothing goes into Details. Pressing Allow pairing again makes the notice disappear without a word.
- **Why it matters to me:** This is the one sign that someone on my network may have tried to get in, and I'd only find it if I happened to open Settings and scroll. "For now" doesn't tell me whether it's 5 minutes or until I act. I can't tell which computer it was, or whether it was me making a typo.
- **Suggested fix:** Put it on the main window (and in a notification): "Someone at Desk PC (192.168.1.31) typed 3 wrong codes at 14:02. Pairing requests are turned down until 14:32, or until you allow them." Record it in Details. Keep the Settings button.
- **Severity:** confuses

### F3. The computer showing the code doesn't tell me to cancel when someone else is typing
- **Screen:** `http://localhost:5180/?mock=pairing-wrong-there&os=mac`
- **Screenshot:** `shots/privacy-04-wrong-code-there.png`
- **What happened:** "A wrong code was typed on Desk PC. 2 tries left." The code stays in large type, and the line "It works once, for the next 2 minutes, and only on that computer" is gone.
- **Why it matters to me:** If I'm not the one at Desk PC, a wrong code means someone else is guessing. The screen doesn't say so, and the code stays up for anyone walking past.
- **Suggested fix:** "A wrong code was typed on Desk PC. If that wasn't you, press Cancel: nothing gets paired." Make Cancel the main button in this state, and keep the time-left line.
- **Severity:** confuses

### F4. "Works for the next 2 minutes", but the code and the request never seem to expire
- **Screen:** `http://localhost:5180/?mock=pairing-show&os=win` and `?mock=pairing-incoming&os=win`, left open for 140 seconds
- **Screenshot:** `shots/privacy-06-code-after-140s.png`
- **What happened:** After 2 min 20 s the same code was still on screen, still saying "for the next 2 minutes", with no countdown. The incoming request also stayed up for as long as I watched. (This may be a limit of the simulation, but it's what the screen shows me.)
- **Why it matters to me:** In a shared flat I may walk away from a code or a request. I need to know when it stops being usable, and the screen should clear itself.
- **Suggested fix:** Show a visible countdown. At zero, replace the code with "Code expired. Nothing was paired." Let unanswered requests lapse after the same 2 minutes, with "Desk PC's request ran out" recorded in Details.
- **Severity:** confuses

### F5. When I start pairing, I can't tell my PC from a housemate's computer with the same name
- **Screen:** `http://localhost:5180/?mock=add-device&os=mac&devices=2`, then gear → **Pair another computer** (also `?mock=nearby&os=mac`)
- **Screenshot:** `shots/privacy-07-pair-another.png`
- **What happened:** The list shows only "Desk PC · Windows · Pair". The address appears only on the *other* computer's request screen, after I've already asked.
- **Why it matters to me:** Computer names are chosen by their owners, so a housemate can call theirs "Desk PC" too. The code means picking the wrong one fails rather than leaking anything. But I'd still be asking a stranger's computer to pair with me, and showing them my name and address.
- **Suggested fix:** Show each computer's address in the nearby list and on the "is nearby" screen ("Desk PC · Windows · 192.168.1.31"). Tell me where to check it: "Your PC shows its address in Settings → This computer's addresses." If two computers share a name, say so.
- **Severity:** confuses

### F6. The Name field says only my other computer sees my name. Everyone on the network does
- **Screen:** gear → This computer → Name (any state, e.g. `?mock=pairing-locked&os=mac`)
- **Screenshot:** `shots/privacy-05-locked-settings.png` (top)
- **What happened:** The help line says "Your other computer shows this name." But my computer shows up by name on every nearby computer running SeamDrop, paired or not. A housemate's request screen shows it, along with my address.
- **Why it matters to me:** Many people name computers "Sam's MacBook". I'd choose differently if I knew my housemates' computers list it.
- **Suggested fix:** "Anyone on this network who runs SeamDrop sees this name. Paired computers use it too." Consider adding an option to stay hidden from unpaired computers once all my devices are paired.
- **Severity:** confuses

### F7. Details records none of the trust events, and says "Connected" to a computer I never paired
- **Screen:** `?mock=pairing-incoming&os=mac` → Ignore → **Details**. Also checked after pairing (`?mock=pairing-type&os=win`, typed 048 213), after Remove, and after turning sync on, pausing and turning it off.
- **Screenshot:** `shots/privacy-03-details-after-ignore.png`
- **What happened:** Details lists "Connected to Desk PC over Wi-Fi · 192.168.1.31:47920, dialled" for a computer that isn't paired. None of these appear anywhere: a pairing request arriving, Ignore, the code being shown, wrong codes, the lockout, "Paired with…", "Removed…", sync turned on, paused or turned off. Entries are also out of order even though it says "newest last" (14:00:29 below 14:00:36; in another run, "Clipboard sent" at 13:59:36 below an entry at 13:59:37).
- **Why it matters to me:** Details is my only audit trail. "Connected" to an unpaired computer reads as if data could flow to it. And the events I care most about (who tried to pair, when sync was on) are exactly the ones missing.
- **Suggested fix:** Word unpaired contacts as "Saw Desk PC (not paired): nothing can be sent to or from it." Record every trust change with time and address. Sort by time.
- **Severity:** confuses

### F8. The way to send a secret on purpose shows it in plain text, and it arrives as an ordinary item
- **Screen:** `?mock=online&os=mac&clip=history` → Shelf → type in "Type or paste text to send" → Send
- **Screenshots:** `shots/privacy-09-shelf-secret.png`, `shots/privacy-08-shelf.png` (how arrivals look)
- **What happened:** The README and the refusal message both point me to the Shelf's text box for private things. The box shows what I type in full and has spell-check on, so passwords get red underlines. After sending, only "Sent to Desk PC (text, 43 bytes)" appears. On the receiving side, arrivals are listed with a readable preview ("“Meeting notes, Thursday: budg…”"). Nothing says whether text I sent on purpose this way is treated as private when it lands, or put straight on the other clipboard.
- **Why it matters to me:** That box is the deliberate route for a password. If the PC shows the first 30 characters on its Shelf, and drops it on the clipboard for anyone at that desk, I've undone what my password manager protects.
- **Suggested fix:** Add a "Private" toggle (with show/hide) to the box, on by default when the text came from a refused private copy. Turn spell-check off there. Show private arrivals as "Private text, 43 bytes" with no preview. Mark them private on the receiving clipboard so clipboard history doesn't keep them, and clear them after a short time.
- **Severity:** confuses

### F9. "Until SeamDrop quits" really means indefinitely, and I can only clear everything at once
- **Screen:** `?mock=online&os=mac&clip=history` → Shelf → Clear
- **Screenshot:** `shots/privacy-08-shelf.png`
- **What happened:** "Kept in memory only, never on disk, until SeamDrop quits or you clear it." But closing the window keeps SeamDrop running in the menu bar or tray, so items can sit there for days. Each item has only **Copy**, with no way to remove one. **Clear** empties the whole list at once without a word. It doesn't say whether it also empties the system clipboard, where "the newest is already" sitting.
- **Why it matters to me:** "Never on disk" is good. But I want arrivals gone when I've used them, not whenever I next restart, and I want to drop one sensitive item without losing the rest.
- **Suggested fix:** Add a retention choice (clear after 1 hour, or when the screen locks, or on sleep). Give each item a remove button. After Clear, say what was cleared ("Cleared 4 items. Your clipboard was emptied too" or "…left as it is").
- **Severity:** confuses

### F10. A 22.4 MB image is "Not kept (too big)" though the limit for images is 25 MB, and I can't tell where it went
- **Screen:** `?mock=online&os=mac&clip=history` → Shelf
- **Screenshot:** `shots/privacy-08-shelf.png` (last row)
- **What happened:** "An image, 6000 × 4000 · image, 22.4 MB · Not kept (too big)". The same panel says "images up to 25 MB go as a clipboard; anything bigger arrives as a file."
- **Why it matters to me:** I want to know where every arrival is. "Not kept" could mean it was thrown away, saved as a file in Downloads, or sitting on my clipboard, and the size rule contradicts it.
- **Suggested fix:** Use one limit everywhere, and say where the item is: "Saved as a file in Downloads/SeamDrop", "On your clipboard, not kept here", or "Thrown away".
- **Severity:** confuses

### F11. Arrivals go straight to my clipboard by default, and the app doesn't say whether clipboard history keeps them
- **Screen:** gear → Paired devices → "Clipboard from Desk PC" (`?mock=online&os=mac&clip=sync`)
- **Screenshot:** `shots/privacy-12-device-settings.png`
- **What happened:** The default is **Straight to the clipboard**. Neither the Shelf nor Settings says whether SeamDrop keeps arrivals out of the operating system's clipboard history (Windows' Win+V history and its "sync across devices" cloud clipboard, or Mac clipboard managers).
- **Why it matters to me:** "Never through a server" covers SeamDrop's own path. But if my PC's clipboard history uploads what SeamDrop put there, my Mac's clipboard can still reach a cloud.
- **Suggested fix:** Say it plainly next to the option, for example: "Arrivals are marked so clipboard history and cloud clipboard don't keep them". If that isn't true, say so and suggest **Shelf only**. Consider making **Shelf only** the default the first time sync is turned on.
- **Severity:** confuses

### F12. After Remove, the Shelf vanishes, so I can't check or clear what that computer sent
- **Screen:** `?mock=online&os=mac&clip=history` → gear → Remove → Remove. Other side: `?mock=revoked&os=win`
- **Screenshot:** `shots/privacy-15-removed-there.png`
- **What happened:** The confirmation is clear ("stops trusting Desk PC at once… Files already received stay"). After it, the Shelf button is gone, and with it the 4 items that came from Desk PC. I can't tell whether they were wiped or are still in memory. On the removed computer: "MacBook Pro removed this PC. It no longer trusts this device." It's unclear which computer "it" is, and Settings there says "None yet" as if nothing had ever been paired.
- **Why it matters to me:** When I remove a device, I want everything it sent me gone too, or at least to know it isn't.
- **Suggested fix:** Add to the confirmation: "Its 4 items on the Shelf are cleared too." On the removed side: "MacBook Pro removed this PC, and this PC forgot MacBook Pro too. Nothing more goes either way."
- **Severity:** confuses

### F13. Send feedback saves a log file in Downloads without asking, and I'm not told what's in it
- **Screen:** Details → **Send feedback** (any state, e.g. `?mock=pairing-incoming&os=mac`). Same button in Settings → About.
- **Screenshots:** `shots/privacy-14-send-feedback.png`, `shots/privacy-13-details-files.png`
- **What happened:** One click and the footer says "Feedback by email isn't set up yet. The technical log is saved in Downloads: SeamDrop technical log 2026-11-15 19-31 UTC.txt". The file is written straight away, and its time doesn't match the 13:48 shown in Details. **Copy technical log** just says "Paste it into a message for support." Judging by Details, the log holds file names ("Résumé_ Q3_.pdf"), my full home path with my username (`/Users/sam/Downloads/SeamDrop`) and the IP addresses on my network. Nowhere does the app list what's in the log.
- **Why it matters to me:** Writing a file I didn't ask for isn't what I expect from a button that says "Send feedback". Before I share a log with anyone, I want to see what's in it and leave out file names.
- **Suggested fix:** Hide or disable Send feedback until it works. Otherwise ask first: "Save a technical log to Downloads? It includes file names, folder paths and network addresses, never file or clipboard contents." Offer a "leave out file names" option and a preview. Also offer a way to clear Details.
- **Severity:** annoys

### F14. The control to stop sync is cut off, and the close button covers the Shelf's Copy buttons
- **Screen:** `?mock=online&os=win&clip=sync` (also `os=mac`) → Shelf → scroll to Automatic sync
- **Screenshot:** `shots/privacy-11-sync-controls.png`
- **What happened:** The row "Pause for 1 hour · Pause until SeamDrop restarts · Turn off sync" runs off the right edge, clipping **Turn off sync**. When the panel is scrolled, the panel's × sits on top of an item's **Copy** button. The same thing happens in Settings, where the × lands on the lockout notice.
- **Why it matters to me:** "Turn off sync" is the most important button in this panel for me, and it's the one that's cut off.
- **Suggested fix:** Let the row wrap, or put **Turn off sync** on its own line first. Keep the × out of the scrolling content.
- **Severity:** annoys

### F15. I can't tell whether text from my private browser windows is held back, and the refusal is easy to miss
- **Screen:** `?mock=online&os=mac&clip=private` → **Send clipboard**
- **Screenshot:** `shots/privacy-10-private-refused.png`
- **What happened:** The refusal works and is well worded, but it appears only as small grey text beside "Details" at the bottom, and it isn't recorded in Details. Every screen in the app says "Anything an app marks as private, **as password managers do**, is never sent". Only the README adds "and private browser windows". There's no list of which apps count, and no way to add one, such as my banking app or a terminal.
- **Why it matters to me:** With automatic sync on, I copy from private windows all the time. I can't check the claim that matters most to me, and I can't add my own exceptions.
- **Suggested fix:** Show the refusal where I'm looking (on the button, or as a notice that stays a few seconds) and record "Not sent: marked private (1Password)" in Details. In Settings → Clipboard, add "Never send from these apps", listing the apps it already treats as private.
- **Severity:** confuses

### F16. Wording that leaves me unsure where data goes
- **Screens:** `?mock=offer-in&os=mac` and `?mock=done-in&os=mac` ("Every file is checked on arrival"); `?mock=nearby&os=mac` ("You'll check a code on both screens")
- **Screenshot:** `shots/privacy-13-details-files.png` (Details shows the check is "BLAKE3, 214 files")
- **What happened:** "Every file is checked on arrival" sounds like a scan, perhaps uploaded somewhere. Details shows it's an integrity check. The nearby screen promises "You'll check a code on both screens", but the real flow shows the code on one computer and types it on the other.
- **Why it matters to me:** "Checked" makes me wonder who checks it. A different description of the code step makes me second-guess whether I'm doing it right.
- **Suggested fix:** "Every file is checked to make sure it arrived complete and unchanged. Nothing leaves your computers." and "One computer shows a code; you type it on the other."
- **Severity:** polish

## By severity

- **Blocks use:** none. I could finish everything I needed, and nobody could pair without seeing my screen.
- **Confuses:** F1 Ignore leads to a Pair prompt, with no record or block · F2 lockout hidden in Settings · F3 no "cancel if it wasn't you" · F4 code and request don't visibly expire · F5 no address when choosing whom to pair · F6 name visibility understated · F7 Details misses trust events and says "Connected" to unpaired computers · F8 Shelf text box shows secrets in plain text and they arrive with a preview · F9 Shelf keeps items indefinitely, no per-item remove · F10 "Not kept (too big)" contradicts the 25 MB limit · F11 clipboard-history handling not stated · F12 Shelf items after Remove · F15 private-window claim can't be checked, no exclusion list
- **Annoys:** F13 Send feedback writes a log to Downloads without asking · F14 Turn off sync cut off, × covers Copy
- **Polish:** F16 "checked on arrival" and "check a code on both screens"
