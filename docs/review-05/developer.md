# Review 05: the developer

## Who I am

I work on a Mac and a Windows PC side by side and copy API keys, tokens and `.env` values between them all day. I care that secrets don't leak (into clipboard history, previews, notifications, logs), that things are quick, and that I always know what was synced and what wasn't. All I knew going in was: "SeamDrop moves files and your clipboard between your computers, and keeps a shared Shelf." I used the browser build with simulated other computers, mostly at 552×780, plus 420×640 and 1280×900, in both the Mac and Windows windows, light and dark. Screenshots are in `shots/developer-<n>.png`.

## What I tried

| Task | Finished? | Steps / clicks | Verdict |
|---|---|---|---|
| 1. Turn on automatic clipboard sync; work out what is synced, with which computer, how to pause or stop | Yes, using both windows | Mac: Clipboard icon, tick "Sync with Desk PC" (2). It says "Waiting for Desk PC". Windows: "Turn on" in the banner (1). Pause or Turn off: 1 click each | Turning it on, pausing and stopping are easy and clear. Knowing what was actually synced is weak: Details records nothing, and the top label says "Clipboard sync on" even when the other computer is unreachable. |
| 2. Copy something marked private, see what the app says, get it to the other computer safely | Partly | Clipboard icon, read the notice, Put on Shelf, Open the Shelf (3). The item isn't there. Workaround: Shelf, paste, tick Private, Add (4), then Settings, scroll, tick "Share the Shelf with Desk PC" (3), then wait for the other computer | The notice itself is good. The button it offers says it worked but nothing lands on the Shelf, and nothing on screen says the Shelf has to be shared from Settings before anything reaches the other computer. |
| 3. Shelf: add a private key, reveal, copy, remove, share with the other computer | Mostly (couldn't finish sharing in the simulation) | Add private: 4. Reveal: 1. Copy: 1. Remove: 2 (+1 Undo). Share: 3 in Settings | Masking and reveal work well. Private is off by default, so a pasted key shows in plain text first. Sharing is hidden in Settings. |
| 4. Recently received: find what arrived a minute ago, including an image, and save it | Yes | Clipboard icon, find the row by clock time, Save as file (2) | Works, but every image row is called "An image" and "Saved as a file." doesn't say where. |
| 5. Check Details and Settings for secret content or anything unclear | Yes | Details, Technical details, Copy technical log; Settings scrolled through | No secret content anywhere, which is good. Details also has no clipboard or Shelf activity at all. |

## What worked well

- **Asking before sync starts.** On the other computer: "MacBook Pro asks to sync the clipboard · Nothing syncs unless you turn it on. **Turn on** / **Not now**". On my side: "Sync waits for the other computer to agree." Nobody can quietly start pulling my clipboard. (`?mock=online&os=win&clip=request`, developer-16, developer-12, developer-6)
- **Pause says when it ends.** "Sync paused until 19:19." and "Sync paused until SeamDrop restarts.", with Resume and Turn off right there. (`?mock=online&os=mac&clip=paused`, developer-14, developer-88)
- **Sync is chosen per computer.** With three computers each one has its own "Sync with …" tick, and the top label names the one I sync with: "Clipboard sync: Desk PC". (`?mock=online&os=mac&devices=3&clip=sync`, developer-72, developer-73)
- **The send message gives type and size, never the content,** and uses the other computer's paste shortcut: "Sent to MacBook Pro (text, 1.2 KB). Paste it there with ⌘V." when sending from Windows. (developer-21)
- **The private-copy notice is plain:** "Not synced: the app you copied from marked it private." A private item that arrives shows as "Private text" with a lock and no preview. (`?mock=online&os=mac&clip=notice`, developer-27; `&clip=sync`, developer-8)
- **Shelf privacy works.** Ticking Private masks the text box straight away (developer-41). Items show as dots with a lock. Reveal hides itself again after a few seconds (developer-43, developer-44). Clear Shelf names the computers: "Clear the Shelf on this PC and MacBook Pro?" (developer-63). Remove has Undo (developer-48).
- **Logs stay clean.** Details and the technical log hold only start, discovery and connection lines. With Technical details on you get ports and addresses, which is useful to me (developer-25). Settings says clearly: "Never what's in your files or your clipboard." (developer-54)
- **Help is honest about the limits:** which browsers mark private copies, "Other browsers may not mark them, so copy from them with care", and the caveat about the Mac key file. (developer-4, developer-90)

## Findings

1. **blocks use** (unless this is the simulation; see the note): **"Put on Shelf" says it worked, but nothing appears on the Shelf.**
   - **What I saw:** On the private-copy notice, Put on Shelf showed "Kept on the Shelf, private. · Open the Shelf". Open the Shelf showed an empty Shelf with "Only on this computer." I got the same with Put on Shelf on a received item: "Kept on the Shelf." and an empty Shelf. With `&shelf=1` the three preset items stayed and no new one was added. The second case worries me more because the item was the received **"Private text"**: the message said "Kept on the Shelf." without the word "private", where every other private add says "…, private". I couldn't check whether it would have been stored as non-private.
   - **Where:** `?mock=online&os=mac&clip=notice` → Clipboard → Put on Shelf → Open the Shelf. `?mock=online&os=mac&clip=sync` → Clipboard → Put on Shelf on "Private text" → Shelf.
   - **Screenshots:** developer-33, developer-34, developer-35, developer-36, developer-101. Adding through the text box does work (developer-42).
   - **Note:** the notice path may depend on the real system clipboard, which isn't simulated. The received-item path shouldn't depend on it.
   - **What would have helped:** show the new item, marked private, or don't say "Kept".

2. **confuses: Nothing on the Shelf tells you how to get a private item to the other computer.**
   - **What I saw:** The private notice offers only Put on Shelf, and the Shelf says "Only on this computer." with no button. Sharing is in Settings → Paired devices → "Share the Shelf with Desk PC", below the fold. After I ticked it, Settings said "Waiting for Desk PC to agree." while the Shelf still said "Only on this computer."
   - **Where:** `?mock=online&os=mac`, then Shelf, then Settings.
   - **Screenshots:** developer-34, developer-57, developer-58.
   - **What would have helped:** a "Share with Desk PC…" link next to "Only on this computer.", and the waiting state shown on the Shelf too. Even better, a "Send privately to Desk PC" action on the notice.

3. **confuses: Send clipboard goes to every ticked computer, not just the one I sync with.** This can leak a secret to a computer I didn't intend.
   - **What I saw:** With three computers the panel says "Syncing with Desk PC." I pressed Send clipboard and got "Sent to Desk PC, Studio iMac and MacBook Air (text, 1.2 KB)." The panel doesn't show where Send clipboard will go until it has gone.
   - **Where:** `?mock=online&os=mac&devices=3&clip=sync` → Clipboard → Send clipboard.
   - **Screenshots:** developer-73, developer-75.
   - **What would have helped:** a button that names its targets ("Send to 3 computers") or a line under it saying where it goes.

4. **confuses: The main window never says a copy wasn't synced.**
   - **What I saw:** In the private-copy state the main window shows only "Clipboard sync on" (I watched it for 6 s, no notice). "Not synced: the app you copied from marked it private." exists only inside the Clipboard panel. "Too big for the clipboard." and "Couldn't read that copy. Try Send clipboard." are the same. Help promises "If a copy is not synced, the window says why, once." In real use I'd paste the old clipboard on the PC and not know why.
   - **Where:** `?mock=online&os=mac&clip=notice` (and `toobig`, `unreadable`).
   - **Screenshots:** developer-26 next to developer-27; developer-28 next to developer-29; developer-30 next to developer-31.
   - **Note:** the real app may also show a system notification, which isn't simulated.
   - **What would have helped:** the top label changing to "Last copy not synced", or a dot on the clipboard icon.

5. **confuses: "Clipboard sync on" while the other computer is offline, and "sync waits" doesn't say what happens to my copies.**
   - **What I saw:** With Desk PC "Not reachable" or "Disconnected", the label still reads "Clipboard sync on". The panel says "Desk PC isn't reachable; sync waits." That doesn't tell me whether copies made meanwhile are queued and delivered later (a token from two hours ago turning up on the PC clipboard) or dropped. The same panel also says "Tick a connected computer to send to." with only one computer, where there is nothing to tick.
   - **Where:** `?mock=offline&os=mac&clip=sync`, `?mock=disconnected&os=mac&clip=sync`.
   - **Screenshots:** developer-91, developer-92, developer-95.
   - **What would have helped:** "Sync on · Desk PC offline" in the label, and one line such as "Copies made while it's away aren't sent."

6. **confuses: Details doesn't record any clipboard or Shelf activity, so I can't check what was synced.**
   - **What I saw:** I turned sync on, sent the clipboard (message: "Sent to MacBook Pro (text, 1.2 KB)"), paused, turned it off, and added and copied Shelf items. Details still listed only "SeamDrop started as …", "Found Desk PC on this network" and "Connected to Desk PC over Wi-Fi". There is no "sent" history anywhere; Recently received covers incoming only.
   - **Where:** `?mock=online&os=win&clip=request` → Turn on → Send clipboard → Details. `?mock=online&os=mac&devices=3&clip=sync` → Send, Pause, Turn off → Details.
   - **Screenshots:** developer-22, developer-23, developer-78, developer-102.
   - **What would have helped:** lines with type and size only, for example "Clipboard sent to Desk PC (text, 1.2 KB)", "Private copy not synced", "Sync paused until 19:19".

7. **confuses: The app never says whether what it puts on the clipboard stays out of clipboard history.**
   - **What I saw:** Copy on a private Shelf item said "Copied. Paste it anywhere." Copy on a received "Private text" said "On the clipboard again, ready to paste." Searching Help for "history" gave "Nothing in Help matches that." On Windows (Win+V history and cloud clipboard) this is the first thing I need to know before I copy a token. The README answers it ("unless it was sent as private"); the app doesn't. I found no option to clear the clipboard after a while either.
   - **Where:** `?mock=online&os=mac` (Shelf → Copy); `?mock=online&os=mac&clip=history` (Copy on "Private text"); `?mock=online&os=win&clip=sync` → How the clipboard works → search "history".
   - **Screenshots:** developer-45, developer-70, developer-89.
   - **What would have helped:** "Copied as private: kept out of clipboard history" in the message, and a sentence in Help.

8. **confuses: The README and the app disagree on how the Shelf works.**
   - **What I saw:**
     - The README says the Shelf keeps things "in memory only, until SeamDrop quits". The app's Help says "The Shelf is kept encrypted on disk." For someone deciding where to keep API keys, that's the most important fact on the page.
     - The README says Send clipboard is "on the Shelf (the clipboard key next to the gear)". The app has separate Clipboard and Shelf buttons.
     - The README says "per computer you choose whether arrivals also go straight to the clipboard or wait on the Shelf". I found no such setting, and Help says "Nothing lands on it by itself."
   - **Where:** README, "The clipboard and the Shelf". In the app: Help (developer-4, developer-90) and Settings (developer-51 to developer-54).
   - **What would have helped:** one consistent description.

9. **annoys: A pasted key is visible until you tick Private, and Private is easy to miss.**
   - **What I saw:** Pasting a key into the Shelf shows it in plain text until Private is ticked, which is off by default. With three items on the Shelf, the Private box falls below the window edge at 552×780. Once an item is added it can't be made private (only Reveal, Copy and Remove), so a slip means remove and re-add. On a shared Shelf the slip has already reached the other computer.
   - **Where:** `?mock=online&os=mac` → Shelf. `?mock=online&os=mac&shelf=1` → Shelf → Add to Shelf.
   - **Screenshots:** developer-40, developer-103.
   - **What would have helped:** Private above the text box (or remembered), and "Make private" on existing items.

10. **annoys: "Saved as a file." doesn't say where.**
    - **What I saw:** Save as file on a received image gives only "Saved as a file.", with no file name, no folder and no Show in Finder or Show in Explorer. The finished file-receive screen does this well: "saved in Downloads/SeamDrop" plus Show in Finder.
    - **Where:** `?mock=online&os=mac&clip=history` → Clipboard → Save as file. `?mock=online&os=win&clip=sync` at 420×640.
    - **Screenshots:** developer-67, developer-100; compare developer-108.

11. **annoys: You can't tell the received images apart.**
    - **What I saw:** Three rows all called "An image", with no size, dimensions or type. Only one has a thumbnail, and clicking a row does nothing. The last one says "too big to keep" without a size. I found "the one from a minute ago" by comparing clock times.
    - **Where:** `?mock=online&os=mac&clip=history` → Clipboard.
    - **Screenshots:** developer-66, developer-69.
    - **What would have helped:** "Screenshot · 1440×900 · 320 KB", relative times ("1 min ago"), and a click to preview.

12. **annoys: You can't clear Recently received.**
    - **What I saw:** No per-item remove and no Clear. A received private text keeps a working Copy button until SeamDrop quits. Non-private texts show their first ~30 characters ("“Meeting notes, Thursday: budg…”"). A key copied from a `.env` in my editor isn't marked private, so it would sit there readable on screen.
    - **Where:** `?mock=online&os=mac&clip=sync` → Clipboard.
    - **Screenshot:** developer-66.
    - **What would have helped:** "Clear" plus per-item remove, and an option to hide previews.

13. **annoys: The "How … works" links don't open at the right part of Help.**
    - **What I saw:** "How the clipboard works" and "How the Shelf works" both open Help at "Pairing", not at the section you came from.
    - **Screenshots:** developer-4, developer-50.

14. **polish: A revealed key has no Hide button.**
    - **What I saw:** The eye button disappears while the key is shown, so you can't hide it yourself. It re-hides after a few seconds with no sign of when.
    - **Screenshot:** developer-43.

15. **polish: Remove, Clear Shelf and the bottom message have rough edges.**
    - **What I saw:**
      - Remove asks "Remove 'Stripe test key' from the Shelf on your devices?" and then also offers Undo, so it takes two steps. It says "on your devices" while the Shelf says "Only on this computer." Clear Shelf is better: it names the computers.
      - In the Windows window at 552×780 the Clear Shelf question overlaps "How the Shelf works" and its buttons are below the edge.
      - "Kept on the Shelf, private." stays in the bottom bar after the item has been removed.
    - **Screenshots:** developer-46, developer-48, developer-63.

16. **polish: A newly added item can end up out of view.**
    - **What I saw:** On a Shelf that already has items, the new item goes in at the top, scrolled out of view. Only its bottom edge shows.
    - **Where:** `?mock=online&os=mac&shelf=1`.
    - **Screenshot:** developer-104.

17. **polish: The sync buttons are unclear before sync starts and with several computers.**
    - **What I saw:** While it still says "Waiting for Desk PC", it already offers "Pause 1 hour / Pause until restart / Turn off" (pausing something that hasn't started), and "Turn off" repeats the tick box. With several computers, Pause and Turn off don't say whether they act on one computer or all of them.
    - **Screenshots:** developer-6, developer-73.

18. **polish: Small layout and wording issues.**
    - **What I saw:**
      - At 420×640, "From MacBook Pro · 18:34" wraps to "From MacBook / Pro · 18:34", and the Shelf's "from" sits alone on its own line.
      - The disabled "Add to Shelf" is a bright white button and looks clickable.
      - The Help search icon sits above the box instead of inside it.
      - The red warning triangle on "marked it private" reads as an error, though the app did the right thing.
      - "Too big for the clipboard." doesn't say what the limit is.
    - **Screenshots:** developer-100, developer-82, developer-3, developer-4, developer-27, developer-29.

### Probably the simulation (not counted)

- Reveal showed "sk_live_51H…demo" both for the key I typed ("sk_test_51Hx…") and for the preset "Vercel key", so it looks like placeholder text (developer-43, developer-63). If the real reveal also cuts the middle out, I'd want a way to see the whole key to check it's the right one.
- The simulated other computer never agreed to clipboard sync or Shelf sharing started from the Mac (I waited 12 s and 20 s; developer-15). I used `&clip=request` on the Windows window to see the other side. There is no state that shows the other side being asked to share the Shelf, so I couldn't judge that prompt.
- "Send as file" on a too-big copy answered "Sent to Desk PC (text, 1.2 KB). Paste it there with Ctrl+V." and the notice stayed (developer-105). The size looks canned, but "paste it" is the wrong message for a file.
- With three computers, Details logs only Desk PC (developer-78).
- No live arrivals, and no real clipboard, notifications or menu bar/tray, so I couldn't check whether private items stay out of notifications.

Findings per severity: **blocks use 1 · confuses 7 · annoys 5 · polish 5**
