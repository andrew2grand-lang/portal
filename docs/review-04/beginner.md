# Review 04: the beginner

**Who I am.** I'm 52. I use a MacBook Air for email and photos, and the family has a Windows PC in the study, in a different room. The only thing I've ever "paired" is Bluetooth headphones. I read every word on the screen, technical words make me nervous, and when I'm unsure I click the biggest button. I want two things: get the holiday photos from the Mac onto the PC, and paste a shopping list I copied on the PC into a message on the Mac. I looked at the screens in the browser simulation (Mac and Windows windows, 552×780 and the smallest size, 420×640) and read the user part of the README. Nothing else.

## What I tried

| Task | Finished? | Steps | Verdict |
|---|---|---|---|
| Pair for the first time, starting on the Mac (the side that asks) | Yes | Mac: **Pair** → walk to the PC → **Show code** → walk back → type 6 digits → **Pair** → **OK** (4 clicks plus typing) | Works, and it feels safe. The Mac never actually tells me to go to the PC and press Show code (finding 2). |
| Pairing on the PC (the side that shows the code) | Yes | **Show code** (1 click), then **OK** when it says paired | Very clear. The big code and "works once, for the next 2 minutes" are just right. The IP number in the headline is the only wobble. |
| Typing a wrong code | Yes | 3 wrong tries → "Not paired" → **OK** → **Pair** again | It recovers safely ("Nothing was paired"). The wrong-code message is easy to miss, and the dead end has no "Pair again" button (finding 9). |
| Choosing where files go | Partly (the folder picker isn't simulated) | Gear → "Received files go to" → **Change…**; also **Change folder…** on the first "Received" screen | Easy to find, especially on the first-received screen. |
| Sending holiday photos Mac → PC | Yes, in the simulation | Click the big circle (I had to guess that) → **Send** → PC: **Accept** → **Show in File Explorer** | Choosing the photos took guessing (finding 1), and at the end I couldn't tell whether it had finished (finding 5). The PC side is lovely. |
| The other computer not answering | Understood, after a fashion | Read the "isn't reachable" and "Paused" screens | The Paused screen says exactly what I'd want to hear. The "isn't reachable" screen talks about permissions, IP numbers and VPNs before "is the PC asleep?" (finding 3). |
| Shopping list PC → Mac (the Shelf) | Yes | PC: copy, then **Send clipboard** (1 click) → Mac: ⌘V | Fast once I knew how. Feedback is tiny, and the Shelf confused me when I opened it on the Mac to look for my list (findings 6–8). |
| The Details button | Yes | 1 click | The main lines are mostly plain English, but there are technical grey lines under each. Once it contradicted the screen (finding 10). |
| Window made small (420×640) | Yes | Same tasks | The main buttons stay big and reachable. The Shelf's list and the "isn't reachable" advice disappear (findings 3, 8). |

## What worked well

- **Showing the code on the PC** (`?mock=pairing-show&os=win`): the six big digits, "Type this code on MacBook Pro", and "It works once, for the next 2 minutes, and only on that computer". I knew exactly what to do.
- **The PC asks first** (`?mock=pairing-incoming&os=win`): "Show a code only if you're pairing this computer with MacBook Pro yourself" made me feel safe and in charge.
- **Typing the code**: the box was already selected, so I could just type. It didn't mind the space in "048 213", and Enter worked.
- **After three wrong codes**: "Nothing was paired. Pair again for a new code." I wasn't scared I'd broken anything.
- **Receiving for the first time** (`?mock=done-in&os=win&first=1`): a big **Show in File Explorer**, plus **Change folder…** with "what's already here stays". This is the best screen in the app for me.
- **Accepting files** (`?mock=offer-in&os=win`): it tells me where they'll go before I say yes.
- **Cancel asks first** (`?mock=sending&os=mac`): it says what happens, and the dark button is **Keep sending**. Clicking the biggest button is the safe thing.
- **Paused** (`?mock=paused-out&os=mac`): "Is the PC asleep, or SeamDrop closed there?" and "Your files are safe on this Mac". This is how every problem screen should talk.
- **Remove a paired computer** asks first and explains itself in plain words.
- **Private clipboard** says *why* nothing was sent and what to do instead.

## Findings

### 1. With nothing added yet, there's no "Add files" button, only "Add a folder"
- **Screen:** `http://localhost:5180/?mock=online&os=mac` (Windows is the same)
- **Screenshot:** `shots/beginner-online-empty-mac.png`
- **What happened:** The main screen says "Drop files or folders" with two small buttons: **Add a folder** and **Send clipboard**. I don't know how to "drop", and my holiday photos are in the Photos app, not a folder. Clicking the big circle turned out to open the file chooser, but nothing on screen says so. **Add files** only appears *after* something is already in the list. The README promises "**Add files** / **Add a folder**". In the small window the circle picture disappears too, so it's only a heading.
- **Why it matters to me:** This is the very first thing I want to do. I'd probably click **Add a folder** and get lost in Finder looking for a folder of photos.
- **Suggested fix:** In the empty state, make **Choose files…** (or "Add photos or files…") the big dark button, with **Add a folder** next to it. Add one line: "Or drag photos straight from the Photos app onto this window."
- **Severity:** **confuses**

### 2. The pairing hand-off between rooms isn't spelled out
- **Screen:** `http://localhost:5180/?mock=nearby&os=mac` → **Pair**
- **Screenshot:** `shots/beginner-pairing-asking-mac.png`
- **What happened:** After **Pair**, the Mac says "Desk PC asks whoever is there first. When they agree, it shows a code to type here." I'm alone; nobody is "there", and "they" sounds like someone else. Before that, the nearby screen says "You'll check a code on both screens", but that's not what happens: one computer shows a code and I type it on the other. Nothing says how long the Mac will wait, and I couldn't see what it says if nobody answers on the PC.
- **Why it matters to me:** The PC is in another room. I need to be told, in order: go to the PC, press **Show code**, come back, type it here. Otherwise I'd sit staring at "Waiting for Desk PC…".
- **Suggested fix:** "Now go to Desk PC. It's asking whether to pair: press **Show code** there, then come back and type the 6 digits here." On the nearby screen: "Desk PC will show a code; you type it here." If no one answers for a while, say "Desk PC hasn't answered. Is SeamDrop open there?"
- **Severity:** **confuses**

### 3. "Desk PC isn't reachable" leads with permissions, numbers and VPNs, not "is the PC asleep?"
- **Screen:** `http://localhost:5180/?mock=offline&os=mac` (and at 420×640)
- **Screenshots:** `shots/beginner-offline-mac.png`, `shots/beginner-small-offline-mac.png`
- **What happened:** The first lines are "No answer from 192.168.1.31", then "Likely cause: SeamDrop may need Local Network access on this Mac… use Connect by IP and type this Mac's address: 192.168.1.24", then a warning about guest Wi-Fi. "SeamDrop is open on the other computer" comes third, marked "Not sure". Nothing asks whether Desk PC is turned on or asleep, which is by far the likeliest thing in our house. It worked yesterday, so a Mac permission is the least likely cause. In the small window only that first, scariest item is visible, half-covered by three stacked buttons. "Connect by IP" is also the *only* button on the very first "Looking for your other computer" screen, before the app has even had a few seconds.
- **Why it matters to me:** Warning triangles and numbers make me think something is broken on *my* Mac, and I'd start changing System Settings for nothing. The Paused screen (`?mock=paused-out`) already says it perfectly: "Is the PC asleep, or SeamDrop closed there?"
- **Suggested fix:** Start with "Is Desk PC turned on and awake, with SeamDrop open?" Only show the Local Network item first when the app knows that permission is actually off, and keep the IP number out of the headline sentence. In the small window, keep that first plain question visible above the buttons. Show **Connect by IP** only on "Still looking", not in the first seconds. Suggesting **Open at login** during pairing would also stop many of these (it is off in Settings).
- **Severity:** **confuses**

### 4. "Connected" at the top, "lost connection" in the middle
- **Screen:** `http://localhost:5180/?mock=paused-out&os=mac`
- **Screenshot:** `shots/beginner-paused-out-mac.png`
- **What happened:** Under the computer's name it says "Windows · Connected", with a "Wi-Fi" badge. The card right below says "Paused: lost connection to Desk PC". Next to 44% there's a round pause symbol that looks like a button but does nothing.
- **Why it matters to me:** Which one is true? If it's connected, why is it paused? I'd try clicking the pause symbol.
- **Suggested fix:** Make the top line agree ("Windows · Not connected" or "Reconnecting…", without the Wi-Fi badge), and drop the button look from the pause symbol.
- **Severity:** **confuses**

### 5. 100%, all ticked, but "A few seconds" and a Cancel button stay on screen
- **Screen:** `http://localhost:5180/?mock=sending&os=mac` (receiving on Windows did the same after I pressed Accept on `?mock=offer-in&os=win`)
- **Screenshot:** `shots/beginner-sending-100-mac.png`
- **What happened:** The bar reached 100% and every item got a tick, but it still said "Sending to Desk PC", "A few seconds", and offered **Cancel**. It stayed like that for 20 seconds and never turned into the "Sent" screen. This may be the simulation; I'm reporting what I saw.
- **Why it matters to me:** I don't know whether it's finished. **Cancel** is the only button, and I'm afraid pressing it would undo everything.
- **Suggested fix:** At 100% switch straight to "Sent" / "Received". If the app needs a moment to check the files, say "Checking the files on Desk PC…", hide the time estimate, and don't offer Cancel as the only button.
- **Severity:** **confuses**

### 6. Clipboard results are tiny grey words at the bottom, and success and failure look alike
- **Screen:** `http://localhost:5180/?mock=online&os=mac&clip=private` → **Send clipboard**
- **Screenshot:** `shots/beginner-clip-private-mac.png`
- **What happened:** Nothing changes in the middle of the window. A small grey note appears next to **Details**. When it worked, it said "Sent to MacBook Pro (text, 1.2 KB). Paste it there with ⌘V." When it didn't, it said "The app you copied this from marked it as private, so it wasn't sent. To send it anyway, paste it into the Shelf's text box." Both are the same size and colour. "The Shelf" is an unlabelled clipboard icon at the top right, so I didn't know what it meant. "1.2 KB" means nothing to me.
- **Why it matters to me:** I'd walk to the Mac and press ⌘V either way. If it wasn't sent, I'd paste the wrong thing and not know why.
- **Suggested fix:** Show the result as a clear message in the middle of the window for a few seconds, with a tick or a warning sign, and quote the start of the text ("'Milk, eggs, bread…' is ready to paste on MacBook Pro"). Make "the Shelf" a link that opens it. Drop the KB.
- **Severity:** **confuses**

### 7. The Shelf mixes sending and receiving, and the biggest button sends
- **Screen:** `http://localhost:5180/?mock=online&os=mac&clip=history` → Shelf (clipboard icon, top right)
- **Screenshot:** `shots/beginner-shelf-mac.png`
- **What happened:** I went to the Mac to *get* my shopping list. The main window showed no sign that anything had arrived, only "Shelf · 4" on a small button. In the Shelf, the first thing is a dense paragraph ("…unless it came from a device set to Shelf only… Kept in memory only, never on disk…"), then a box to *send* text, then the list of what arrived. The big dark button at the bottom is **Send clipboard**, which sends the Mac's clipboard the other way. Nothing says which item is on my clipboard right now. Labels like "rich text, 860 bytes" and "Not kept (too big)" don't tell me where that big picture went.
- **Why it matters to me:** I'd press the big button, thinking it fetches my list, and send something back to the PC instead. "Memory" and "disk" make me worry my things will disappear.
- **Suggested fix:** Put "Arrived here" first, mark the top item "On your clipboard now: press ⌘V to paste", and move sending (the text box and **Send clipboard**) below it as a secondary section. Shorten the intro to one line ("Things sent to this computer. They're gone when SeamDrop quits."). For a too-big item, say where it went ("Saved as a file in Downloads/SeamDrop"). Briefly show "Shopping list arrived from Desk PC" in the main window when something comes in.
- **Severity:** **confuses**

### 8. In the small window, the Shelf hides its own list, and Clear doesn't ask
- **Screen:** `http://localhost:5180/?mock=online&os=mac&clip=history` at 420×640 → Shelf
- **Screenshot:** `shots/beginner-small-shelf-mac.png`
- **What happened:** All I see is the paragraph, the send box, and two full-width buttons, **Clear** and **Send clipboard**. The four things that arrived are below, out of sight, with no hint that I can scroll. When I pressed **Clear** (at normal size), everything went at once, with no question and no undo. Then it said "Nothing has arrived yet.", which isn't true.
- **Why it matters to me:** The list I came for isn't visible, and the big **Clear** button right there can throw it away with one click.
- **Suggested fix:** At small sizes, show the arrived items first and make the bottom buttons smaller (not full width). Make **Clear** ask ("Clear 4 items from the Shelf?") or offer **Undo**. After clearing, say "Shelf cleared."
- **Severity:** **confuses**

### 9. A wrong code gets a quiet message, and the dead end has no way forward
- **Screen:** `http://localhost:5180/?mock=pairing-wrong-here&os=mac`; after three wrong tries from `?mock=nearby&os=mac`
- **Screenshot:** `shots/beginner-pairing-wrong-here-mac.png`
- **What happened:** The box goes empty with an orange border. The grey line under the heading quietly changes to "That wasn't the code shown on Desk PC. 1 try left." It doesn't tell me the PC's code is still the same, or what happens after the last try. After the third wrong code, the "Not paired" screen has only **OK**, and the line under the name still says "Pairing". The empty box shows "000 000" in large dark digits, which I could take for a code, and the disabled **Pair** button looks almost the same as the working one.
- **Why it matters to me:** "1 try left" sounds like I'm about to be locked out of something. After the third try I have to work out for myself to press OK and then Pair again.
- **Suggested fix:** Show the wrong-code line in colour, near the box: "That's not it. Check the 6 digits on Desk PC; it's the same code. 1 try left, then you'll get a new one." On "Not paired", make the main button **Pair again**. Use a lighter placeholder (or dashes), and make the disabled button clearly greyed out.
- **Severity:** **annoys**

### 10. Details has technical lines under each entry, and once it contradicted the screen
- **Screen:** `http://localhost:5180/?mock=offline&os=mac` → **Details**
- **Screenshot:** `shots/beginner-details-offline-mac.png`
- **What happened:** The main lines are nice and plain ("Found Desk PC on this network"). Each one has a grey line underneath, though, like "port 47920, protocol 3", "192.168.1.31:47920 via mDNS", "dialled", "1 MiB chunks", "disk_full". The button's icon is a ">_" symbol that looks like something for programmers. On the "isn't reachable" screen, the last line in Details was "Connected to Desk PC over Wi-Fi", with nothing about losing it. My pairing and my wrong code weren't listed either. (This may be the simulation's fixed log.) **Send feedback** didn't send anything; it said email isn't set up and saved a file in Downloads.
- **Why it matters to me:** The README promised "plain words". The grey lines make it look like a log for experts, and a "Connected" line while the screen says "not reachable" makes me trust neither.
- **Suggested fix:** Keep the technical lines for **Copy technical log** only, or show them behind a "Show technical details" switch. Make sure losing the connection, pairing and wrong codes appear as entries. Use a plain icon (a list or an "i"). Until email works, call the button "Save a report for support" instead of **Send feedback**.
- **Severity:** **confuses**

### 11. "Ran out of space": the big button just tries again
- **Screen:** `http://localhost:5180/?mock=failed&os=mac`
- **Screenshot:** `shots/beginner-failed-mac.png`
- **What happened:** "Transfer stopped. Desk PC ran out of space." The dark button is **Send again**. The screen doesn't say how much room is needed or what to do. Details does say "Free up space there and send it again", but I'd never have looked there.
- **Why it matters to me:** I'd press **Send again**, it would fail again, and I'd decide the app is broken.
- **Suggested fix:** "Desk PC needs about 633 MB more space. Delete some files on Desk PC, or choose a folder on another drive in SeamDrop's settings there, then press Send again."
- **Severity:** **annoys**

### 12. In Settings, the round Close button sits on top of "Remove"
- **Screen:** `http://localhost:5180/?mock=online&os=mac` → gear → scroll down to "Paired devices"
- **Screenshot:** `shots/beginner-settings-overlap-mac.png`
- **What happened:** Once I scroll, the floating **×** stays at the top right and overlaps the top of the **Remove** button for Desk PC. Clicking just below the × hits **Remove**. (It does ask first, thankfully.) **Rename**, **Remove** and **Disconnect** all look equally important, and I can't tell the difference between Disconnect and Remove.
- **Why it matters to me:** I'd be one slip away from un-pairing the computers I just worked hard to pair.
- **Suggested fix:** Give the Settings panel a fixed header row for the ×, so content scrolls underneath it, not beside it. Make **Remove** look less important than **Rename** (a text button, red only in its confirmation). Add a short hint under Disconnect ("stops connecting until you press Connect; stays paired").
- **Severity:** **annoys**

### 13. Numbers and network words show up where I don't need them
- **Screens:** `http://localhost:5180/?mock=pairing-incoming&os=win` ("MacBook Pro (192.168.1.24) wants to pair"), plus the offline, Details and clipboard notes above
- **Screenshot:** `shots/beginner-pairing-incoming-win.png`
- **What happened:** The PC's question puts the Mac's IP number in the big headline. Elsewhere I met "IP", "VPN", "Local Network", "mDNS", "KB", "bytes" and "Skip system files like .DS_Store and Thumbs.db" (in the first section of Settings).
- **Why it matters to me:** Each number or code word makes me think I'm in the wrong place or doing something risky. A number in a headline about "wants to pair" even looks a little suspicious.
- **Suggested fix:** Keep the headline to "MacBook Pro wants to pair", with the address in small text if it's needed for safety. Rename "Skip system files like .DS_Store and Thumbs.db" to "Skip hidden system files" (put the names in a tooltip), and move it below Appearance.
- **Severity:** **polish**

### 14. "1 name changed" and "2 items left out": which ones?
- **Screen:** `http://localhost:5180/?mock=done-in&os=win&first=1`
- **Screenshot:** `shots/beginner-done-in-win.png`
- **What happened:** Under the received list: "1 name changed so it works on Windows." and "2 items left out on MacBook Pro: links and system files aren't sent." Neither says which. The list shows "Résumé_ Q3_.pdf", so I guess that's the renamed one. The photos in the list are "IMG_0412.HEIC", and nothing warns me that the PC might not open them without an extra from Microsoft.
- **Why it matters to me:** "Left out" makes me worry that some holiday photos didn't come.
- **Suggested fix:** Make each note expandable to name the items ("Résumé: Q3?.pdf became Résumé_ Q3_.pdf"). Say "no photos were left out" when that's true. If HEIC photos arrive on Windows, add one line: "If these photos don't open, install HEIF Image Extensions from the Microsoft Store."
- **Severity:** **polish**

### 15. Smaller wording slips
- **Screen:** `http://localhost:5180/?mock=pairing-paired&os=mac` and `?mock=pairing-show&os=win`
- **Screenshot:** none saved (see `shots/beginner-pairing-asking-mac.png` for the same header)
- **What happened:** The "Paired with Desk PC" and "Not paired" screens still say "Windows · Pairing" under the name. The PC's code screen says "for the next 2 minutes" with no countdown; in the simulation it still said that after 2 minutes and 15 seconds. Sending to the Mac from the PC's main screen says "Paste it there with ⌘V", but from the Shelf's text box it says "Paste it there as usual".
- **Why it matters to me:** Small, but I read every word, and things that don't match make me double-check.
- **Suggested fix:** Change the subtitle to "Paired" / "Not paired". Show a gentle countdown ("1:20 left"), and say "This code has expired: press Pair again on MacBook Pro" when it runs out. Use the same paste wording everywhere.
- **Severity:** **polish**

## Notes for the team (probably the simulation, not the app)

- **The pairing check in the simulation is loose.** From `?mock=nearby&os=mac`, the simulated PC accepted **084 213** (two digits swapped) and **333 333** as well as 048 213; 111 111, 048 214, 048 212 and 999 999 were rejected. Every accepted code's digits add up to 18, so I assume the simulator checks something simpler than the real code. Please confirm the real app needs the exact six digits.
- After I added files and pressed **Send**, the "Waiting to accept" list showed "Site photos" (212 files), which I hadn't added. It looks like the simulator jumps to its prepared "offer-out" state.
- In Details for `?mock=failed`, the times go 14:13:37, then 14:13:30, which isn't "newest last". This is probably the fixed simulated log.

## Summary by severity

- **Blocks use:** none. I finished everything I tried.
- **Confuses:**
  - No "Add files" button until something is already added (1)
  - The pairing hand-off between rooms isn't told (2)
  - The "isn't reachable" screen is technical and leads with the least likely cause (3)
  - "Connected" next to "lost connection" (4)
  - Stuck at 100% with Cancel showing (5)
  - Clipboard feedback is tiny and looks the same for success and failure (6)
  - The Shelf mixes send and receive, and its biggest button sends (7)
  - The small Shelf hides its list, and Clear doesn't ask (8)
  - Details shows technical lines and contradicted the screen (10)
- **Annoys:**
  - The wrong-code message is quiet, with no "Pair again" button (9)
  - "Out of space" offers only Send again (11)
  - Settings' × overlaps Remove (12)
- **Polish:**
  - IP numbers and jargon (13)
  - Changed or left-out items aren't named, and nothing about HEIC photos (14)
  - Subtitle, countdown and paste-wording slips (15)
