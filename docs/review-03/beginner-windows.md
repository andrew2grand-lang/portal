# Review 03: beginner on a Windows gaming PC

**Persona:** 19, games on a Windows PC wired to the router, at home in Steam and Discord. Wants a 12 GB capture and photos off a family Mac, maybe a link too. Doesn't know mDNS, firewall rules or "paired devices".
**Tested:** the top of the README, then the real screens at 552×780 (`os=win` mostly). Shots: `shots/beginner-*.png`.

## Tasks at a glance

| Task | Finished? | Steps | Verdict |
|---|---|---|---|
| A. Install + pair | Yes on the happy path, **no** if the firewall check fails | Windows: 5 (download, More info, Run anyway, admin Yes, open). Pairing: 2 clicks | Pairing is easy; the install text and firewall fix are scary |
| B. Transfer drops and resumes | Yes, if I don't touch Cancel | Accept, wait | I'd wait, not panic, but Cancel is a trap |
| C. Update | Yes | 1 | "Restart" sounded like rebooting my PC |
| D. Clipboard | Sending yes; finding received items only because I was told | 1–2 | Worried the Mac can turn on syncing of my clipboard |

## Findings

**1. The firewall fix means deleting Windows Firewall rules by hand**
- Screen: `?mock=searching-long&os=win` (same on `offline`) · Shot: `beginner-a5-searching-long.png`
- What happened: a red ✗ says "Block rules always win over Allow rules. In Windows Defender Firewall → Advanced settings → Inbound Rules, delete: SeamDrop." The button opens a console with hundreds of rules.
- Why it matters: I won't delete firewall rules (it feels like turning off security), and I can't tell a block rule from an allow rule. This is the case I'd most likely hit, because I click Cancel on "Allow this app?" pop-ups while gaming. I'd give up here.
- Fix: a **Fix it** button that asks for admin once and removes only the block rule. Add one line on why: "You may have clicked Cancel on a Windows pop-up."
- Severity: **blocks use**

**2. Cancel on a paused transfer throws the progress away, with no warning**
- Screen: `?mock=paused-in&os=win` → Cancel · Shot: `beginner-b-paused-in.png`, `beginner-b-paused-in-cancel.png`
- What happened: Cancel is the only button. One click and the transfer is gone. I land on "isn't reachable", with nothing saying the transfer was cancelled.
- Why it matters: with 12 GB, my reflex is to press the only button to close or "restart" it, and everything received is lost.
- Fix: confirm with the stakes ("Stop and delete the 5.4 GB already received?"). Add **Keep waiting**, and say "Transfer cancelled" afterwards.
- Severity: **confuses**

**3. The PC never says how to *get* files from the Mac**
- Screen: `?mock=online&os=win` · Shot: `beginner-a4-after-match.png`, `beginner-a10-choose-folder.png`
- What happened: after pairing, the screen only offers "Drop files … They go to MacBook Pro". I looked for Receive. "Choose a folder" looked like "where to save", but it adds a folder to *send*.
- Why it matters: receiving is my whole goal. Nothing says the Mac sends first, or that files land in `Downloads\SeamDrop` (that's only in Settings, and in the README's Portal section).
- Fix: add one line: "To get files from MacBook Pro, send them from there; you'll be asked here. They're saved to Downloads\SeamDrop." Rename the key to "Add a folder", which is what the README calls it.
- Severity: **confuses**

**4. A "Project · Paid" tab on every screen**
- Screen: any; click it on `?mock=online&os=win` · Shot: `beginner-a11-project-paid.png`
- What happened: a "Paid" badge sits next to Files. It leads to "Project Send is coming soon … a code project … ready to run".
- Why it matters: I thought the app was a trial that would ask for money. "Code project" means nothing to me.
- Fix: hide it until it ships, or put it in Settings.
- Severity: **confuses**

**5. The README install reads like a security warning, and the Mac needs Terminal**
- Screen: README → Install · Shot: none
- What happened: "aren't signed", "Run anyway", "Firewall rules … on private *and* public networks", "WebView2". The intro also has dev notes (`src-tauri/tauri.conf.json`, "Mission 03"). The Mac steps say to paste `xattr -dr …` into Terminal.
- Why it matters: "public networks" plus "unsigned" is when a parent says no. A Terminal command on someone else's Mac felt like hacking.
- Fix: put user steps first and dev and status notes below. Explain plainly: "lets the Mac reach this PC at home; Windows often labels home Wi-Fi 'Public'". For a home Mac, give **Open Anyway** first and Terminal as the fallback.
- Severity: **confuses**

**6. Paused: "After that it's removed", and no hint what to do**
- Screen: `?mock=paused-in&os=win` · Shot: `beginner-b-paused-in.png`
- What happened: "What has arrived is kept for 24 hours … After that it's removed."
- Why it matters: "removed" made me fear my finished photos would be deleted. Nothing names the likely cause (the Mac asleep or its lid closed), so I don't know what to fix.
- Fix: "Keep the Mac awake with its lid open. What's arrived waits 24 hours; finished files are safe in Downloads\SeamDrop."
- Severity: **confuses**

**7. The received-clipboard key is just an icon and "4"**
- Screen: `?mock=online&os=win&clip=history` · Shot: `beginner-d1-clip-history.png`
- What happened: a small key with a clipboard icon and the number 4.
- Why it matters: it reads as a notification badge. I'd never guess it holds the link the Mac sent.
- Fix: label it "4 from Mac".
- Severity: **confuses**

**8. Is the Mac's link already on my clipboard, or do I press Copy?**
- Screen: same, open the key · Shot: `beginner-d2-clip-history-open.png`
- What happened: the panel opens with "Copy here, paste on MacBook Pro" (the other direction), then lists items with **Copy** buttons. The README says received items are "ready to paste".
- Why it matters: I don't know whether to press Ctrl+V or Copy, and nothing tells me something just arrived.
- Fix: a toast on arrival ("Link from MacBook Pro, press Ctrl+V"). Start the panel with my direction.
- Severity: **confuses**

**9. "One switch for both computers": the Mac can turn on syncing of *my* clipboard**
- Screen: Settings → Clipboard, clipboard panel · Shot: `beginner-s3-settings-bottom.png`, `beginner-d3-clip-sync.png`
- What happened: auto-sync is shared. If the family member turns it on, everything I copy goes to their Mac. Password managers are excluded, but I copy Steam Guard and Discord codes from other places.
- Why it matters: this would make me uninstall. The "Clipboard sync on" pill helps, but I want to be asked.
- Fix: turning it on from the other computer asks this one, or at least notifies it. Spell out what's never sent.
- Severity: **confuses**

**10. "Restart to update": does it restart my PC?**
- Screen: `?mock=online&os=win&update=ready`; `mock=sending&update=waiting` · Shot: `beginner-c1-ready.png`, `beginner-c2-waiting-sending.png`
- What happened: the banner says "Restart to update" and later "Restarts to update when the transfer finishes".
- Why it matters: to me, restart means rebooting the PC mid-game, so I'd press Later forever. Waiting for the transfer, with "Nothing is lost", is great though.
- Fix: "Restart SeamDrop (a few seconds)".
- Severity: **confuses**

**11. Update failed: "didn't pass its signature check", and the only button is Later**
- Screen: `?mock=online&os=win&update=failed` · Shot: `beginner-c3-failed.png`, `beginner-c7-about-failed.png`
- What happened: there's no next step. About only offers "Check for updates".
- Why it matters: after the install warnings, this sounds like a hacked download.
- Fix: "The download was damaged or not from us, so it was thrown away. Nothing changed." Add **Try again** and a link to the download page.
- Severity: **confuses**

**12. The accept screen doesn't say where files go or whether there's room**
- Screen: `?mock=offer-in&os=win`, `&view=accept` · Shot: `beginner-b-offer-in.png`, `beginner-b-accept-window.png`
- What happened: it shows names, sizes, Accept and Decline. The small window doesn't say "SeamDrop" anywhere.
- Why it matters: my C: SSD is full of games, so 12 GB might not fit. An unbranded pop-up over a game looks like spam.
- Fix: "Saves to Downloads\SeamDrop · 48 GB free", with a warning if it won't fit. Put the app name in the small window.
- Severity: **annoys**

**13. Engine-log text is always on screen**
- Screen: bottom bar, every screen · Shot: `beginner-a1-nearby.png`
- What happened: "found MacBook Pro 192.168.1.24:47920 via mdns" and "start Desk PC on port 47920".
- Why it matters: it looks like an error console, so the app feels unfinished.
- Fix: show just "Details" until opened.
- Severity: **annoys**

**14. Connect by IP shows my own address right under the box**
- Screen: `offline` → Connect by IP · Shot: `beginner-a6-connect-by-ip.png`
- What happened: there's a real-looking placeholder (192.168.1.20), with "This computer's addresses: 192.168.1.31" just below.
- Why it matters: the obvious move is to type the number I can see, which is my own.
- Fix: "Type the number shown on the **Mac's** SeamDrop." Reject this PC's own address with a clear message.
- Severity: **annoys**

**15. Small stuff**
- Screen / Shot: as listed
- What happened:
  - The done screen says "1 name changed" and "2 items left out" without saying which; did the video get skipped? (`beginner-b-done-in.png`)
  - Resuming says it continues while the stats say "Paused", and the swirl animates while paused (`beginner-b-resuming.png`).
  - Buttons cover the last checklist item on the offline screen (`beginner-a5-offline.png`), and Settings' ✕ covers text when scrolled (`beginner-c7-about-failed.png`).
  - ".DS_Store and Thumbs.db" is jargon (`beginner-s1-settings-top.png`).
  - "Clear" beside "Send clipboard" might wipe my clipboard, and "No longer kept" is unexplained (`beginner-d2-clip-history-open.png`).
- Severity: **polish**

## What worked well
- Pairing: two clicks, a huge code, and "Codes match" is plain English.
- The searching text covers my setup (PC wired, Mac on Wi-Fi).
- "Paused … Will continue when it's back" and "MacBook Pro is back" kept me calm.
- The update waits for the transfer and says "Nothing is lost".
- "Show in File Explorer" when it's done, and a clear message when a private clipboard item isn't sent.
- It closes to the tray like Discord and Steam.

## Notes on the mocks
- `stalled` only shows the sending side; I couldn't see a receiver's stall.
- `mock=sending&update=waiting` names "Desk PC" (this PC). The normal Restart path names the Mac, so this is probably only the fixture.
