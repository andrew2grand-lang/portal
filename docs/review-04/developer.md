# SeamDrop review: the developer who moves big files

**Persona.** I'm a developer who moves 20–80 GB VM images and build folders with thousands of files between a MacBook Pro and a Windows workstation, both on wired Ethernet, every day. A transfer like that takes 5 to 15 minutes, so I look at the numbers. I want speed and time-left figures that are honest, proof that a dropped connection picks up where it stopped instead of resending 40 GB, and enough in **Details** to diagnose a problem without opening `seamdrop.log`. I tried the real screens against the simulated other computer at 552×780 (the default), 420×640 and 1280×900, in both window styles. I read only the README.

**Limits of what I could see.** The simulation sends 633 MB (and one 4.2 GB file in the several-transfers view) at a steady 64 MB/s. I couldn't see a 20–80 GB transfer, real speed changes or a long time-left estimate. In the browser, **Copy technical log** doesn't place anything I could read, so I couldn't check what the log contains. When I sent files myself, the simulated PC never accepted. Some of the inconsistencies below may come from the simulation, and I say where I suspect that. Even so, they're what a user sees on these screens.

## Tasks tried

| # | Task | Finished? | Steps | Verdict |
|---|---|---|---|---|
| 1 | Watch a send run for ~20 s (`sending`) and a receive (`receiving`) | Yes | 0 clicks, 6+ looks | The numbers agree with each other (MB done, MB/s and time left add up), but the "current file" never changes, and the transfer sits at 100 % with "A few seconds" and **Cancel** for 15+ s |
| 2 | Add a folder and a file, then send over Ethernet (`online-ethernet`, Windows) | Partly | 3 (Add a folder, Add files, Send) | The folder count ("Counting…" → 212 files) and the ".lnk can't be sent" warning were good. The simulated Mac never accepted. Details didn't log the offer and said "over Wi-Fi" while the window said Ethernet |
| 3 | Connection drops and comes back (`stalled` → `paused-out`/`paused-in` → `resuming` → `done-out`) | Yes | 0 | The resume story is the best part of the app. While paused, the header says "Connected" |
| 4 | Cancel a send and a paused receive, then send again | Yes | 2 (Cancel, Stop sending) + 1 (Send again) | Clear inline confirmation. It doesn't say how much arrived data gets thrown away |
| 5 | Understand a failure (`failed`, disk full) | Yes, via Details | 1 (Details) | The screen has no numbers. Everything that had arrived is thrown away, and **Send again** starts from zero |
| 6 | Done screens: where files went, whether every file was checked, what was renamed or skipped | **No** | 2–3 (screen, then Details) | Where files went and "every file checked" are clear. I could not find out which items were skipped, and on the sending side not which name was changed |
| 7 | Diagnose from Details; Copy technical log; Send feedback | Partly | 1 each | Good technical level (IPs, port, protocol, error codes, BLAKE3). Timestamps are out of order, and in several states the log contradicts the screen. Send feedback only saves a file |
| 8 | Connecting, offline, Connect by IP, Connection check | Partly | Offline 0; Connect by IP 3; Connection check 2 (gear, then Connection check) | The offline screen is strong. Connect by IP accepted `192.168.1.300`. Connection check blamed the network while the PC was connected |
| 9 | Several transfers at once (`many-mixed`, `many-offers`, 6 devices) | Yes | 0–1 (scroll) | I can see everything, but no speeds, a wrong "2 transfers under way" header, and no way to open one transfer |
| 10 | Window at 420×640 / 552×780 / 1280×900 | Yes | — | Good at the default size. At 420×640 the transfers are crowded out. At 1280×900 the extra space is unused |
| 11 | Update arrives during a transfer (`sending&update=ready`, `paused-out&update=ready`) | Yes | 1 | It waits for a running transfer and says so. While a transfer is paused it restarts at once without saying the transfer survives |

## Findings

### Blocks use

**1. I can't find out which items were skipped, or which name was changed.**
- Screen: http://localhost:5180/?mock=done-in&os=mac, and its Details. Also http://localhost:5180/?mock=done-out&os=mac
- Screenshots: `shots/dev-06-done-in.png`, `shots/dev-07-details-done-in.png`, `shots/dev-05-done-out.png`
- What happened: The receive screen says "2 items left out on Desk PC: links and system files aren't sent." and "1 name changed so it works on macOS." Neither line can be clicked or hovered for more. Details lists the rename but never names the 2 left-out items. On the sending side (done-out), "1 name changed so it works on Windows." isn't in Details at all, so the sender can't learn which file it was.
- Why it matters to me: Build folders are full of symlinks (`node_modules/.bin`, versioned `.so`/`.dylib` links, framework bundles). If SeamDrop quietly leaves 2 of them out, the build breaks on the other machine, and I have to diff the two trees to find out why. For this task I couldn't finish.
- Suggested fix: Make each note open a list (path, then reason: "symlink → ../lib/libfoo.so.1", "system file"). Log every skipped and renamed path in Details on **both** computers. Consider an option to send symlinks as symlinks between two computers that support them.

### Confuses

**2. A paused transfer says "lost connection" while the header says "Connected".**
- Screen: http://localhost:5180/?mock=paused-out&os=mac (also `paused-in`, and `&devices=3`)
- Screenshot: `shots/dev-04-paused-says-connected.png`
- What happened: The card title says "Paused: lost connection to Desk PC". The header above it says "Windows · Connected" with a Wi-Fi pill, and with 3 devices the Desk PC card shows "Wi-Fi" under "3 connected". The `stalled` state gets this right ("Reconnecting…").
- Why it matters to me: When a transfer pauses, my first question is "is the other side reachable?" The window gives both answers at once, and I can't tell whether to walk over to the PC or wait.
- Suggested fix: While a transfer is paused for lost connection, show the device as "Not reachable" or "Reconnecting…" everywhere, or explain the gap ("Connected again, waiting for the transfer to restart").

**3. The finished speed is inflated and disagrees with the live speed.**
- Screen: http://localhost:5180/?mock=done-out&os=mac (and `done-in`), compared with http://localhost:5180/?mock=sending&os=mac
- Screenshot: `shots/dev-05-done-out.png`
- What happened: The live screen showed a steady "64 MB/s" for the whole transfer. The done screen says "633 MB in 7.4 s (85 MB/s)", and right below, "the 280 MB already on Desk PC wasn't sent again". So the average counts 280 MB that didn't cross the wire this time, and it's higher than any speed the live screen ever showed. Details agrees: "632.5 MB … in 7.4 s, 85.5 MB/s". It's unclear whether 7.4 s includes the time paused.
- Why it matters to me: I use this number to judge whether my wired link is healthy (a 1 GbE link should give ~110 MB/s). An average that counts resumed bytes makes a slow link look fine.
- Suggested fix: Base the average on bytes actually sent and time actually moving: "353 MB sent in 3.0 s (118 MB/s); 280 MB was already there; paused 2 s". Use the same calculation for the live figure and the summary.

**4. The transfer sits at 100 % with "A few seconds" and Cancel, and nothing says what it's doing.**
- Screen: http://localhost:5180/?mock=sending&os=mac (also in `many-mixed`)
- Screenshot: `shots/dev-02-sending-at-100.png`
- What happened: The transfer reaches 100 % about 7 s after opening. Then for 15+ s the screen still says "Sending to Desk PC", "633 MB of 633 MB", "64 MB/s over Wi-Fi", "A few seconds", with all three rows ticked and Cancel showing. In the several-transfers view the Desk PC row sat at "100 % · A few seconds" for the rest of my 24 s. This may be the simulation not moving on to `done-out`. Even so, no state between 100 % and done tells me what's happening.
- Why it matters to me: With an 80 GB image, the final check on the other side can take a while. "100 %, 64 MB/s, a few seconds" that doesn't move looks exactly like a hang, and I'd be tempted to cancel.
- Suggested fix: At 100 %, change the line to "Checking files on Desk PC… 31 of 80 GB" (with its own progress), hide the speed, and say "Finishing" instead of a time.

**5. The "current file" label never changes.**
- Screen: http://localhost:5180/?mock=sending&os=mac (also `receiving`)
- Screenshot: `shots/dev-01-sending-current-file.png`
- What happened: The mono line under the stats shows `IMG_0412.HEIC` the whole time: through all 212 files of Site photos, while Site photos is ticked and **Floor plan.dwg** has the spinner, and at 100 %.
- Why it matters to me: With a build folder of 3,000 files, the current-file line is how I see it's moving and where it slows down (thousands of tiny files compared with one big one). A frozen name reads as "stuck on this file".
- Suggested fix: Update it as files change, perhaps a few times a second at most, and add a file count: "1,204 of 3,418 files · src/gen/proto/foo.pb.go".

**6. Details is out of order, and in several states it contradicts the screen.**
- Screens: http://localhost:5180/?mock=failed&os=mac, `?mock=offline&os=mac`, `?mock=online-ethernet&os=win` (after Add a folder → Send), `?mock=stalled`, `?mock=incompatible`, `?mock=many-mixed`. Open Details on each.
- Screenshots: `shots/dev-09-details-failed.png`, `shots/dev-10-details-offline.png`, `shots/dev-11-details-ethernet.png`
- What happened:
  - **Order:** "What happened, newest last" shows "13:49:47 SeamDrop started", then "13:49:49 Connected", then "13:49:42 Offered 3 items". The transfer appears to start before the app did. This happens in every transfer state I opened.
  - **Offline / connecting / incompatible / blocked-peer:** the window says "isn't reachable", "Connecting…", "Older version" or "doesn't accept files", but the last Details line in each is "Connected to Desk PC over Wi-Fi". Nothing records the failed attempts, the version mismatch or the refusal.
  - **Ethernet:** the Windows window says **Ethernet**, but Details says "Connected to MacBook Pro over Wi-Fi". The offer I had just sent isn't logged.
  - **Stalled:** "Connection interrupted. Waiting…" isn't logged at all.
  - **Several transfers:** Details shows only one Desk PC transfer, which it says "Lost the connection … at 44 %". On screen, Desk PC is sending fine, and the paused transfer is the Old Laptop one, at 18 %.
- Why it matters to me: The README says Details "tells what happened". If its order and facts disagree with the window, I can't trust it, and I'm back to digging through `seamdrop.log`, which this panel is meant to save me from.
- Suggested fix: Sort by time (or show the order events happened in, without timestamps that contradict it). Log every change the window shows, with its cause in the second line: "No answer from 192.168.1.31:47920 (timed out after 5 s, attempt 4)", "Desk PC speaks protocol 2; this Mac speaks 3", "No data for 10 s; reconnecting". Name the device on every line when there are several.

**7. Connection check blames the network while the PC is connected.**
- Screen: http://localhost:5180/?mock=online-ethernet&os=mac → gear → Connection check; also `?mock=offline&devices=3&os=mac`
- Screenshot: `shots/dev-12-check-while-connected.png`
- What happened: With "Windows · Connected" in the header, the check says "Not sure: SeamDrop is open on the other computer", "Not sure: Both are on the same network", and "Likely cause: Your network may keep devices apart". With 3 devices, 2 of them connected over this same network right now, it lists "Likely cause: Local Network access on this Mac" and "network keeping devices apart". It never says which device it's checking.
- Why it matters to me: A diagnosis that ignores a working connection sends me off changing macOS permissions that are clearly fine.
- Suggested fix: Feed live facts into the check ("Desk PC connected 2 min ago over Ethernet", "2 other devices reachable, so the network and Local Network are fine"). Let me run it for a chosen device. Show the raw evidence under each line (mDNS seen or not, TCP to :47920 refused, timed out or succeeded).

**8. Connect by IP accepts an impossible address, and afterwards a paired PC shows "Not paired yet".**
- Screen: http://localhost:5180/?mock=offline&os=mac → Connect by IP → type `192.168.1.300` → Connect
- Screenshot: `shots/dev-13-connect-by-ip-result.png`
- What happened: Connect was enabled for `192.168.1.300` (not a valid IPv4 address). Immediately afterwards the header read "Desk PC · Windows · Not paired yet" and the card read "Desk PC is nearby. Pair once…", although Desk PC was paired a moment before. A valid address (`192.168.1.31`) gave exactly the same result. The second part may be the simulation, but no validation reached the screen.
- Why it matters to me: I'd think pairing was lost and that I have to redo codes on both machines. And I'd never learn that I had mistyped the address.
- Suggested fix: Validate as I type ("192.168.1.300 isn't a valid address"). Accept an optional `:port`. After connecting, say what happened ("Reached Desk PC at 192.168.1.31:47920; still paired"), or why it failed ("Nothing answered at 192.168.1.300:47920 within 5 s").

**9. Rename messages contradict each other, and the sender sees names that aren't on its disk.**
- Screen: http://localhost:5180/?mock=done-in&os=mac → Details; http://localhost:5180/?mock=online-ethernet&os=win → Add a folder, Add files, Send
- Screenshot: `shots/dev-07-details-done-in.png`
- What happened: On the Mac receiving from the PC, the done screen says "1 name changed so it works on **macOS**". Details says "Renamed Résumé: Q3?.pdf to Résumé_ Q3_.pdf: ':' and '?' aren't allowed on **Windows**". On the sending PC, the list shows "Résumé Q3.pdf", and after pressing Send the same row reads "Résumé_ Q3_.pdf". The sending screens show the other computer's name for my own file.
- Why it matters to me: I need to know exactly what changed on which side, so I can fix references (a path in a VM config, a build manifest) on the receiving machine.
- Suggested fix: On the sender, show my original name, with "→ saved as Résumé_ Q3_.pdf" when it changes. Make the reason name the receiving OS correctly. Log the rename on both sides.

**10. "2 transfers under way" when none are, or when four are listed.**
- Screen: http://localhost:5180/?mock=many-offers&os=mac and http://localhost:5180/?mock=many-mixed&os=mac
- Screenshot: `shots/dev-14-many-mixed.png`
- What happened: With two offers waiting for my Accept and nothing moving, the heading says "2 transfers under way". With one sending, one receiving, one paused and one sent, it also says "2 transfers under way".
- Why it matters to me: The number at the top is what I glance at before closing the laptop lid.
- Suggested fix: Count what's actually happening: "2 moving · 1 paused · 2 waiting for you".

**11. Restart to update while a transfer is paused doesn't say the transfer survives.**
- Screen: http://localhost:5180/?mock=paused-out&update=ready&os=mac → Restart SeamDrop to update
- Screenshot: none (the banner reads "Installing SeamDrop 0.4.0 · SeamDrop restarts in a moment." above the paused card)
- What happened: During a running transfer, Restart says it waits ("SeamDrop waits for the transfer with Desk PC, then installs 0.4.0. Nothing is lost."), which is excellent. With the transfer paused, it installs at once and says nothing about the paused transfer.
- Why it matters to me: A paused 80 GB transfer at 60 % is 48 GB I don't want to resend. The README says resume survives a restart, but this screen doesn't.
- Suggested fix: Add "The paused transfer with Desk PC continues after the restart (48 GB already there)."

### Annoys

**12. Disk full: no warning up front, all progress thrown away, no numbers.**
- Screen: http://localhost:5180/?mock=failed&os=mac, its Details, and http://localhost:5180/?mock=offer-in&os=mac
- Screenshots: `shots/dev-08-failed-disk-full.png`, `shots/dev-09-details-failed.png`
- What happened: The offer screen says "3 items, 633 MB. Saved in Downloads/SeamDrop if you accept" and doesn't check free space. The failure screen says "Desk PC ran out of space. … nothing was saved on Desk PC." with OK / Send again. Only Details says where it stopped ("at 61 %"). No screen says how much space was needed or free. **Send again** goes back to "Waiting to accept", from zero.
- Why it matters to me: An 80 GB image failing at 61 % means 49 GB moved, about 7 minutes at wire speed, then thrown away. This is the failure I'm most likely to hit.
- Suggested fix: Check free space when the offer arrives ("Needs 80 GB; 52 GB free in Downloads/SeamDrop"), with a choice of folder. If space runs out mid-transfer, **pause** instead of failing ("Free 28 GB on Desk PC, and it continues"), keeping what arrived, the same way a lost connection does.

**13. Cancel is the only control, and it doesn't say how much it throws away.**
- Screen: http://localhost:5180/?mock=sending&os=mac → Cancel; http://localhost:5180/?mock=paused-in&os=mac → Cancel
- Screenshot: none (the confirmation reads "Stop sending? Your files stay on this Mac; the part that arrived on Desk PC is removed.")
- What happened: The confirmation is clear and safe, with **Keep sending** as the default. But it doesn't say how much arrived. The paused-in note does ("The 280 MB that arrived…"), but the confirmation doesn't. There's no Pause, so stopping for a while means cancelling or pulling the cable. Escape doesn't dismiss the confirmation.
- Why it matters to me: Sometimes I need the bandwidth for 10 minutes, or need to take the laptop to a meeting. Cancelling an 80 GB transfer at 70 % is expensive, and I want to see the number before I confirm.
- Suggested fix: Put the amount in the confirmation ("the 56 GB that arrived on Desk PC is removed"). Add **Pause**, which reuses the 24-hour resume path. Make Escape choose Keep.

**14. A stalled transfer gives no sense of time.**
- Screen: http://localhost:5180/?mock=stalled&os=mac (watched for 24 s)
- Screenshot: `shots/dev-03-stalled.png`
- What happened: "Connection interrupted. Waiting for it to come back…" with "Waiting" and a spinner on the active row, unchanged for 24 s. It doesn't show how long it has been stalled, when data last arrived, how many reconnect attempts there have been, or when it will become a pause. Details logs nothing. The paused screens say "within 24 hours" without giving the deadline.
- Why it matters to me: On a wired link, 5 s without data and 2 minutes without data mean very different things (a cable or switch problem compared with the PC going to sleep).
- Suggested fix: "No data for 0:42 · reconnecting (attempt 3)". On pause, show the real deadline: "Kept until tomorrow 14:05".

**15. With several transfers I can't see speed or open one.**
- Screen: http://localhost:5180/?mock=many-mixed&os=mac (also at 1280×900, `os=win`)
- Screenshot: `shots/dev-14-many-mixed.png`
- What happened: Each row shows only "57 % · A few seconds". There's no MB/s and no total, even at 1280 px wide with a lot of empty room. Clicking a row does nothing, so I can't see its files, its current file or its link. The finished row "Sent to Office PC" has only an ✕: no "every file checked", no destination, no rename or skip notes.
- Why it matters to me: When two transfers share my uplink, per-transfer speed is how I see which one is hogging it. The done row loses exactly the checks the single-transfer view promises.
- Suggested fix: Add "64 MB/s" to each row and a total in the heading. Let a row open the full single-transfer view. Keep the checked, destination and notes line on finished rows.

**16. With several devices, an unreachable one has no way to show why.**
- Screen: http://localhost:5180/?mock=offline&devices=3&os=mac
- Screenshot: none (the Desk PC card says "Not reachable"; clicking it only toggles its tick)
- What happened: With one device, "Desk PC isn't reachable" comes with the IP, a checklist and fix buttons. With three devices, Desk PC's card just says "Not reachable". Clicking it toggles the tick. Settings → Paired devices shows "Windows · Not reachable" with no "Why?". Connection check is generic (see 7).
- Why it matters to me: I have a home server and a laptop paired too. Once there are several, the good diagnosis screen disappears.
- Suggested fix: Add a "Why?" link on a not-reachable card and in its Settings row that opens the single-device diagnosis for it.

**17. At 420×640, the important parts are crowded out.**
- Screens: http://localhost:5180/?mock=many-mixed&os=mac, `?mock=offline`, `?mock=paused-in` at 420×640
- Screenshot: `shots/dev-15-small-window.png`
- What happened: In the several-transfers view, the 6-device grid takes most of the window, so only one transfer row shows and its Cancel is cut off. In the offline view, three stacked full-width buttons leave a ~150 px strip for the checklist, and the Mac's address is half hidden behind the buttons. In paused-in, the note about where the partial data waits is below the fold. In sending, the third file row hides behind Cancel.
- Why it matters to me: I keep SeamDrop small in a corner while a long transfer runs.
- Suggested fix: Collapse the device grid to one line ("6 devices · 4 connected ▾") while transfers are running. At narrow widths, put the buttons side by side, or let them scroll with the content.

**18. Send feedback doesn't send.**
- Screen: any → Details → Send feedback
- Screenshot: none (the footer reads "Feedback by email isn't set up yet. The technical log is saved in Downloads: SeamDrop technical log 2026-11-15 19-31 UTC.txt.")
- What happened: It only saves a file. The file name's time (19-31 UTC) didn't match the clock (13:50 UTC), and it was the same on every try, so it may be fixed text in the simulation.
- Why it matters to me: A button called Send feedback that doesn't send looks unfinished. Saving the log to a file is useful, though.
- Suggested fix: Call it "Save technical log" until email works, and stamp the file with the real time.

### Polish

**19. The resume check has no progress.** http://localhost:5180/?mock=resuming&os=mac shows "Checking the 280 MB that already arrived" with only "Checking…". For 40 GB, show "Checking 12 of 40 GB".

**20. The link pill could tell me more.** It says "Wi-Fi" or "Ethernet" and does nothing on hover or click. Show the link speed ("Ethernet · 1 Gb/s") and the peer address on hover, so I can compare MB/s with what the link can carry.

**21. Units are mixed.** The screens say "633 MB", Details says "632.5 MB" and "1 MiB chunks". State whether MB is 10⁶ or 2²⁰ bytes, and use one convention everywhere. Offering Gb/s next to MB/s would help anyone comparing against a network link.

**22. No version numbers anywhere.** http://localhost:5180/?mock=incompatible&os=win says "older version" but shows no versions on screen or in Details. Add "MacBook Pro: 0.2.4 (protocol 2) · this PC: 0.3.1 (protocol 3)".

**23. The connecting screen hides the address.** http://localhost:5180/?mock=connecting&os=mac says "Trying the address it was last seen at". Show it: "Trying 192.168.1.31:47920".

**24. The wide window isn't used.** At 1280×900 (`?mock=sending&os=win`) the three numbers are ~550 px apart and half the card is empty. Group the stats, and consider showing Details beside the transfer at wide sizes.

**25. Screen readers hear "Sending" on the receiving side.** In `receiving`, each row's hidden status text says "Sending" / "Queued". It should say "Receiving".

## What worked well

- **The resume story.** Paused-in says the 280 MB waits in a hidden folder in Downloads/SeamDrop for 24 hours, gets checked again before continuing, and is removed on Cancel. Resuming says it's "Checking the 280 MB that already arrived". Done-out says "the 280 MB already on Desk PC wasn't sent again". Details adds "Resumed at 280 MB (nothing sent twice) … file 3 of 214". This is exactly the reassurance I wanted.
- **Honest time left.** It starts with "Working out time left…" instead of a wild guess. Then it uses coarse wording that roughly matched my arithmetic for the 4.2 GB file at ~63 MB/s ("Less than a minute" at 2.9 GB left, "About 20 seconds" at 1.6 GB left, where I worked out about 25 s). Bytes done, MB/s and time left agree on the live screen.
- **Done screens say where and whether files were checked.** That includes the path on the other computer ("Saved in Downloads\SeamDrop on Desk PC; every file was checked there"), and Details names the hash (BLAKE3) and the file count.
- **Cancel asks first** and says what it removes and where. The safe button is the default.
- **The offline screen** shows the IP it can't reach, ranks causes as Likely / Not sure / Fine, and has real fixes (Open Settings; on Windows, **Fix it** for the specific firewall rule, and the Public-network explanation).
- **Details is at the right technical level:** IP:port, mDNS compared with dialled, protocol version, chunk size, error codes (`disk_full`). It scrolls to the newest entry, and Copy technical log confirms in the footer.
- **Updates never interrupt a running transfer,** and the banner says so.
- **Adding a folder** shows "Counting…" and then the real file count. An item that can't be sent (`.lnk`) is flagged in the list before I press Send.
