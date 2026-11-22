# Review 05: the beginner

## Who I am

I'm about 50 and not technical. I have a MacBook Air in the study and a Windows laptop in the kitchen. I read every word on the screen, words like "IP" or "VPN" make me nervous, and when I'm not sure what to do I click the biggest button. All I knew going in was this sentence: "SeamDrop moves files and your clipboard between your computers, and keeps a shared Shelf." I also read the user part of the README, as a real user might.

I used the app window in a browser, at `http://localhost:5180/`, with simulated other computers. Most screens are at 552×780 and light. A few are at 420×640, in dark mode, or in the Windows window. Screenshots are in `shots/beginner-<n>.png`.

## What I tried

| Task | Finished? | Steps / clicks | Verdict |
|---|---|---|---|
| 1. Pair the two laptops, starting on the Mac (`?mock=nearby&os=mac`), and check what the PC shows (`?mock=pairing-incoming&os=win`) | Yes | Mac: **Pair**, click the box, type 6 digits, **Pair**, **OK**. PC: **Show code**. That's 5 clicks, 6 digits and two walks between rooms. | Works and feels safe. But the Mac told me the PC "shows six digits", and the PC actually showed a question first (finding 3). |
| 2. Send holiday photos Mac → PC, accept, find them later (`?mock=online&os=mac`, `?mock=offer-in&os=win&first=1`) | Yes, after a scare | Mac: **Add files**, **Send**. PC: **Accept**, **Show in File Explorer**. Finding them again later: **Settings**, then **Open**. That's 4 clicks to send and accept, plus 2 to find the photos later. | My first try looked stuck. One file that "Can't be sent" took over the screen and hid the "waiting for the PC to accept" message (finding 2). Without that file it was clear and pleasant. |
| 3. Shelf: work out what it is for, put something on it, get it on the other laptop (`?mock=online&os=mac`, `?mock=online&os=win&shelf=1`) | Partly | Mac: **Shelf**, type, **Add to Shelf**. That gave "Only on this computer." Then **Settings**, scroll, tick **Share the Shelf with Desk PC**, which gave "Waiting for Desk PC to agree." The simulated PC never agreed, so I couldn't see this step on the PC. With a Shelf that was already shared: PC **Shelf**, **Copy**, paste. | I could put things on it. Sharing it is hidden in Settings, and I never learned what to do on the other laptop (finding 1). The app describes the Shelf in different ways in different places (finding 4). |
| 4. The other laptop asleep or not answering (`offline`, `connecting`, `paused-out`, `waiting-there`, `resuming`, `stopped-in`) | Yes, I understood it | Looked at 8 screens and pressed **Try again** and **Stop trying**. | The transfer screens are reassuring ("It continues by itself"). The "not reachable" screen and "waiting for Desk PC to continue" left me unsure what to do (findings 10–12). |

## What worked well

- **The pairing code screens.** On the PC: "Type this code on MacBook Pro · It works once, and only on MacBook Pro." with big digits **048 213** (`?mock=pairing-show&os=win` after Show code, beginner-15; dark mode, beginner-89). The Mac's typing screen and "Paired with Desk PC · Files and the clipboard can go between them now." with one **OK** button (beginner-07, beginner-08). The code box was already selected, so I could type straight away.
- **Mistakes are explained kindly.** A wrong code gives "That's not it. Check the 6 digits on Desk PC: it's the same code. 1 try left, then you'll need a new one." (`?mock=pairing-wrong-here&os=mac`, beginner-18). When the code runs out: "The code expired after 2 minutes. Nothing was paired. Press Pair again for a new one." (beginner-94). When I press Ignore: "Ignored MacBook Pro (192.168.1.24). Nothing was paired." (beginner-96).
- **The incoming-files question on the PC** (`?mock=offer-in&os=win`, beginner-37). It says who is sending, what, how big, and where the files will go ("Saved in Downloads\SeamDrop if you accept."). **Accept** is the big button.
- **The "Received" screen** (`?mock=done-in&os=win&first=1`, beginner-95, beginner-42). "Received 3 items from MacBook Pro … every file checked", a big **Show in File Explorer** button, and on the first time "Received files go to Downloads\SeamDrop from now on." It also says in plain words why one name changed ("':' and '?' aren't allowed in Windows names") and why a link and a system file were left out.
- **Paused transfers** (`?mock=paused-out&os=mac`, beginner-73) say exactly what I'd wonder: "Is the PC asleep, or SeamDrop closed there? It continues by itself." Then comes "Desk PC is back" (beginner-75), and at the end "the 280 MB already on Desk PC wasn't sent again" (beginner-84).
- **Bad news always says my files are safe.** For example "Desk PC declined · Nothing was sent. Everything is still on this Mac." (`?mock=declined&os=mac`, beginner-81) and "Your files are still on this Mac" (`?mock=failed&os=mac`, beginner-83).
- **Cancel is safe if I click the big button.** The big dark button is **Keep sending** (`?mock=sending&os=mac` → Cancel, beginner-98).
- **Clipboard.** After **Send clipboard**: "Sent to Desk PC (text, 1.2 KB). Paste it there with Ctrl+V." (beginner-61). The PC asks before syncing: "MacBook Pro asks to sync the clipboard · Nothing syncs unless you turn it on." (`?mock=online&os=win&clip=request`, beginner-64a).
- **Shelf removal asks first.** It reads "Remove 'Office address' from the Shelf on your devices?" with **Keep** / **Remove** (beginner-58).
- **Details** reads like a diary: "Found MacBook Pro on this network", "Accepted here", "Every file checked ✓" (beginner-41).
- **Dark mode** reads as well as light (beginner-89 to beginner-92).

## Findings

### 1. Sharing the Shelf is hidden, and I never learned what to do on the other laptop. **blocks use**
- **What I saw:** I put "Hotel address" on the Shelf. Underneath it said **"Only on this computer."**, with no button to share. The one sentence I was given promised a *shared* Shelf. The help ("How the Shelf works") says only "You can share the Shelf with a paired computer when both say yes." It doesn't say where. I found it by luck in Settings → Paired devices → "Share the Shelf with Desk PC". Ticking it shows "Waiting for Desk PC to agree." That message stayed for over 20 seconds, and nothing told me what to press on the PC. Back on the Shelf, it still said "Only on this computer."
- **Where:** `?mock=online&os=mac` → Shelf → type → Add to Shelf → Settings → tick "Share the Shelf with Desk PC" → back to Shelf.
- **Screens:** beginner-47, beginner-50, beginner-52, beginner-54.
- **Simulation caveat:** the simulated PC never agreed, so I couldn't see what the PC shows when asked. A banner like the clipboard one (beginner-64a) may exist. I'm counting this because the Mac side gave me no path at all.
- **What would help:** a **Share with Desk PC** button right where it says "Only on this computer". Then a plain next step: "On Desk PC, press Share when SeamDrop asks."

### 2. One file that can't be sent hides my transfer, and the screen says the PC is "busy". **confuses**
- **What I saw:** I pressed **Add files**, and one of the files was "Shortcut to Notes.lnk · Can't be sent". I pressed **Send** anyway, as the big button invites. The screen became "**Busy sending to Desk PC.** 0 files · 0 B · next for Desk PC". Below that were "Desk PC is busy: these go next." and "Transfers under way carry on meanwhile.", and the big button was a greyed-out **Send next**. It stayed like that for 20 seconds. I thought it was stuck, or that someone else was using the PC. Only when I pressed **Clear** did the real screen appear: "Waiting for Desk PC to accept".
- **Where:** `?mock=online&os=mac` → Add files → Send (without removing the .lnk).
- **Screens:** beginner-25 (before), beginner-26 and beginner-30 (20 s later), beginner-105 (after Clear). If I removed the file first, I got the right screen straight away (beginner-32).
- **What would help:** when I press Send, leave the unsendable file out and show "Waiting for Desk PC to accept", with a small note: "Shortcut to Notes.lnk was left out (it's a shortcut)." Never call the other computer "busy" when it's my own transfer that's waiting.

### 3. The Mac says the PC "shows six digits", but the PC first asks a question. **confuses**
- **What I saw:** on the Mac, after **Pair**: "Type the code shown on Desk PC · Desk PC shows six digits. Type them here to pair." I walked to the kitchen expecting digits. The PC instead showed "MacBook Pro wants to pair · From 192.168.1.24. Show a code only if it's you pairing." with **Show code** / **Ignore**. Then a clock: "The request lapses in 1:59 if nobody answers." Meanwhile the Mac's clock was running too ("The code works for 1:56 more."). The help's Pairing section has the same gap.
- **Where:** `?mock=nearby&os=mac` → Pair; `?mock=pairing-incoming&os=win`.
- **Screens:** beginner-05, beginner-14, beginner-15.
- **What would help:** on the Mac: "On Desk PC, press **Show code**, then type its six digits here." "Lapses" is also not an everyday word: "The request goes away in 1:59" would be clearer.

### 4. The app tells three different stories about what the Shelf is. **confuses**
- **What I saw:**
  - The Shelf panel: "Things you keep, on your devices."
  - The help: "The Shelf keeps text you put there on purpose: keys, addresses, snippets you paste often. Nothing lands on it by itself."
  - The update banner: "the Shelf keeps what was sent to you".
  - The README: "The Shelf keeps what arrived".

  There's also a separate **Clipboard** panel with "Recently received", **Copy** and **Put on Shelf**. So I ended up with two places for things that arrived, and I couldn't tell which one I'm meant to use.
- **Where:** `?mock=online&os=mac` → Shelf; How the Shelf works; `?mock=online&os=mac&update=ready`; `?mock=online&os=win&clip=history` → Clipboard.
- **Screens:** beginner-47, beginner-48, beginner-102, beginner-62b.
- **What would help:** one sentence, used everywhere, with a home example, such as: "The Shelf is a noticeboard both computers can see: pin an address or a booking number here and copy it on the other one."

### 5. The two icons at the top have no labels, and the README points to the wrong one. **confuses**
- **What I saw:** after pairing, three small icon buttons appear at the top right: a clipboard, a box and a gear (beginner-11). Hovering shows a tooltip ("Clipboard: send it, sync it, what arrived", "Shelf: things you keep"), but I had to guess before that. The README says to press **Send clipboard** "on the **Shelf** (the clipboard key next to the gear)". In the app, the key next to the gear is the box (Shelf), and Send clipboard is under the separate clipboard key (beginner-60). Before pairing, only the box and the gear are there (beginner-01).
- **Where:** `?mock=online&os=mac`, top right.
- **Screens:** beginner-11, beginner-60.
- **What would help:** short words under or beside the icons ("Clipboard", "Shelf", "Settings"), and a README that matches the app.

### 6. The first screen's only button is "Connect by IP". **confuses**
- **What I saw:** "Looking for your other computer · Open SeamDrop on your other computer, on this network." The one button is **Connect by IP**, so I pressed it. That took me to "The other computer's IP address · for example 192.168.1.20". I don't know what that is, and it isn't what I needed: I just had to open SeamDrop on the PC.
- **Where:** `?mock=searching&os=mac` → Connect by IP.
- **Screens:** beginner-01, beginner-101.
- **What would help:** make the main thing the instruction ("Now open SeamDrop on your other computer"). Turn Connect by IP into a small "Still not showing up?" link, or show it only on the "Still looking" screen.

### 7. "SeamDrop is open on the other computer" reads like a fact. **confuses**
- **What I saw:** in the checklist, next to a small "?" icon, it says "**SeamDrop is open on the other computer** · Open SeamDrop there too…". The other lines are ticked facts ("On Wi-Fi", "No VPN in the way"), so I read this one as "good, it's open". It actually means the app isn't sure.
- **Where:** `?mock=searching-long&os=mac`, `?mock=offline&os=mac`.
- **Screens:** beginner-03, beginner-65.
- **What would help:** phrase it as a question or a task: "Is SeamDrop open on Desk PC? Open it there too."

### 8. The pairing-lock warning appears twice and is frightening. **confuses**
- **What I saw:** the same box appears twice on one screen, once at the top and once inside "Computers nearby": "**Pairing is locked after three wrong codes.** Last from Stranger's laptop (192.168.1.57) at 18:26." with **Allow pairing again**. A stranger was trying to get into my computer? Am I safe? Should I press the button, or is that what the stranger wants?
- **Where:** `?mock=pairing-locked&os=mac`.
- **Screens:** beginner-19.
- **What would help:** show it once, and say what happened and what to do: "Someone on your network typed 3 wrong codes, so nothing was paired and pairing is locked. Press Allow pairing again only when you're pairing your own computer."

### 9. "Stop sending? The part on Desk PC is removed." scared me about my photos. **confuses**
- **What I saw:** with holiday photos, "removed" made me wonder whether pressing it would delete them from my Mac. The receiving side says "Stop receiving? What arrived is removed." The README promises "Your files on the sending computer are never touched". The window should say that too.
- **Where:** `?mock=sending&os=mac` → Cancel; `?mock=receiving&os=win` → Cancel.
- **Screens:** beginner-98, beginner-99.
- **What would help:** "Stop sending? Your photos on this Mac stay as they are; the half-copied ones on Desk PC are deleted."

### 10. "Paused: waiting for Desk PC to continue": continue what? **confuses**
- **What I saw:** the Mac says only "Paused: waiting for Desk PC to continue" (beginner-74). The PC meanwhile says "Receiving Site photos and 2 more from MacBook Pro stopped partway." with **Continue** / **Cancel**, and no reason why it stopped (beginner-80). Nothing on the Mac told me to go and press Continue on the PC.
- **Where:** `?mock=waiting-there&os=mac`, `?mock=stopped-in&os=win`.
- **Screens:** beginner-74, beginner-80.
- **What would help:** on the Mac: "On Desk PC, press **Continue**." On the PC, a reason, for example "SeamDrop was closed during the transfer."

### 11. On the "not reachable" screen, "Try again" seems to do nothing, and "Stop trying" sounds final. **confuses**
- **What I saw:** "Desk PC isn't reachable · Is Desk PC on and awake, with SeamDrop open?" That question is good. I pressed the big **Try again** and nothing on the screen changed, not even for a moment. The screen never says SeamDrop keeps trying by itself. **Stop trying** sounded like it would unpair. It actually leads to "Disconnected from Desk PC · Still paired. Press Connect to use it again."
- **Where:** `?mock=offline&os=mac` → Try again; → Stop trying.
- **Screens:** beginner-65, beginner-68 (just after Try again), beginner-69 (5 s later), beginner-70.
- **Simulation caveat:** the real app may show a retry. Even so, the screen should say it.
- **What would help:** after Try again: "Tried again just now: still no answer." A line saying "SeamDrop connects by itself as soon as Desk PC wakes up." And a gentler name for Stop trying, such as "Stop looking for now".

### 12. While the PC is asleep, the Shelf says "Shared with Desk PC" and doesn't say whether new things will reach it. **confuses**
- **What I saw:** with Desk PC "Not reachable", I added "Flight AZ 1234, Tuesday 9:40". It said "Kept on the Shelf." with "Shared with Desk PC." underneath. Will the flight number be there when the PC wakes up? Nothing says.
- **Where:** `?mock=offline&os=mac&shelf=1` → Shelf → Add to Shelf.
- **Screens:** beginner-103, beginner-104.
- **What would help:** for example "Shared with Desk PC (asleep): it gets this when it wakes up", or a small "waiting" mark on the new item.

### 13. "How the Shelf works" opens the help at Pairing. **annoys**
- **What I saw:** I pressed "How the Shelf works" and got "Help" starting with "Pairing". The Shelf section is about three screens further down. I had to scroll and search for it.
- **Where:** `?mock=online&os=mac` → Shelf → How the Shelf works.
- **Screens:** beginner-48.
- **What would help:** open the help at "The Shelf". The same probably applies to "How the clipboard works".

### 14. After "Done", there's no way back to the photos from the main window. **annoys**
- **What I saw:** once I pressed **Done** on "Received 3 items", the window went back to "Drop files here" (beginner-40). It shows no "last received" line and no folder button. I only found the photos again through **Settings → Received files → Open** (beginner-43). Details does say "Saved 3 items in Downloads/SeamDrop", but I can't click it, and it uses a "/" where every other Windows screen uses "\" (beginner-41).
- **Where:** `?mock=done-in&os=win` → Done.
- **Screens:** beginner-38, beginner-40, beginner-41, beginner-43.
- **What would help:** a small "Last received: 3 items from MacBook Pro · **Show**" line on the main screen, or a Show button in Details.

### 15. I can't line up photos to go when the other laptop wakes up. **annoys**
- **What I saw:** when Desk PC isn't reachable, the whole window becomes the checklist. There's no **Add files** and nowhere to drop files (beginner-65). The help says a *busy* computer "gets your files next", but a sleeping one doesn't. I would have liked to set the photos going and walk away.
- **Where:** `?mock=offline&os=mac`.
- **Screens:** beginner-65.
- **What would help:** keep the drop area. Say "Desk PC is asleep. These go when it wakes up." and keep the checklist one click away.

### 16. "Ran out of space" offers "Send again", which would just fail again. **annoys**
- **What I saw:** "Transfer stopped · Desk PC ran out of space." The big button is **Send again**. Nothing says how much room is needed or that I need to make space on the PC first.
- **Where:** `?mock=failed&os=mac`.
- **Screens:** beginner-83.
- **What would help:** "Desk PC needs about 633 MB free. Make room there, then press Send again."

### 17. Too many technical words and numbers on everyday screens. **polish**
- **What I saw:**
  - An address under each computer: "Windows · 192.168.1.31" (beginner-04, beginner-24), "From 192.168.1.24" (beginner-14), "No answer from 192.168.1.31." (beginner-65).
  - "No VPN in the way" (beginner-03).
  - "64 MB/s over Wi-Fi" and the file path "Site photos/IMG_0466.HEIC" in a computer-code font (beginner-acc-0).
  - "633 MB: 353 MB sent in 7.4 s (48 MB/s)" (beginner-84).
  - "(text, 1.2 KB)" (beginner-61).
  - The Mac shortcut shown as symbols, "⌃⌥⌘C" (beginner-60).
- **What would help:** keep names and plain words up front ("About 10 seconds left", "Sent 3 items"). Put addresses, speeds and file paths in Details.

### 18. Some things look pressable but aren't, and some that are switched off look switched on. **polish**
- **What I saw:**
  - The "Wi-Fi" pill at the top right looks like a button, but clicking it does nothing (beginner-11).
  - On a paused transfer, the round pause symbol next to "44%" looks like a button. I clicked it, hoping to resume, and nothing happened (beginner-73, beginner-74).
  - The greyed-out **Pair** and **Connect** buttons are still nearly black, so they look ready to press (beginner-05, beginner-101).
- **What would help:** make the pill and the symbol look like plain labels, and make switched-off buttons clearly pale.

### 19. The Shelf's example name is "Vercel key". **polish**
- **What I saw:** the empty name box on the Shelf shows the example "Vercel key" (beginner-47). I don't know what a Vercel is. It made the Shelf feel like it's for programmers.
- **Where:** `?mock=online&os=mac` → Shelf.
- **What would help:** a home example such as "Wi-Fi password" or "Hotel address".

### 20. Small wording and layout snags. **polish**
- "Add folder" on the empty screen, but "Add a folder" once files are listed (beginner-11, beginner-25).
- "Shortcut to Notes.lnk can't be sent (a link, or unreadable)": which one is it? (beginner-25)
- "Checking the 280 MB already here" also appears on the *sending* Mac, where the files aren't "here" (beginner-75).
- "Copied. Paste it anywhere." shows in small print at the bottom left, far from the Copy button I pressed (beginner-57).
- In the small window (420×640):
  - The Shelf line "from 🖥 this computer" breaks after "from" (beginner-107).
  - "Shared with MacBook Pro." is scrolled out of sight.
  - On "Received", the notes about the renamed and left-out files sit under the buttons (beginner-86).
- Settings shows "Website: seamdrop.com (not live yet)" and "Feedback by email isn't set up yet." These made me wonder whether the app is finished (beginner-45).

## Things I didn't count (they come from the simulation)

- The sample files I added on the Mac ("Floor plan.dwg", "Résumé Q3.pdf", the .lnk) aren't the ones the waiting and receiving screens show ("Site photos · 212 files" and so on). The computer names "MacBook Pro" and "Desk PC" aren't my laptops' names.
- The simulated PC never accepted my files and never agreed to share the Shelf when I started from the Mac. I looked at the PC's screens through their own states instead.
- **Show in File Explorer** and folder pickers can't open. Hover tooltips exist on the top icons, but screenshots can't capture them.
- On the "Waiting for Desk PC to accept" screen, "Asked Desk PC just now" still said "just now" after 70 seconds. That may be the simulation.

## Count

- **blocks use:** 1 (finding 1)
- **confuses:** 11 (findings 2–12)
- **annoys:** 4 (findings 13–16)
- **polish:** 4 (findings 17–20)
