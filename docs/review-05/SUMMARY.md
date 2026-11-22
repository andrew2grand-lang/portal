# Review 05: summary

Three reviewers looked at SeamDrop with fresh eyes (Mission 05 §11). Each was told only "SeamDrop moves files and your clipboard between your computers, and keeps a shared Shelf." and could read the user part of the README. None saw the mission, the plan or the decisions. They used the real screens in a browser, against simulated other computers (`http://localhost:5180/?mock=…`), at 552×780, 420×640 and 1280×900, in the Mac and Windows windows, light and dark.

| Reviewer | Report | Blocks use | Confuses | Annoys | Polish |
|---|---|---|---|---|---|
| A non-technical person with two laptops | [beginner.md](beginner.md) | 1 | 11 | 4 | 4 |
| A developer copying keys between a Mac and a PC | [developer.md](developer.md) | 1 | 7 | 5 | 5 |
| Three computers, a big transfer interrupted | [three-computers.md](three-computers.md) | 1 | 7 | 3 | 5 |

Screenshots are in `docs/review-05/shots/` on this Mac only (ignored by git, as before).

The same problem seen by several reviewers is one row. **Real** means the app does it; **simulation** means only the simulated other computer did, and the simulation was fixed too, so the next review isn't misled (decision 237). Every fix was checked again afterwards: screenshots of each fixed screen, the type check, lint and the 98 UI unit tests, the core tests named, and the layout check over every state, size, theme and window (`scripts/overlap.mjs`, with the new main-window notice added).

## Blocks use: 3, all fixed

| # | What | Who | Real? | What changed |
|---|---|---|---|---|
| B1 | The Shelf says "Only on this computer." with no way to share it from there; sharing was a checkbox in Settings, and nothing said what to do on the other computer | beginner 1, developer 2 | Real | The Shelf shows **Share with Desk PC** for each paired computer, "Waiting for Desk PC: on Desk PC, press Share." while it waits, and "Desk PC is away and gets these when back." when it is away. Help says where (decision 291) |
| B2 | **Send next** for a busy computer doesn't queue; the queued files vanish | three 1, 2, 3 | Simulation: the core queues them and starts them when the computer is free (`tests/next.rs`); the simulated core refused instead | The simulation queues and starts the queue like the core. The queued line now names the files ("notes.txt and plan.pdf") and sits above the file list, in view |
| B3 | **Put on Shelf** says "Kept on the Shelf" and nothing appears | developer 1 | Simulation (the core adds it, private stays private: `node/clip.rs` `shelf_put_*`) | The simulation adds the item. Real wording fix: a private item says "Kept on the Shelf, private." |

## Confuses: all fixed

| # | What | Who | Real? | What changed |
|---|---|---|---|---|
| C1 | "Not synced: … marked it private" only inside the Clipboard panel; the main window says nothing | developer 4 | Real | The notice shows in the main window too, with Put on Shelf, until dismissed |
| C2 | Send clipboard goes to every ticked computer, not only the one it syncs with; you only learn where after | developer 3 | Real | A line under the key says where it goes before it goes: "To 3 computers: Desk PC, MacBook Air and Studio iMac." |
| C3 | "Clipboard sync on" while the other computer is away; "sync waits" doesn't say what happens to copies; "Tick a connected computer" with one computer | developer 5 | Real | The label says "Clipboard sync waits" when everyone it syncs with is away; Help: "Copies made while the other computer is away are not sent later". The spec's status words ("isn't reachable; sync waits") stay. With one computer: "Desk PC isn't connected." |
| C4 | Nothing says whether copies stay out of clipboard history | developer 7 | Real (they do; it wasn't said) | Copy on a private item says "Copied, kept out of clipboard history."; Help says it (Win+V and the cloud clipboard on Windows) |
| C5 | Details records no clipboard activity | developer 6 | Simulation (the core logs "Sent to … (text, 1.2 KB)" and "Not synced") | The simulation logs it |
| C6 | The README, the update notice and the app tell different stories about the Shelf (and the README says "in memory only" and "the clipboard key next to the gear") | beginner 4, 5, developer 8 | Real (README); simulation (its update notes were 0.4.0's) | One description everywhere: "Text you keep, to copy here or on your other computers." README rewritten for Clipboard and Shelf, received files, quit vs drop and the queue; the simulation's notes are 0.5.0-beta.1's |
| C7 | The header's icons have no words | beginner 5 | Real | "Clipboard" and "Shelf" beside their icons from 520 px wide (on the Mac the centred title gives way below 640 px); icons alone below that |
| C8 | Sending with an item that can't be sent showed "Busy sending to Desk PC. 0 files" for good | beginner 2 | Real | After Send, what can't be sent leaves the list with the rest (it was already said to be left out) |
| C9 | The Mac says the PC "shows six digits", but the PC asks first; "lapses" | beginner 3 | Real | "On Desk PC, press Show code, then type its six digits here."; "goes away"; Help's Pairing says the order |
| C10 | The first screen's only key is Connect by IP | beginner 6 | Real | The first seconds say only what to do; Connect by IP comes with "Still looking" |
| C11 | "SeamDrop is open on the other computer" reads like a ticked fact | beginner 7 | Real | "Is SeamDrop open on the other computer?" (core `doctor.rs` and the simulation) |
| C12 | The pairing-lock warning shows twice and frightens | beginner 8 | Real | Once, above: "Pairing is locked: someone typed three wrong codes. Nothing was paired. … Allow it again only to pair your own computer." |
| C13 | "Stop sending? The part on Desk PC is removed." made the sender fear for their files | beginner 9 | Real | "Stop sending? Your files here stay; the part on Desk PC is removed." and the same idea on every cancel question |
| C14 | After a quit: no reason, no "how long", nothing telling the other side to press Continue; the drop screen said "SeamDrop closed there? It continues by itself", wrong after a quit | beginner 10, three 6 | Real | Stopped here: "SeamDrop closed before it finished. What's done is kept for 24 hours." The other side: "SeamDrop closed on Desk PC before it finished. Press Continue there." The drop screen: "Lost connection. Is the PC asleep or off the network? It continues by itself." |
| C15 | Paused screens don't say which way the files go | three 5 | Real | "Sending to Desk PC: paused" / "Receiving from Desk PC: paused"; "Checking the 280 MB already there/here, then sending/receiving carries on" |
| C16 | Device tiles read backwards ("Desk PC · Sending" when you send to it) | three 4 | Real | Always what that computer does: "Getting your files", "Sending you files", "Wants to send", "Asked to accept", "Paused" |
| C17 | A stalled transfer says only "Waiting for Desk PC…" | three 7 | Real | "Nothing is moving right now. It carries on by itself, or pauses if Desk PC goes away." (a real disappearance pauses within 3 s, §1.2) |
| C18 | One of three computers not reachable: no help in the window | three 8 | Real | Under the tiles: "Desk PC isn't reachable. Is it on and awake, with SeamDrop open? It connects by itself." |
| C19 | Not reachable: Try again shows nothing; "Stop trying" sounds like unpairing; nothing says it reconnects by itself | beginner 11 | Real (and the simulation doesn't retry) | "…It connects by itself when it's back."; after Try again: "Tried again just now: no answer yet."; the key is **Stop for now** |
| C20 | With the PC asleep, does a new Shelf item reach it? | beginner 12 | Real (it does, when back) | Said on the Shelf (B1) and in Help |

## Annoys and polish: listed, a few fixed

Fixed because they were one line each: **Help links open at their own section** ("How the Shelf works" opens The Shelf; beginner 13, developer 13); **Saved as a file** names the file ("Saved as Clipboard image ….png in your received files folder"; developer 10); the Shelf's example name is "Wi-Fi password", not "Vercel key" (beginner 19); "Add folder" everywhere (beginner 20); **+** at the end of the device strip (Mission 05 §2.2, should-have).

For the next mission:
- **No "last received" line on the main screen after Done** (beginner 14): only Settings → Open finds the files again.
- **Files can't wait for a computer that's asleep** (beginner 15): the queue is only for a busy one.
- **"Ran out of space" offers Send again** without saying how much room is needed (beginner 16).
- **Received images all read "An image"**, no size or type (developer 11); **Recently received can't be cleared** item by item (developer 12).
- **A pasted key shows in plain text until Private is ticked**, and an item can't be made private afterwards (developer 9).
- **Adding files during a transfer hides that transfer** behind the send list (three 10).
- **"Checking…" after a resume has no progress of its own** (three 11).
- **"Pair another computer" spins with no checklist** when nothing is found (three 9).
- **The finished summary doesn't say there was an interruption** (three 12).
- **Device icons follow the system, not the machine** (an iMac gets a laptop; three 13): device pictures (§2.3) are the answer.
- Polish: the pause mark looks like a button (beginner 18, three 14); disabled keys look enabled (beginner 18, developer 18); addresses, speeds and paths on everyday screens (beginner 17); a revealed key has no Hide (developer 14); Remove and Undo both ask (developer 15); a new Shelf item can land out of view (developer 16); sync keys before sync starts (developer 17); small-window wraps (developer 18, three 16); full stops and Cancel pairs differ between screens, "If the Mac still can't reach this PC" assumes a Mac (three 15); "Website: seamdrop.com (not live yet)" in Settings (beginner 20).

## Simulation only (not counted by the reviewers, and fixed where it misled)

Fixed in the simulation: queueing for a busy computer and starting the queue; Put on Shelf; Reveal showing the item's own text; Details lines for the clipboard; the received-files choice (the layout check couldn't reach "A folder for each computer" either); Remove after "removed this pairing"; its update notes. Still simulated: the other computer never accepts or agrees by itself, sample files don't match what was added, the same "9.9 s" for every transfer, no real clipboard, notifications, tray or folder pickers.
