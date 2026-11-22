# SeamDrop review: four computers at home

**Persona.** I have four computers at home: a Mac Studio in the office, a MacBook Air, a Windows gaming PC and an old Windows laptop in the kitchen that is usually asleep. I'm comfortable with computers and impatient with fiddly apps. I want to send a folder to two of them at once, see which ones are on at a glance, and not have the kitchen laptop get in the way. I used the real screens in a browser against simulated computers (`http://localhost:5180/?mock=…&devices=4|6`), at 552×780 (default), 420×640 (smallest) and 1280×900, on the Mac and Windows windows. The simulated names don't match mine exactly (Desk PC, MacBook Air, Studio iMac, Old Laptop; with six devices, Office PC and Kitchen Mac mini too). "Old Laptop" stands in for my kitchen laptop.

## Tasks I tried

| # | Task | Finished? | Steps | Verdict |
|---|------|-----------|-------|---------|
| 1 | See which computers are on (4 devices) | Yes | 0 (one look) | Good. "3 connected · 1 not reachable" and a dot on each tile. One case contradicts itself (finding 5). |
| 2 | Choose where files go (untick MacBook Air) | Yes | 1 click per device | Good. The sentence under the drop area updates live: "They go to Desk PC and Studio iMac". I couldn't check that the choice is remembered, because the simulation resets on reload. Settings says the shortcut and menu use the ticks. |
| 3 | Send files to two computers at once, kitchen laptop ticked but asleep | Yes | 3 (untick, Add, Send) | Good. One row per computer; the asleep laptop is skipped with a note (finding 7 on its wording). |
| 4 | Send something to a third, idle computer while two transfers run | **No** | none found | The drop area and Add buttons disappear while transfers run (finding 2). |
| 5 | Two computers offer files at the same time | Yes | 1 click each | Works, but the compact rows drop details and flip the button order (finding 6). |
| 6 | Follow transfers to different computers side by side | Yes | 0 to 1 (scroll) | The count in the heading is wrong, and the tiles don't show who is busy (findings 4, 9). |
| 7 | One paused (kitchen laptop asleep) while others run; then stop it | Partly | 2 (Cancel, Stop) | The paused row is clear; after Stop the row gets stuck (finding 3). |
| 8 | Send the clipboard to several computers | Yes | 1 | Works; the confirmation is tiny (finding 11). |
| 9 | Use the Shelf with several computers, turn on sync with one | Yes | 3 + scrolling | Dense. Clear wipes everything with no confirmation (finding 10). |
| 10 | Remove the kitchen laptop | Yes | 4 (Settings, scroll, Remove, Remove) | Clear and safe; the grid drops to 3 straight away. |
| 11 | Pair another (fifth) computer | **No** | 8 (Settings, scroll, Pair another, Connect by IP, type address + Connect, Pair, type code + Pair, OK) | Ended with only one device listed (finding 1). "Looking on this network…" never found anything in the simulation. |
| 12 | Window at its smallest (420×640) with six devices | Usable, cramped | n/a | The grid takes half the window; lists and the Shelf are mostly hidden (finding 8). |

## Findings

### 1. Pairing a fifth computer by IP replaced my four devices with one — **blocks use** (needs checking in the real app)
- **Screen:** `http://localhost:5180/?mock=online&os=mac&devices=4` → Settings → **Pair another computer** → **Connect by IP** → `192.168.1.40` → **Connect** → **Pair** → code `048213` → **OK**. Same on `os=win`.
- **Screenshot:** `shots/four-11-pair-ip-result.png`
- **What happened:** After **Connect**, the whole window turned into a one-computer screen: "Desk PC is nearby · Windows · Not paired yet". But Desk PC was already paired, and it was in my grid seconds earlier. The grid and the Shelf key disappeared. After pairing and **OK**, the window shows only "Desk PC · Connected", and Settings → Paired devices lists only Desk PC. MacBook Air, Studio iMac and Old Laptop are gone. On Windows the same flow showed "MacBook Pro is nearby · Not paired yet", also a computer that was already paired.
- **Why it matters to me:** Adding a computer must never cost me the others. If this is real, I'd have to re-pair three machines, one of them asleep in the kitchen. Even if it's only the simulator, being told an already-paired computer is "Not paired yet" would make me distrust the list.
- **Suggested fix:** Pair inside the "Pair another computer" panel and keep the device grid visible. If the address belongs to a computer that's already paired, say "That's Desk PC, already paired" and connect to it. After **OK**, return to the grid with five tiles and the new one ticked.

### 2. No way to start another send while transfers are running — **blocks use** (with buttons; dragging may still work)
- **Screen:** `?mock=many-sending&os=mac&devices=6`, `?mock=many-mixed&os=mac&devices=6`, and my own flow from `?mock=online&os=mac&devices=4` after **Send**.
- **Screenshot:** `shots/four-03-two-waiting.png`
- **What happened:** Once a send starts, the drop area, **Add files**, **Add a folder** and **Send clipboard** all disappear. The pane shows only the transfer rows with **Cancel**. The README says "any number with different computers at once", but nothing on screen lets me send to the MacBook Air while Desk PC and Studio iMac are busy. Ticking or unticking tiles does nothing visible. I couldn't test dragging from Finder in the browser, so dropping might still work, but nothing says so.
- **Why it matters to me:** The gaming PC receiving a 4 GB video shouldn't stop me sending a folder to the Mac Studio. Waiting for every transfer to finish defeats "side by side".
- **Suggested fix:** Keep a slim "Drop more here, or **Add files** / **Add a folder**" strip under the transfer list (as the Ready-to-send list already has), going to the ticked computers that aren't busy. If a ticked computer is busy, say "MacBook Air gets these after its current transfer" or "is busy, skipped".

### 3. Stopping the paused transfer leaves a stuck row — **confuses**
- **Screen:** `?mock=many-mixed&os=mac&devices=6`, then on "Paused: waiting for Old Laptop" press **Cancel**, then **Stop**.
- **Screenshot:** `shots/four-05-cancel-stuck.png`
- **What happened:** The title changes to "Cancelled: Old Laptop", but the question "Stop sending? The part that arrived on Old Laptop is removed." and the **Keep** / **Stop** buttons stay. Thirty seconds later they were still there. The same steps with only one transfer (`?mock=paused-out`) work properly: they go to a "Cancelled" screen with **OK** / **Send again**.
- **Why it matters to me:** I can't tell whether it stopped. Pressing **Keep** on a row that says "Cancelled" makes no sense.
- **Suggested fix:** After **Stop**, replace the question with "Cancelled. Nothing was kept on Old Laptop" and a dismiss ×, like the "Sent to Office PC" row.

### 4. "2 transfers under way" doesn't match what's on screen — **confuses**
- **Screen:** `?mock=many-mixed&os=mac&devices=6` (and `&os=win`), `?mock=many-offers&os=mac&devices=6`
- **Screenshot:** `shots/four-04-mixed-wide.png` (1280×900, so all four rows are visible)
- **What happened:** The heading says "2 transfers under way" above three active rows (sending to Desk PC, receiving from MacBook Air, paused for Old Laptop) and one finished row. With two incoming offers that I haven't answered, it also says "2 transfers under way", although nothing is moving. At the default 552×780 the fourth row ("Sent to Office PC") is below the fold with no scrollbar or hint.
- **Why it matters to me:** The heading is the first thing I read to know whether I can shut a machine down. A wrong number makes me count rows myself.
- **Suggested fix:** Count what's shown and name the kinds: "3 under way · 1 done", "2 computers want to send you files". Show a fade or "1 more ↓" when rows are hidden.

### 5. "Lost connection to Desk PC" while Desk PC's tile says it's connected — **confuses**
- **Screen:** `?mock=paused-out&os=mac&devices=4` (also `paused-in`)
- **Screenshot:** `shots/four-06-paused-tile.png`
- **What happened:** The pane says "Paused: lost connection to Desk PC … Is the PC asleep, or SeamDrop closed there?" Meanwhile the Desk PC tile shows a blue dot and "Wi-Fi", and the heading says "3 connected · 1 not reachable", which counts Desk PC as connected.
- **Why it matters to me:** "Which ones are on" is exactly what I use the grid for. When the tile and the transfer disagree, I walk to the other room to check.
- **Suggested fix:** When a transfer loses its computer, set that tile to "Not reachable" (orange) or "Reconnecting…" and update the count.

### 6. Two offers at once: less information, and the buttons swap places — **confuses**
- **Screen:** `?mock=many-offers&os=mac&devices=6`, compared with the single offer `?mock=offer-in&os=mac&devices=4`; also the paused-row cancel in `many-mixed` compared with `paused-out`
- **Screenshot:** `shots/four-07-two-offers.png`
- **What happened:** A single offer lists the items, says "Saved in Downloads/SeamDrop if you accept", offers "Always accept from Desk PC", and puts **Decline | Accept** with Accept on the right. With two offers, each row shows only "Site photos and 2 more · 633 MB" with **Accept | Decline**: Accept is on the left, and there's no file list, no save folder and no Always accept. Clicking the row doesn't expand it. Cancel confirmations swap the same way: **Keep | Stop** in a row, **Stop sending | Keep waiting** on the single screen. At 420×640 only the first offer is visible; the second is below the fold.
- **Why it matters to me:** Muscle memory from the single offer makes me hit the wrong button. Two computers sending at once is exactly when I'd want to tick "Always accept" for my own Mac Studio.
- **Suggested fix:** Keep the same button order everywhere. Let a row expand to the full offer, with its file list, save folder and Always accept. At the small size, say "2 offers" in the heading so I know to scroll.

### 7. "Doesn't get these now": will the kitchen laptop get them later? — **confuses**
- **Screen:** `?mock=online&os=mac&devices=4` (untick MacBook Air, press the drop area) and `?mock=online&os=mac&devices=6`
- **Screenshot:** `shots/four-02-ready-asleep.png`
- **What happened:** With the asleep Old Laptop still ticked, the list says "Old Laptop isn't connected, so it doesn't get these now." With six devices it says "Kitchen Mac mini and Old Laptop aren't connected…", although Kitchen Mac mini's tile says "Connecting…", not "Not reachable".
- **Why it matters to me:** "Now" suggests it might get them when it wakes. I need to know whether to come back and send again. A computer that's still connecting might be ready in two seconds.
- **Suggested fix:** Say it plainly: "Old Laptop is asleep or off; it won't get these. Send again when it's on." For one that's connecting, either wait a moment before sending or say "Kitchen Mac mini is still connecting; it isn't included."

### 8. At the smallest size with six devices, the grid takes over the window — **confuses**
- **Screen:** `?mock=many-mixed&os=win&devices=6` at 420×640; `?mock=online&os=mac&devices=6&clip=request` at 420×640; `?mock=online&os=win&devices=6` at 420×640 then press the drop area; Shelf from `?mock=online&os=mac&devices=6&clip=history` at 420×640
- **Screenshots:** `shots/four-12-small-transfers.png`, `shots/four-13-small-banner.png`, `shots/four-14-small-ready.png`, `shots/four-15-small-shelf.png`
- **What happened:** The six tiles take about 45% of the height. With four transfers only one row shows. In Ready to send, the full-width **Send** and **Clear** buttons cover the file list completely, so I press Send without seeing what I'm sending or the note about the asleep laptop. With a clipboard-sync request or an update banner, the grid is cut through the middle of the first row ("Desk PC" sliced in half). It does scroll, but nothing hints at that. The Shelf shows three lines of its explanation and the two buttons; the text box and the items are out of sight.
- **Why it matters to me:** The kitchen laptop has a small screen, and I keep SeamDrop small in a corner on the others. At that size I can't see what's going where.
- **Suggested fix:** At narrow heights, collapse the grid to a one-line strip ("6 devices · 4 on · ticked: 4 ▾") that opens the tiles on demand. Keep Send and Clear in a row below the list, not on top of it. Put banners below the grid, or make them one line with a "More" link.

### 9. The tiles don't show what each computer is doing — **annoys**
- **Screen:** `?mock=many-mixed&os=mac&devices=6`
- **Screenshot:** `shots/four-04-mixed-wide.png`
- **What happened:** During four transfers, the tiles still say only "Wi-Fi" or "Ethernet". To learn that MacBook Air is sending to me and Old Laptop is paused, I read the list, which may be scrolled out of view.
- **Why it matters to me:** The grid is the one thing that's always visible. It's where I look for "what's the gaming PC doing?".
- **Suggested fix:** Add a small second line or badge on the tile: "Sending 56%", "Receiving", "Paused", "Wants to send", "Busy".

### 10. Shelf: Clear wipes everything at once, and the panel is crowded — **annoys**
- **Screen:** `?mock=online&os=mac&devices=4&clip=history` → Shelf key
- **Screenshot:** `shots/four-08-shelf-scrolled.png`
- **What happened:** **Clear** sits right next to **Send clipboard** and removes every item from every computer at once, with no confirmation and no undo. Items live in memory only, so they're gone for good. It isn't clear whether it clears the text box or the items. When the panel scrolls, the close × floats over the text ("an ap[×]", "up to [×] MB"). The four sync switches sit at the very bottom, each followed by the same sentence: "Off. Both devices have to turn it on; turning it off on either one stops it." Turning one on adds a "Sync: waiting for MacBook Air" pill above the grid, which pushes everything down by about 50 px.
- **Why it matters to me:** One slip loses the thing I copied on the Mac Studio but haven't pasted yet. With four computers, the repeated sentences turn the sync section into a wall of text.
- **Suggested fix:** Name it **Clear Shelf** and move it away from the main button, or offer "Cleared · Undo" for a few seconds. Give the × a solid background or keep it out of the scrolling area. Say the sync rule once, above the switches, and give each switch a short status ("Off", "On here; waiting for MacBook Air", "On"). Reserve space for the sync pill so the grid doesn't jump.

### 11. The clipboard confirmation is easy to miss — **annoys**
- **Screen:** `?mock=online&os=mac&devices=4` → **Send clipboard**
- **Screenshot:** `shots/four-09-clip-sent.png`
- **What happened:** "Sent to Desk PC, Studio iMac and MacBook Air (text, 1.2 KB). Paste it there as usual." appears in small grey text in the footer, next to Details. It doesn't mention that ticked Old Laptop was skipped. The names are in a different order from the grid.
- **Why it matters to me:** I glance, see nothing change in the main area, and press it again.
- **Suggested fix:** Show a short toast near the button ("Sent to 3 · Old Laptop is asleep"), using the grid's order.

### 12. Settings with four computers is long, and "Pair another computer" is buried — **annoys**
- **Screen:** `?mock=online&os=mac&devices=4` → Settings
- **Screenshot:** `shots/four-10-settings-remove.png`
- **What happened:** Each paired computer gets a block of about 325 px with the same controls: Rename, Remove, Files from, Clipboard from, Sync, the same sentence again, and Disconnect. The blocks have no dividers, so where Studio iMac's settings end and Old Laptop's begin is hard to see. **Pair another computer** comes after all of them, about 1,900 px down with four computers and further with six. **Disconnect** appears even for the computer that is "Not reachable".
- **Why it matters to me:** Pairing a new machine is the main reason I'd open Settings, and I have to scroll past four copies of the same form to find it.
- **Suggested fix:** Put **Pair another computer** at the top of Paired devices. Show each computer as a one-line row (name, status, Files: Ask, Clipboard: Straight) that expands to its options. Hide Disconnect when it isn't connected.

### 13. Device icons follow the operating system, not the computer — **polish**
- **Screen:** `?mock=many-mixed&os=mac&devices=6`, `?mock=online&os=mac&devices=4`
- **Screenshots:** `shots/four-01-grid.png`, `shots/four-04-mixed-wide.png`
- **What happened:** Every Mac gets a laptop icon, including "Studio iMac" and "Kitchen Mac mini". Every Windows PC gets a desktop monitor, including "Old Laptop". In the wide window the tiles don't stretch, so "Kitchen Mac mini" wraps onto two lines while space is left at the right.
- **Why it matters to me:** My Mac Studio would show as a laptop and my kitchen laptop as a desktop. That's a small thing, but the icon is how I'd tell tiles apart at a glance.
- **Suggested fix:** Use the device's real kind when the OS reports it (laptop, desktop, mini), or let me pick it under Rename. Let tiles share the full width.

### 14. Small wording slips — **polish**
- **Screen:** `?mock=online&os=mac&devices=4` (Settings; and untick every tile with files added)
- **Screenshot:** `shots/four-02-ready-asleep.png` (the "Can't be sent" row)
- **What happened:**
  - Settings says "Your other computer shows this name", singular, with four paired.
  - With no tiles ticked at all, the list says "nothing ticked is connected".
  - "Shortcut to Notes.lnk · Can't be sent" doesn't say why.
- **Suggested fix:** "Your other computers show this name"; "Tick a computer to send to"; "Can't be sent: it's a shortcut, not a file".

## What worked well
- **The grid:** a tick per computer, "3 connected · 1 not reachable", and a coloured dot on each tile answer "which ones are on" at a glance. The sentence under the drop area names exactly who will get the files ("They go to Desk PC and Studio iMac (Old Laptop isn't connected)"), and it updates as I tick.
- **The asleep laptop doesn't get in the way.** It's skipped with a one-line note, not an error. If it's the only one ticked, **Send** and **Send clipboard** are disabled with "Nothing ticked is connected".
- **One Send, one row per computer.** "One at a time with each device; different devices go side by side" sets the right expectation.
- "Paused: waiting for Old Laptop" sits calmly among the running transfers, and the others carry on.
- **Cancel asks first and says what it removes**, both in a row and on the full screen.
- **Removing a device** explains the consequences ("Files already received stay. To use it again, pair again."), offers **Keep**, and the grid updates at once.
- Settings states that the shortcut and menu bar send to "the devices ticked in the window", so I know the ticks matter beyond this window.
- The single-transfer paused screen is very good: "Is the PC asleep, or SeamDrop closed there? It picks up when it wakes or opens."
- Dark mode and the Windows window look as finished as the Mac one.

## What I couldn't judge (probably the simulation)
- Simulated transfers never finish; they stay at "100% · A few seconds" with **Cancel**. My offers were never accepted on the other side, so I never saw "done" across several computers.
- Transfer rows show demo contents ("Site photos and 2 more · 633 MB", current file "IMG_0412.HEIC") instead of what I added (Floor plan.dwg and Résumé Q3.pdf, 21.1 MB). "Résumé Q3.pdf" turns into "Résumé_ Q3_.pdf" once it's sending.
- Reloading resets the ticks, so I couldn't check that "where files go" is remembered.
- In `many-mixed`, Details lists only Desk PC events and the times run backwards (14:08:30 before 14:08:22).
- `?mock=add-device&devices=4` and `&devices=6` show duplicate tiles (two "MacBook Air", two "Studio iMac"), with nothing to tell them apart. If two computers in a real house share a name, the tiles would look identical too; maybe show the OS or address in the tile's tooltip.
- The `many-*` states always show six devices, whatever `devices=` says.
- Dragging files from Finder or Explorer can't be simulated in the browser.
