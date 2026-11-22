# Review 05: three computers

## Who I am

I have three computers at home: a Mac (the one in front of me, "MacBook Pro" in the app), a Windows desktop ("Desk PC") and a second laptop. In the simulation, `devices=3` also gave me a "Studio iMac", so I treated MacBook Air and Studio iMac as my other machines. I move big folders of video between them. Transfers get cut off: a laptop sleeps, someone quits the app, the Wi-Fi drops. I want to start a transfer, walk away, and trust the window to tell me what happens next.

I only used the window at `http://localhost:5180/` with `mock=` states, the Mac and Windows frames, light and dark, at 552×780, 420×640 and 1280×900. I read the user part of the README and nothing else. Screenshots are in `docs/review-05/shots/` (`three-*.png`).

## What I tried

| Task | Finished? | Steps / clicks | Verdict |
|---|---|---|---|
| 1a. Find the computers on first launch | Yes | `?mock=nearby&devices=3`: all three listed with name, OS and IP, each with **Pair** | Clear. Pairing the first one goes straight to "Pair another computer" with the rest. |
| 1b. Pair the third computer when two are already paired | Yes | Settings → **Pair another computer** → **Pair** next to Desk PC → type 048213 → **Pair** → **OK**: 5 clicks plus the code | Smooth. The other side ("MacBook Pro wants to pair · Show code / Ignore") is clear too. |
| 1c. One computer doesn't show up | Partly | "Pair another computer" with nothing new on the network; a paired computer "Not reachable" | Weak. The spinner runs forever, and with three computers the window doesn't help with the one that's missing. |
| 2a. Send a big folder to a computer that is busy | **No** | Tick only the busy one → Add a folder → **Send next** (3 clicks) | The window promises "these go next", then refuses: "Send this when it's done". Nothing goes by itself afterwards. |
| 2b. Send to two at once | Yes | Untick one → Add folder → **Send** (3 clicks) | Works: "2 waiting to be accepted", one row each with its own Cancel. |
| 2c. Work out what goes next, change my mind | Partly | `?mock=next`: scroll inside the card to find "Next for Desk PC: 2 files", **×** removes it (1 click) | Removing works. The queue doesn't name its files, sits below the fold, and vanished without starting when the current transfer ended. |
| 3a. Network drop (sender and receiver) | Yes | `paused-out`, `paused-in`, `many-mixed` | Calm and clear: "It continues by itself". Neither side says which way the files are going. |
| 3b. App quit on one side | Yes (observed) | `stopped-out`, `stopped-in`, `waiting-there`, then **Continue** | It asks you, but doesn't say why it stopped, and the other side doesn't say someone has to press Continue. This contradicts the drop screen. |
| 3c. Transfer stalls | Yes (observed) | `stalled` on Mac and Windows, watched for 60 s | "Waiting for Desk PC…" with no explanation and no next step. |
| 4. Time left and progress | Mostly | Watched a 4.20 GB receive for 50 s (`many-mixed`); resumed a stopped send | Calm, believable and never jumps around. The progress doesn't go back to zero after a resume. The finished summary doesn't mention the interruption. I couldn't test tens of GB: the biggest sample was 4.2 GB. |

## What worked well

- **Pairing several in a row.** After pairing the first one from `?mock=nearby&os=mac&devices=3`, the window goes straight to "Pair another computer" with the remaining two ("Desk PC · Pair", "Studio iMac · Pair", "MacBook Air ✓ Paired"). This is exactly right for three machines (three-105). It's the same panel from Settings in `?mock=add-device&os=mac` (three-09). The code screen counts down: "The code works for 1:58 more." (three-12). The finish is plain: "Paired with Desk PC" (three-15).
- **The other side of pairing.** `?mock=pairing-incoming&os=win`: "MacBook Pro wants to pair. From 192.168.1.24. Show a code only if it's you pairing." Then a big, readable "048 213" (three-92, three-93).
- **Network drop.** `?mock=paused-out&os=mac&devices=3`: "Paused: lost connection to Desk PC. Is the PC asleep, or SeamDrop closed there? It continues by itself." The bar stays at 44 %, "280 MB of 633 MB" (three-paused-out-1). Cancel asks first and says what it removes: "Stop sending? The part on Desk PC is removed." and, on the receiver, "Stop receiving? What arrived is removed." (three-68, three-69).
- **Several transfers at once.** `?mock=many-mixed&os=mac`: the heading "2 moving · 1 paused · 1 done", one row each with its own Cancel, and an inline confirm on the row: "Stop sending? The part that arrived on Old Laptop is removed. Stop / Keep" (three-51, three-94).
- **A busy target, before you press the button.** Ticking only MacBook Air while receiving from it gives "Busy receiving from MacBook Air. 212 files · 611 MB · next for MacBook Air" and a **Send next** button (three-54). That's the right idea; see finding 1 for what happens next.
- **A computer that's switched off.** `?mock=disconnected&os=mac&devices=3`, then Add folder: "Desk PC is not connected, so it won't get these." It's honest, before you send (three-70).
- **Time left on the 4.20 GB receive** (`?mock=many-mixed&os=mac`, three-eta-1 to three-eta-5): "About 40 seconds" at 43 %, "About 30" at 56 %, "About 20" at 68 %, "About 10" at 80 %, then "A few seconds". Each step held about 10 s, which matched the real rate (about 1.5 %/s). It never went back up. It's calm and believable.
- **Out of space.** `?mock=failed&os=mac&devices=3`: "Desk PC ran out of space. Your files are still on this Mac; nothing was saved on Desk PC." **Send again** goes only to Desk PC, not to all three ticked computers (three-failed, three-106).
- **One computer, not reachable.** `?mock=offline&os=win` names the machine and its address ("MacBook Pro isn't reachable … No answer from 192.168.1.24.") and lists checks with a **Fix it** button (three-101). It's very good, but it only appears when you have a single computer (see finding 8).

## Findings

1. **Blocks use: "Send next" doesn't queue anything, and the window contradicts itself.**
   - **What I saw:** Desk PC was busy, so I ticked only Desk PC and added a folder. The window said "Busy sending to Desk PC. 212 files · 611 MB · next for Desk PC" and "Desk PC is busy: these go next.", with a **Send next** button. Pressing it added, below the fold: "Desk PC is busy with another transfer. Send this when it's done (one at a time per device)." At 552×780 that line sits under the bottom of the card, so the button seems to do nothing until you scroll inside the card. When Desk PC finished, nothing was sent. The window switched to "Ready to send … to Desk PC" with a plain **Send** button and still showed "Desk PC is busy with another transfer" while Desk PC's chip said "Wi-Fi". It was the same when the busy computer was sending *to* me (MacBook Air in `many-mixed`).
   - **Where:**
     - `?mock=sending&os=mac&devices=3`: untick MacBook Air and Studio iMac → **Add a folder** → **Send next**.
     - `?mock=many-mixed&os=mac`: untick everything except MacBook Air → **Add a folder** → **Send next**.
   - **Screenshots:**
     - Desk PC case: three-63, three-64, three-65.
     - MacBook Air case: three-60 (nothing visible), three-61 (after scrolling), three-59 (1280 wide), three-62 (after the receive ended).
   - **What would have helped:** **Send next** should actually queue, show the queued item in the card where I can see it, and start it by itself. If queueing really isn't possible, don't offer "Send next" or say "these go next". Clear the "is busy" message once the computer is free.
   - **Note:** This might be the simulation not modelling the queue, but the refusal is in the app's own words.

2. **Confuses: the queued files vanish when the current transfer finishes.**
   - **What I saw:** In `next`, the sending card says "Next for Desk PC: 2 files · starts when the current transfer finishes". About 10 s later it says "Sent 3 items to Desk PC" with **Done**. The 2 queued files never started and don't appear anywhere, not in the window and not in Details. The queue line also doesn't name the 2 files. At 552×780 it's hidden below the file list: I had to scroll inside the card to find it.
   - **Where:** `?mock=next&os=mac&devices=3`. Watch for 15 s; at 552×780, scroll inside the card.
   - **Screenshots:**
     - three-40 (queue visible at 1280×900), three-41 (queue gone, "Sent 3 items").
     - three-37 (queue not visible at 552×780), three-43a (after scrolling), three-43b (after **×**).
   - **What would have helped:** Show "Now sending the next 2 files to Desk PC" and roll straight into it. Name the queued files, or let me open the list. Keep the queue line visible above the fold.

3. **Confuses: with several computers ticked, the busy one is quietly left out, although the window says "these go next".**
   - **What I saw:** All three were ticked while Desk PC was busy. The composer said "212 files · 611 MB · to MacBook Air and Studio iMac" and, underneath, "Desk PC is busy: these go next." After **Send**, Desk PC finished its current transfer and never got the folder, even after 25 s.
   - **Where:** `?mock=sending&os=mac&devices=3` → **Add a folder** → **Send**.
   - **Screenshots:** three-28, three-29, three-31, three-32.
   - **What would have helped:** Either send to Desk PC after its current transfer, with a row like "Next for Desk PC", or say plainly: "Desk PC is busy and won't get these. Send to it later."

4. **Confuses: device chips mix whose point of view the status word is from.**
   - **What I saw:** The chips are a name with a status word under it:
     - "Desk PC · Sending" means *I* am sending to Desk PC.
     - "MacBook Air · Receiving" means *I* am receiving from MacBook Air.
     - "Desk PC · Wants to send" means *Desk PC* wants to send to me.

     Read as a sentence about the named computer, the first two are backwards. At 420×640 the words disappear and only a coloured dot is left (blue, orange or hollow), with no legend.
   - **Where:**
     - `?mock=many-mixed&os=mac`.
     - `?mock=offer-in&os=mac&devices=3`.
     - `?mock=paused-out&os=mac&devices=3` at 420×640.
   - **Screenshots:** three-51, three-100, three-88 (dark, Windows), three-90, three-91.
   - **What would have helped:** Use one point of view, for example "Sending to it" / "Sending to you", or arrows. Keep a short word, or a tooltip, at narrow widths.

5. **Confuses: paused and resuming screens don't say which way the files are going.**
   - **What I saw:** On the Mac with three computers, `paused-out` and `paused-in` look the same: "Paused: lost connection to Desk PC". `waiting-there` says "Paused: waiting for Desk PC to continue", and I only learned it was a *receive* when Cancel asked "Stop receiving?". `resuming` says "Desk PC is back. Checking the 280 MB already here", and again there's no "sending to" or "receiving from". In the multi-row view the paused row ("Paused: waiting for Old Laptop … Paused") leaves out the reassuring "It continues by itself" that the single view has.
   - **Where:** `?mock=paused-in&os=mac&devices=3`, `?mock=waiting-there&os=mac&devices=3` (then **Cancel**), `?mock=resuming&os=mac&devices=3`, `?mock=many-mixed&os=mac`.
   - **Screenshots:** three-paused-out-1, three-paused-in-1, three-waiting-there-1, three-98, three-resuming-1, three-51.
   - **What would have helped:** Keep the direction in the title ("Sending to Desk PC: paused", "Receiving from Desk PC: paused"), and add "continues by itself" to the paused row.

6. **Confuses: after an app quit, it asks without saying why, and the two sides disagree with the network-drop screen.**
   - **What I saw:**
     - On the side that stopped: "Sending Site photos and 2 more to Desk PC stopped partway." with **Continue** / **Cancel**. There's no reason (quit here? quit there?), no word on whether it would continue if I did nothing, and no word on how long it will wait.
     - On the other side: "Paused: waiting for Desk PC to continue". Nothing tells me to walk over to Desk PC and press Continue.
     - The network-drop screen, though, says "Is the PC asleep, or SeamDrop closed there? It continues by itself." If SeamDrop really was closed there, it does *not* continue by itself: someone has to press Continue.
     - The README says a restart continues by itself.
     - Details lists nothing about the stop.
   - **Where:** `?mock=stopped-out&os=mac&devices=3`, `?mock=stopped-in&os=win` (dark), `?mock=waiting-there&os=mac&devices=3`, `?mock=paused-out&os=mac&devices=3`, and **Details** in `stopped-out`.
   - **Screenshots:** three-stopped-out-1, three-stopped-in-1, three-89, three-waiting-there-1, three-paused-out-1, three-det-stopped-out.
   - **What would have helped:**
     - Say why: "SeamDrop was quit on this Mac at 44 %".
     - Say what happens if I do nothing, and until when: "Kept for 24 hours".
     - On the other side, say "Press Continue on Desk PC, or it continues when …".
     - Make the drop screen's "SeamDrop closed there" line match what actually happens.

7. **Confuses: a stalled transfer gives no explanation or next step.**
   - **What I saw:** The bar stays at 44 % with an hourglass. The right side says "Waiting for Desk PC…" and the first file has a spinner. This lasted 60 s with no change. On the Mac with three computers, the chip still says "Sending" and the header says "3 connected", so nothing else hints at a problem. On Windows the header says "Reconnecting…". Details says "No data from Desk PC for a few seconds; waiting for the connection", which is better than the window. Nothing says whether it will pause, retry or give up, or whether I should do something.
   - **Where:** `?mock=stalled&os=mac&devices=3`, `?mock=stalled&os=win`.
   - **Screenshots:** three-stalled-3, three-74.
   - **What would have helped:** After a few seconds, say "No data from Desk PC for 20 s. It keeps trying, and pauses by itself if Desk PC is gone." Make the chip match.
   - **Note:** A real stall would probably turn into a pause sooner; the frozen 60 s may be the simulation. The missing explanation isn't.

8. **Confuses: when one of three paired computers isn't reachable, the window offers no help.**
   - **What I saw:** The chip says "Desk PC · Not reachable" and the header "2 connected · 1 not reachable". The big area still just says "Drop files here", and the chip only ticks or unticks. Settings → **Connection check** is generic: "SeamDrop is open on the other computer — Open SeamDrop there too". It doesn't say *which* computer or which address got no answer. Compare the single-computer screen, which names "MacBook Pro" and "No answer from 192.168.1.24" and offers **Fix it**.
   - **Where:** `?mock=offline&os=mac&devices=3` → Settings → **Connection check**; compare `?mock=offline&os=win`.
   - **Screenshots:** three-offline-1, three-102, three-101.
   - **What would have helped:** A small "Why?" on a not-reachable chip that opens the same named checklist as the single-computer screen.

9. **Annoys: "Pair another computer" spins forever when the computer I want isn't found.**
   - **What I saw:** "Looking on this network…" with a spinner, unchanged after 40 s. There's no "Still looking" list of checks like the first-launch screen shows after about 10 s. Only **Connect by IP** is offered.
   - **Where:** `?mock=online&os=mac&devices=2` → Settings → **Pair another computer**.
   - **Screenshots:** three-20, three-21; compare first launch at three-03 and three-17.
   - **What would have helped:** After a while, show the same "Still looking" checklist, with "Is SeamDrop open on it?" and "Same Wi-Fi?".

10. **Annoys: adding something while transfers run hides those transfers.**
    - **What I saw:** As soon as I added a folder during a transfer, the card became the composer ("Ready to send" or "Busy sending to Desk PC."). The running transfer's progress and time left disappeared. Only the chip still said "Sending", plus the line "Transfers under way carry on meanwhile." When that transfer finished, I never saw its "Sent 3 items to Desk PC" result; the composer stayed up.
    - **Where:** `?mock=sending&os=mac&devices=3` → **Add a folder**.
    - **Screenshots:** three-28, three-63, three-65.
    - **What would have helped:** Keep the running transfers as compact rows above the composer, as the multi-row view already does.

11. **Annoys: "Checking…" after a resume has no progress.**
    - **What I saw:** "MacBook Pro is back. Checking the 280 MB already here, then it continues." with "Checking…". It didn't move in 45 s.
    - **Where:** `?mock=resuming&os=win` and `?mock=resuming&os=mac&devices=3`.
    - **Screenshots:** three-85, three-87, three-resuming-1.
    - **What would have helped:** With 30 GB already there, the check could take a while, so show its own progress ("Checked 12 of 28 GB").
    - **Note:** Never moving on is probably the simulation's frozen state. The missing progress isn't.

12. **Polish: the finished summary hides that there was an interruption.**
    - **What I saw:** After **Continue** on a transfer stopped at 44 %, the end screen said "Sent 3 items to Desk PC. 633 MB in 9.9 s (64 MB/s) over Wi-Fi". That's exactly the same line as an uninterrupted send (three-26), and I only watched about 6 s of it.
    - **Where:** `?mock=stopped-out&os=mac&devices=3` → **Continue** → wait.
    - **Screenshots:** three-75, three-77, three-78.
    - **What would have helped:** Something like "continued after a pause; every file checked", so I trust the result after an interruption.
    - **Note:** The identical "9.9 s" on every transfer is likely a simulation constant.

13. **Polish: device icons follow the operating system, not the kind of computer.**
    - **What I saw:** "Studio iMac" and "Kitchen Mac mini" have a laptop icon, and "Old Laptop" has a desktop monitor.
    - **Where:** `?mock=many-mixed&os=mac`, `?mock=online&os=mac&devices=3`.
    - **Screenshots:** three-51, three-05.
    - **What would have helped:** Pick the icon from the kind of machine, or use a neutral one.

14. **Polish: the pause glyph looks like a button.**
    - **What I saw:** The round "⏸" next to "44 %" on paused screens looks like a pause/resume control, but it can't be clicked. I tried to resume with it.
    - **Where:** `?mock=paused-out&os=mac&devices=3`.
    - **Screenshot:** three-paused-out-1.
    - **What would have helped:** Make it look like a status, not a control (no round chip), or make it work.

15. **Polish: small wording inconsistencies.**
    - **What I saw:**
      - "Files and the clipboard can go between them now." ("them" isn't clear; three-15).
      - "Looking for your other computer" is singular when you have several (three-01).
      - Some headings end with a full stop ("Busy receiving from MacBook Air.", "…stopped partway.") and others don't.
      - Cancel pairs differ from screen to screen: "Stop sending / Keep waiting", "Stop / Keep", "Cancel transfer / Keep" (three-68, three-94, three-97).
      - The Windows "Still looking" check says "If the Mac still can't reach this PC", which assumes the other computer is a Mac (three-17).
    - **What would have helped:** One wording per situation, and names instead of "them" or "the Mac".

16. **Polish: the small window clips things.**
    - **What I saw:** At 420×640, a sliver of the file list pokes out under the progress bar on the paused card (three-90). Names are cut off: "Kitchen Mac mi…" (three-91), and at 552 wide "Received from MacBoo…" next to **Show in Finder** (three-eta-6).
    - **What would have helped:** Wrap long names, or move **Show in Finder** under the row.

## Not counted (simulation)

- The other computers never accept an offer in the simulation ("Waiting for … to accept" forever). That needs a person.
- After **Send**, rows show "Site photos and 2 more · 633 MB" even when I added only "Site photos · 611 MB" (three-48 → three-49). This looks like the simulation reusing its sample transfer.
- `?mock=add-device&devices=3` lists "Studio iMac" twice and no Desk PC (three-06). This comes from mixing parameters.
- In `sending`, all three are ticked but only Desk PC is being sent to; this is the simulation's starting state.
- Details timestamps run ahead of the clock in some states, and stopped states log nothing (three-det-*).
- The biggest simulated transfer is 4.2 GB, so I couldn't see how time left reads for tens of GB (minutes and hours).
