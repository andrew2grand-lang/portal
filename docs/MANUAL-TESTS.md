# Manual tests: Mac ↔ Windows on home Wi-Fi (brief §4, updated)

Everything here needs your real Mac and PC. Each test says what to do, what you should see, and what to send back if it doesn't happen. Tick them off as you go.

**Where to test:** at home, on your own router. Not on school or guest Wi-Fi: those networks keep devices apart on purpose, so nothing there proves anything about SeamDrop.

**The setup to test (the common one):** the **PC plugged into the router by Ethernet**, the **Mac on that router's Wi-Fi**. Same router, not a guest network. If your router has separate 2.4 GHz / 5 GHz names, either is fine.

**If something fails, send:**
1. A screenshot of the window with **Details** open (the **Details** button at the bottom left), and its **Copy technical log** pasted into your message (or **Send feedback** with "Include technical log": it saves the log in Downloads).
2. The log file: macOS `~/Library/Logs/com.seamdrop.desktop/seamdrop.log` (Finder → Go → Go to Folder…), Windows `%LOCALAPPDATA%\com.seamdrop.desktop\logs\seamdrop.log` (paste into the Explorer address bar).
3. For anything network-related: on the PC, the output of `ipconfig`, `Get-NetConnectionProfile` and `Get-NetFirewallRule -DisplayName SeamDrop | Format-List DisplayName,Enabled,Action,Profile` in PowerShell; on the Mac, `ifconfig en0` in Terminal.

## beta.2: the 5-minute Settings check

On each computer with 0.5.0-beta.2, Settings (the gear, top right). Every change should stick when you quit SeamDrop and open it again. If anything doesn't work, the window now says so at the bottom: send a screenshot of **Details** (bottom left).

1. **About:** it says "SeamDrop 0.5.0-beta.2". Tick **Get test builds**: the line under the version says "You're on the latest test build." (never that the update information couldn't be read).
2. **Appearance:** Dark, then Light, then System. The window changes at once each time.
3. **Name:** type a new name and press Enter. The other computer shows it within a few seconds.
4. **Picture:** pick a colour, then an icon: the picture in the window's header changes, and on the other computer too. **Choose photo…** and pick a photo: it shows in a circle. Then **Default**.
5. **Received files:** **SeamDrop folder**, tick **A folder for each computer**, then **New folder each time**, then **Straight in**. The line under them changes each time.
6. **Keep original dates**, **Skip system files**, **Open at login**, **Install updates automatically**: tick and untick each. Quit SeamDrop (menu bar or tray → Quit), open it: they're as you left them.
7. **Send clipboard shortcut → Change…**, press a new combination: it shows the new one. Copy some text and press it: it arrives on the other computer.
8. **Paired devices → Files from <the other computer> → Always accept**, then send a file from the other computer: it arrives without asking.

## Mission 05: the 25-minute check

Ordered by what matters most. **All three computers need 0.5.0-beta.1** (it can't talk to 0.3): this Mac already has it; the PC and Sam's MacBook Air get it by hand from the pre-release page once it's published (PROGRESS → Needs the owner). In each step, the Mac is this Mac (MacBook Pro), the PC is Den_PC, and the Air is Sam's MacBook Air.

1. **Check the version (1 min).** On each computer: Settings → About. **Expect:** "SeamDrop 0.5.0-beta.1". Turn on **Get test builds** on all three. *Send back:* the version line if it says anything else.
2. **Find each other with nothing paired (3 min, three computers).** On each computer remove every pairing (Settings → Paired devices → Remove). Quit and reopen SeamDrop on all three. **Expect:** within about 5 seconds each window lists the two others with name, system and address, a **Pair** key beside each, nothing picked for you. *Send back:* a screenshot of any computer that lists fewer than two, and how long it took.
3. **Pair all three (3 min).** Mac: **Pair** beside the PC, then **Show code** on the PC, type it on the Mac. Do the same Mac ↔ Air and PC ↔ Air. **Expect:** "Paired with …" on both sides, then the list again with whoever is left. *Send back:* anything that read oddly.
4. **A computer that disappears shows within 3 seconds (3 min).** Send a big file (a few GB) Mac → PC. On the PC, pull the Ethernet cable. **Expect, Mac:** "Sending to Den_PC: paused" with "Lost connection…" within about 3 seconds (count them), the bar still. Plug it back in: "Den_PC is back", "Checking the … already there", then it carries on by itself; the time left says nothing until it's moving again, then changes calmly. *Send back:* the seconds you counted, and any time-left words that jumped back and forth.
5. **Quit is asked, a drop isn't (3 min).** During the same kind of transfer, quit SeamDrop on the Mac (menu bar icon → Quit) and open it again. **Expect, Mac:** "Sending … to Den_PC stopped partway", "SeamDrop closed before it finished…", with **Continue** and **Cancel**; **PC:** "Receiving from MacBook Pro: paused", "SeamDrop closed on MacBook Pro before it finished. Press Continue there." Press **Continue**: it goes on from where it was. *Send back:* a screenshot of both sides before you pressed Continue.
6. **The queue (3 min, three computers).** Start a big transfer PC → Air. While it runs, on the Mac tick only the Air, drop a file and press Send. **Expect, Mac:** "Sam's MacBook Air is busy: these go next." then "Next for Sam's MacBook Air: … · starts when the current transfer finishes", with ✕ to take it off. When the PC's transfer ends, the Mac's starts by itself. *Send back:* whether it started without you, and how long after.
7. **The receiving light on the Air (2 min).** Send a big folder Mac → Air and watch the Air's window. **Expect:** a calm light travelling around the edge, smooth, no orange crawl, no stutter. Open Activity Monitor on the Air while it runs. *Send back:* SeamDrop's % CPU, and a short screen recording if it isn't smooth.
8. **Receiving feels finished (2 min).** Send one PDF Mac → PC. **Expect, PC:** a tick, "Received … from MacBook Pro", **Show in File Explorer** (opens the folder with the file selected), **Open** and **Done**. Then send a `.command` or `.exe` file: **Open** isn't offered, only Show in File Explorer. *Send back:* a screenshot of each.
9. **Clipboard (3 min).** Mac: **Clipboard** in the header → **Sync with Den_PC**; on the PC say yes. Copy a sentence on the Mac, paste on the PC; copy on the PC, paste on the Mac. Then copy from a Firefox private window (or a password manager). **Expect:** the window says once "Not synced: the app you copied from marked it private." with **Put on Shelf**. **Recently received** lists the last five things that arrived; an image there has **Save as file**. *Send back:* anything synced that shouldn't have been, or not synced without a reason shown.
10. **The Shelf between three computers (4 min).** Share the Shelf Mac ↔ PC and PC ↔ Air, but not Mac ↔ Air (answer **Not now** there). Add a text on the Mac. **Expect:** it appears on the PC, **not** on the Air. Add one on the PC: it appears on the Mac and on the Air. Remove the PC's item on the Mac: it's gone on all three. Add one as **Private**: dots until you press the eye, never in a notification. Quit and reopen the Mac: the Shelf is still there. *Send back:* a screenshot of the three Shelves after the first two adds.
11. **Where received files go (2 min).** PC: Settings → Received files. **Expect:** **Straight in** picked, going into Downloads; if it used to be Downloads\SeamDrop, it said once where files go now. Send an old file from the Mac: in File Explorer it's dated today, at the top when sorted by date. Turn on **Keep original dates** and send it again: the old date. Try **New folder each time**: "SeamDrop 001 – from MacBook Pro". *Send back:* anything dated wrong.
12. **Help (1 min).** Settings → Help, search "private". **Expect:** the Privacy part, in plain words. *Send back:* anything you couldn't find there.

## Mission 04: the 20-minute check

The things that matter most, in order. Two computers (Mac and PC) first; a third one for the last part if you have it. Update both to the Mission 04 build (test 1) before you start.

1. **Opening both connects by itself (2 min).** Quit SeamDrop on both. Open it on the Mac, then on the PC, within a few seconds of each other. **Expect:** each shows the other as Connected within about 5 seconds; never "isn't reachable" on the way. Repeat three times. *Send back:* how many seconds each time, and a screenshot if it said "isn't reachable".
2. **Pair by typing the code (3 min).** Remove the pairing on both (Settings → Paired devices → Remove), then do test 4 below: Pair on the Mac, **Show code** on the PC, type it on the Mac, once with a wrong digit first. **Expect:** "Paired with …" on both, with a notification. *Send back:* anything that read oddly, and whether you understood which computer to type on.
3. **A big transfer that drops and comes back (5 min).** Send the 10 GB file Mac → PC. While it runs, watch the time left for a minute: it should change calmly ("About 2 minutes", then "Less than a minute"…), never jump back and forth. Turn the Mac's Wi-Fi off for 20 seconds, then on. **Expect:** "Paused" with a still mark (nothing spinning), then it carries on by itself from where it was ("nothing sent twice"); at 100 % it says "Finishing…"; the done screen says where the files went and counts only what moved after the pause in the speed ("10.0 GB: 6.1 GB sent in …"). While it runs, drop a small file on the window: the send list shows, saying the PC is busy with another transfer. *Send back:* the time-left words you saw, and the Details story after it finished (Copy technical log).
4. **Small window (2 min).** Make the window as small as it goes, on both. Look at the drop zone, a transfer, Settings, the Shelf and Details (the button at the bottom). **Expect:** no words on top of each other, nothing cut off. *Send back:* a screenshot of anything that overlaps.
5. **The clipboard and the Shelf (4 min).** Copy a sentence on the PC, press the Shelf key (next to the gear) → **Send clipboard**. Paste on the Mac. Then copy two files in Finder and press Send clipboard: "2 files on the clipboard · Send them to Desk PC" → press it. In a Firefox private window on the PC, copy a sentence and press Send clipboard: it says the app marked it as private; paste it into the Shelf's text box, tick **Private** (the text turns to dots) and press Send. **Expect, Mac:** the Shelf lists "Private text" with no preview, and ⌘V pastes it. Send a private sentence the other way too: on the PC, ⊞ Win+V (clipboard history, if it's on) doesn't list it, and **Copy** on the Shelf keeps it out too. Take one item off the Shelf with its ✕; **Clear Shelf…** asks first. *Send back:* anything that arrived when it shouldn't have, or didn't when it should, and whether Win+V showed the private one.
6. **Appearance and where files go (2 min).** Settings → Appearance → Dark, then System; the tray / menu bar has the same switch. Settings → "Received files go to" → **Open**. Tick **A folder for each device** and send a file: it lands in `SeamDrop/From MacBook Pro`.
7. **A third computer, if you have one (5 min).** See "3–4 computers" below: pair it, tick two devices in the strip, drop a file. **Expect:** it goes to both at once, each with its own progress row.

---

**Test data** (make once; none of it needs an administrator):
- Mac, 10 GB file: `dd if=/dev/urandom of=~/Desktop/big10g.bin bs=1m count=10240` (takes a minute or two).
- Mac, 5,000 small files: `mkdir -p ~/Desktop/many && for i in $(seq 1 5000); do head -c $((RANDOM % 4096 + 512)) /dev/urandom > ~/Desktop/many/file-$i.txt; done`
- Mac, awkward name: `cd ~/Desktop && touch 'Résumé: Q3?.pdf'` (Finder shows it as "Résumé/ Q3?.pdf").
- PC (PowerShell), 10 GB file: `fsutil file createnew $HOME\Desktop\big10g.bin 10737418240`
- PC (PowerShell), 5,000 small files: `mkdir $HOME\Desktop\many; 1..5000 | % { Set-Content "$HOME\Desktop\many\file-$_.txt" (Get-Random) }`

---

## 0. Moving from Portal to SeamDrop (Mission 03; do this first, once)
Start with Portal (the old build) installed and paired on both computers.
- [ ] **One computer first (say the Mac):** install SeamDrop (§1) but leave Portal installed on the PC. Open SeamDrop on the Mac.
- **Expect, Mac:** macOS asks again to **find devices on your local network**: SeamDrop is a new app to macOS. Click **Allow**. Details shows `migrate  settings, 1 paired device and keys brought over from Portal  old folder left at …`. If Portal used the default receive folder: `received files now go to …/Downloads/SeamDrop`. Settings shows the same device name and the PC under Paired devices. If Portal opened at login, SeamDrop's **Open at login** is on and Portal's is gone.
- [ ] Leave Portal open on the Mac too for a moment. **Expect:** Connection check on the Mac says **"The old Portal app is still open on this Mac"** with how to quit and remove it. Quit Portal, move it to the Trash; the line goes away within a minute.
- **Expect, Mac, with Portal still on the PC:** the PC shows up, and the window says **"Update SeamDrop on Desk PC"** ("It runs an older version that can’t talk to this one…"). Not "offline", not a silent wait. Portal on the PC shows its own "needs an update" screen.
- [ ] **Then the PC:** install SeamDrop on the PC (Portal can stay installed for now), open it, quit Portal from the tray, uninstall Portal in Settings → Apps.
- **Expect:** the two connect **without a pairing code**. A file sends both ways. "Files from …" in Settings is what it was in Portal, and a send still lands with the short arrival moment.
- [ ] Portal's old data folders are still there (Mac `~/Library/Application Support/com.project1000.portal`, PC `%APPDATA%\com.project1000.portal`). Delete them once you're happy.
- **If it asks for a pairing code anyway:** send both log files (paths above) and Portal's old folders' file list (not their contents).

## 1. Install from the CI builds
- [ ] **Mac (no administrator):** follow **Install a test build → macOS** in `README.md`: SeamDrop goes into the Applications folder *in your home folder*, then one Terminal command clears the download flag. Open it and click **Allow** when macOS asks about your local network.
- [ ] **PC:** run the installer (More info → Run anyway, accept the admin prompt).
- **Expect:** SeamDrop opens on both. On the PC, PowerShell `Get-NetFirewallRule -DisplayName SeamDrop` lists two rules (TCP and UDP), Enabled True, Action Allow, Profile Any.
- **If macOS refuses to open it** even after the Terminal command: send a screenshot of the message. (If Terminal itself is blocked on a managed Mac, tell me: there's no other way round without an administrator.)

## 2. They find each other within 5 seconds (PC on Ethernet, Mac on Wi-Fi; 5 out of 5)
- [ ] Quit SeamDrop on both (menu bar / tray → Quit SeamDrop). Open SeamDrop on the Mac, then on the PC, and count the seconds until the other computer's name appears at the top ("Desk PC · Not paired yet" before pairing, "· Connected" after). Repeat 5 times.
- **Expect:** ≤ 5 s every time. The pill at the top right says **Wi-Fi** on the Mac and **Ethernet** on the PC.
- [ ] Note in Details how it was found: `found … via mdns`, `via broadcast`, or (if your router passes neither between wired and Wi-Fi) SeamDrop asks every address directly: `asking every address on this network directly`, then `found … via broadcast`. Any of these is fine; tell me which you saw.
- **If not:** after 10 s the window shows "Still looking" with likely causes; screenshot it, then do test 3.

## 3. If the Mac's Local Network permission is off: Connect by IP from the PC
Do this once on purpose, before pairing (test 4), even if test 2 worked. With the permission off, the Mac can't reach out to anything on your network, but connections *into* the Mac still work. So the PC does the reaching.
- [ ] On the Mac: System Settings → Privacy & Security → **Local Network** → turn **SeamDrop off**. Quit and reopen SeamDrop on the Mac. Note the Mac's address: SeamDrop's Settings (gear) → "This computer's addresses" (e.g. `Wi-Fi 192.168.1.23`).
- **Expect, Mac:** it can't get anywhere by itself. It shows "Still looking" (the first line says macOS may be blocking SeamDrop and repeats the address), or it lists the PC but pairing from the Mac fails.
- [ ] On the PC: **Connect by IP** (on the "Looking" or "Still looking" screen) → type the Mac's address → **Connect**. Then press **Pair** on the PC; on the Mac press **Show code** and type that code on the PC (as in test 4).
- **Expect:** paired. Send a small file **both ways**: both work.
- [ ] Quit and reopen SeamDrop on the Mac (permission still off). **Expect:** within about 15 s the PC has reconnected by itself, because it dials the Mac's address.
- [ ] Turn SeamDrop back **on** in Local Network afterwards.

## 4. Pair by typing the code (Mission 04 §2), still paired after a restart
Two computers that aren't paired yet (Settings → Paired devices → **Remove** on both first, if they are).
- [ ] On the Mac click **Pair** next to the PC. **Expect, Mac:** "Asking Desk PC to pair": "Now go to Desk PC: it asks whether to pair. Press Show code there, then come back and type the 6 digits here.", and how long it waits. **Expect, PC:** "MacBook Pro wants to pair", "From 192.168.1.…", with **Show code** and **Ignore**, when the request lapses, and a notification if the window was in the background. No code anywhere yet.
- [ ] On the PC press **Show code**. **Expect, PC:** six digits in two groups, "Type this code on MacBook Pro", "The code works for 1:59 more" counting down. **Expect, Mac:** "Type the code shown on Desk PC" and a box with the cursor in it, the same countdown.
- [ ] On the Mac type the code **with one digit wrong** and press **Pair** (or Return). **Expect, Mac:** in amber under the box, "That's not it. Check the 6 digits on Desk PC: it's the same code. 2 tries left, then you'll need a new one.", and an empty box. **Expect, PC:** the same code still shown, "A wrong code was typed on MacBook Pro. 2 tries left. If that wasn't you, press Cancel: nothing gets paired.", Cancel now the main key.
- [ ] Type the right code. **Expect, both:** "Paired with …" and a notification "Paired with …"; then the drop zone with the other Connected. Details on each: "Paired with …" (key … as the detail).
- [ ] **Ignore:** remove the pairing again, press **Pair** on the Mac, and **Ignore** on the PC. **Expect, PC:** "Ignored MacBook Pro (192.168.1.…). Nothing was paired." at the bottom, and its nearby screen no longer pushes Pair. **Expect, Mac:** nothing changes for 2 minutes ("Asking Desk PC to pair"), then "No answer from Desk PC within 2 minutes. Nothing was paired." with **Pair again**. The Mac never learns that Ignore was pressed. Pressing **Pair** again within a minute of Ignore: the PC isn't asked at all.
- [ ] **A code nobody types:** Pair, Show code, then wait 2 minutes. **Expect, both:** "The code expired after 2 minutes. Nothing was paired." and what to do next.
- [ ] **Cancel** on either side while the code is shown: nothing is paired; the other side says "… cancelled pairing".
- [ ] **Three wrong codes** in one go: "… 3 times. Nothing was paired." on the Mac, "The code was typed wrong 3 times on MacBook Pro" on the PC.
- [ ] Quit SeamDrop on both, open both again.
- **Expect:** they reconnect by themselves (no code), Connected within a few seconds.

## 4b. Pairings made before the update keep working (Mission 04 §2)
- [ ] With 0.3.x installed and paired on both, update both to the Mission 04 build (test 1). **Expect:** they connect **without** pairing again; Settings → Paired devices lists each other as before.
- [ ] Update only one of them. **Expect:** that one says "Update SeamDrop on …: it runs an older version that can't talk to this one" (the versions can't talk until both are updated); nothing is unpaired.

## 5. 10 GB file and 5,000 files, both directions, verified
For each of the four transfers below: drop it on the sender's window (it goes into the send list), press **Send**, then click **Accept** on the receiver.
- [ ] Mac → PC: `big10g.bin`
- [ ] PC → Mac: `big10g.bin`
- [ ] Mac → PC: the `many` folder
- [ ] PC → Mac: the `many` folder
- **Expect:** progress with speed and time left; at the end "Received from …" and "Sent to …". Details on both sides shows `blake3 ok`. The receiver has `Downloads\SeamDrop\big10g.bin` (or `many` with 5,000 files; Windows: right-click → Properties shows 5,000 files). Nothing named `.seamdrop-incoming` is left inside `Downloads/SeamDrop`.
- Optional extra check: Mac `shasum -a 256 ~/Downloads/SeamDrop/big10g.bin` vs PC `Get-FileHash $HOME\Downloads\SeamDrop\big10g.bin`: same value as the original.

## 6. Network changes: it reconnects by itself, no code
Paired and Connected before each step. "Reconnects" means the top of the window says **Connected** again on both; Details shows `net …` lines explaining what happened.
- [ ] **Mac sleeps:** Apple menu → Sleep, wait 2 minutes, wake it. **Expect:** Connected again within about 15 s of waking (Details: `woke up after about 2 min`).
- [ ] **Mac switches Wi-Fi networks:** join another network (e.g. your phone's hotspot). The PC shows the Mac as not reachable. Join your home Wi-Fi again. **Expect:** Connected again within about 10 s of rejoining (Details: `network changed  Wi-Fi … → Wi-Fi …`).
- [ ] **The PC gets a new address:** restart your router (both computers usually get new addresses), or on the PC run `ipconfig /release` then `ipconfig /renew` in an administrator Command Prompt. **Expect:** Connected again within about 30 s of the network being back. Paired the whole time: no code asked.
- [ ] **Unplug the PC's network lead from the router** for a minute, then plug it back in. **Expect:** same as above.

## 7. Transfers pause and continue by themselves (Mission 03)
Use `big10g.bin` (10 GB) so there's time. Each case: when it's done, check Details shows `blake3 ok` on both sides and nothing named `.seamdrop-incoming` is left in the receive folder. Optional exact check: `shasum -a 256` on the Mac vs `Get-FileHash` on the PC.
- [ ] **Wi-Fi off for 30 seconds.** Send `big10g.bin` from the PC to the Mac. At about 30 %, turn the Mac's **Wi-Fi off**, count to 30, turn it back on.
- **Expect, both:** "Connection interrupted. Waiting for it to come back…" under the bar, then (within about 30 s) **"Paused: lost connection to Desk PC"** / **"…to MacBook Pro"**, "Will continue when it’s back.", with what's done so far (e.g. "3.1 GB of 10.7 GB · Paused"). The tray/menu bar says "Paused at 30% · waiting for …". When Wi-Fi is back: connected again by itself, briefly "MacBook Pro is back · Checking the … that already arrived" (the bar says Checking…), then the bar carries on **from about 30 %, not from 0**, with no new question on the Mac. Details: `pause …`, then `resume … continuing from 30 %`.
- [ ] **Close the lid.** Send from the Mac to the PC; at about 40 % close the Mac's lid for a minute, then open it. **Expect:** the PC shows Paused within about 30 s; after the Mac wakes, both carry on from about 40 % by themselves.
- [ ] **Quit the app.** Send from the Mac to the PC; at about 50 %, quit SeamDrop on the **PC** (tray → Quit SeamDrop). **Expect, Mac:** Paused. Open SeamDrop on the PC again. **Expect:** the PC starts with the paused transfer showing, and it carries on from about 50 %. Repeat with quitting on the **Mac** (the sender).
- [ ] **Restart a computer.** As above, but restart the PC at about 50 %. **Expect:** after logging in and opening SeamDrop (or automatically, with Open at login), it carries on.
- [ ] **A file changed while paused.** Send a folder with a few big files from the Mac; pause it (quit SeamDrop on the PC). On the Mac, edit (save) the file that was in the middle. Open SeamDrop on the PC. **Expect:** it carries on; the finished screen on the Mac says "“…” changed on this Mac while the transfer was paused, so it was sent again from the start." The PC gets the new version.
- [ ] **Cancel while paused.** Pause a transfer (Wi-Fi off on the Mac). On the PC press **Cancel**. **Expect, PC:** it asks first: "Stop receiving? What has arrived from MacBook Pro is removed…" with **Keep waiting** and **Stop receiving**. **Keep waiting** changes nothing. Press Cancel again, then **Stop receiving**. **Expect, PC:** "Cancelled · You cancelled it. What had arrived from MacBook Pro was removed.", and nothing partial is in Downloads. Turn the Mac's Wi-Fi on. **Expect, Mac:** "Desk PC cancelled. Your files are still on this Mac; nothing was saved on Desk PC.", with **Send again**.
- [ ] **Send again.** On the Mac's stopped screen press **Send again**. **Expect:** the same items are offered to the PC again, without picking them.
- [ ] **Cancel while sending.** During a transfer, press **Cancel** on the Mac. **Expect:** "Stop sending? Your files stay on this Mac; the part that arrived on Desk PC is removed." **Keep sending** carries on; **Stop sending** ends it with "Cancelled" and **Send again**.
- [ ] **Paused too long (optional, takes a day).** Pause a transfer and leave both computers on for 24 hours without reconnecting them. **Expect:** "Paused for more than 24 hours, so SeamDrop …" on both, and the partial data removed on the receiver.
- **If it starts again from 0 %,** or asks again, send both log files and say which case.

## 7b. Updates (Mission 03)
The public releases repo: https://github.com/andrew2grand-lang/seamdrop-releases (installers on the Releases page).
- [ ] **An older build updates itself.** Install the previous release (e.g. 0.3.0) on the Mac from the releases page (README steps). Publish a newer one (docs/RELEASING.md). Open SeamDrop on the Mac and wait (or Settings → About → **Check for updates**).
- **Expect:** within a minute of checking, a calm row under the header: **"SeamDrop 0.3.1 is ready"** with "What's new: …", **Restart SeamDrop to update** and **Later**. The menu bar menu says "Restart SeamDrop to update to 0.3.1". Press **Restart SeamDrop to update**: SeamDrop closes and opens again by itself; Settings → About says **SeamDrop 0.3.1**. Your pairing and settings are as before.
- **Known until the app is signed (docs/SIGNING.md):** macOS may ask again to find devices on your local network after the update, because an unsigned app's new version looks like a new app to it. Click **Allow**.
- [ ] Same on the PC. **Expect:** the administrator prompt (it's installed per machine), a short installer progress window, then SeamDrop 0.3.1 opens.
- [ ] **Never during a transfer.** With an update ready, start sending `big10g.bin` and press **Restart SeamDrop to update**. **Expect:** "SeamDrop restarts to update when the transfer finishes" (no restart); when the transfer ends, SeamDrop restarts into the new version. **Not now** cancels that.
- [ ] **Later.** Press **Later**. **Expect:** the row goes away and stays away for that version; Settings → About still offers **Restart SeamDrop to update**.
- [ ] **Offline.** Turn Wi-Fi off, Settings → About → Check for updates. **Expect:** "Couldn't reach the update server. Check the internet connection; SeamDrop tries again later." Nothing else changes.
- [ ] **Install updates automatically off.** Untick it, Check for updates with a newer release out. **Expect:** "SeamDrop X is available" with **Update and restart**; nothing was downloaded before you pressed it.

## 7c. Shared clipboard (Mission 03)
- [ ] **Text, Mac → PC.** On the Mac copy a sentence from Notes, then press **Send clipboard** in SeamDrop (or ⌃⌥⌘C anywhere). **Expect, PC:** a notification "Clipboard from MacBook Pro" / "“…” — ready to paste."; Ctrl+V in Notepad pastes it. The PC's window also says "Clipboard from MacBook Pro: “…” — ready to paste with Ctrl+V." The Mac says "Sent to Desk PC (text, … bytes). Paste it there with Ctrl+V." Details on both: `clipboard … text, … bytes`, never the words themselves.
- [ ] **Text, PC → Mac** with Ctrl+Alt+Shift+C (from any app). Same result on the Mac.
- [ ] **Rich text.** Copy a bold, coloured line from Word or a web page on the PC, send, paste into Pages or TextEdit on the Mac. **Expect:** the formatting comes along. Paste into a plain-text field: the plain words.
- [ ] **Image.** Take a screenshot to the clipboard (Mac: ⌃⇧⌘4; PC: Win+Shift+S), send, paste into Paint / Preview → New from Clipboard. **Expect:** the same picture.
- [ ] **A password never arrives.** In 1Password, Bitwarden, KeePassXC or Apple Passwords, copy a password; press Send clipboard. **Expect:** "The app you copied this from marked it as private, so it wasn't sent. To send it anyway, paste it into the Shelf's text box." and nothing arrives on the other computer. Details says who marked it where SeamDrop can tell ("Not sent: 1Password marked it as private, as password managers do"), with the marker's name after it. With automatic sync on, copy a password: nothing arrives either. (Tell me which password manager you used; ones that don't mark their copies can't be detected: SeamDrop never guesses.)
- [ ] **A private browser window (Mission 04 §4.3).** On the PC, copy a sentence in a Firefox **private window** (or Tor Browser), press Send clipboard. **Expect:** the same "marked it as private" message; Details: "Not sent: Firefox marked this copy as private, as it does in private windows (Tor Browser always does)". The same sentence copied in a normal Firefox window goes through. Then paste the private copy into the **Shelf's text box** and press Send: it arrives.
- [ ] **Empty / files.** Clear the clipboard (copy nothing) and press Send clipboard: "The clipboard is empty. Copy something first." Copy two files in Finder or Explorer and press **Send clipboard** in the window: "2 files on the clipboard · **Send them to Desk PC**"; press it: an ordinary file offer arrives. Copy files and press the **shortcut** instead: they're sent straight away ("Sending 2 files from the clipboard to Desk PC").
- [ ] **The Shelf's text box (Mission 04 §4.1).** Open the Shelf, type a line with an emoji and some other script (日本語), press **Send** (or Return). **Expect:** it lands on the other computer's Shelf, and on its clipboard unless that computer set this one to **Shelf only**. This computer's own clipboard is untouched.
- [ ] **Stress, by hand (Mission 04 §4.5).** With automatic sync on: copy ten things quickly one after another: the other computer ends with the last one and nothing bounces back. Copy rich text from Word or Pages, and an image from a web page: they arrive whole. Send clipboard while a big file is moving: it arrives at once and the file carries on. Quit the app you copied from, then paste on the other computer: still there. Put the other computer to sleep and copy: nothing is lost on this one; it says the other isn't connected if you press Send clipboard.
- [ ] **Huge.** Copy a 40 MB screenshot or a very long text and send. **Expect:** "…too big for a clipboard, so it’s being sent to Desk PC as a file instead (Clipboard image.png)", and the other computer gets an ordinary file offer.
- [ ] **Automatic sync needs both computers.** Settings → Clipboard → **Sync the clipboard automatically** on the Mac. **Expect, Mac:** a pill "Sync: waiting for Desk PC"; nothing syncs yet (copy on the Mac: nothing arrives on the PC). **Expect, PC:** a row "MacBook Pro asks to sync the clipboard" with **Turn on** and **Not now**, and a notification saying the same. Press **Not now**: off on both (the Mac's box is unticked). Ask again from the Mac and press **Turn on** on the PC: both windows show **Clipboard sync on**, the tray says "Clipboard sync on". Copy on the Mac, paste on the PC within about a second; then the other way. Nothing bounces back (the received list grows by one per copy). Turn it off on the PC: off on both.
- [ ] **Offline.** Quit SeamDrop on the PC and press Send clipboard on the Mac: "Desk PC isn’t connected right now."
- [ ] **Shortcut.** Settings → Clipboard → Change… and press a new combination; it works from any app. Try one another app uses: SeamDrop says so and keeps the old one.
- [ ] **History.** The **received** button under the drop area (e.g. "4 received") opens "Clipboard from …": Copy puts an item back, **Clear list** empties it. Quit and reopen: the list is empty (it's kept in memory only).

## 8. "Files from …": Ask every time, Always accept, Never (Mission 02)
There are no Send / Both / Receive keys any more. Each computer decides, per paired device, what happens to files from it: Settings (gear) → Paired devices → **Files from MacBook Pro**.
- [ ] **Updating from Mission 01:** before installing the new build, note the PC's old settings (its role key, and whether "Always accept" was ticked). After updating, open Settings on the PC. **Expect:** "Always accept" ticked before → **Always accept**; the PC set to **Send** before → **Never**; otherwise **Ask every time**. Same on the Mac.
- [ ] **Ask every time** (the default) on the PC. On the Mac drop a file and press **Send**. **Expect:** the PC asks ("MacBook Pro wants to send"); Accept works.
- [ ] Set it to **Never** on the PC. On the Mac drop a file and press **Send**. **Expect:** the Mac says **"Desk PC doesn’t accept files from this Mac"** and "On Desk PC, open Settings and set “Files from MacBook Pro” to Ask every time or Always accept. Nothing was sent; it sends as soon as that changes." The PC shows nothing new (Details on the PC may say `refuse … set to Never`). Nothing arrives in Downloads/SeamDrop on the PC.
- [ ] Still on the PC, switch back to **Ask every time**. **Expect:** the offer arrives on the PC by itself within a couple of seconds.
- [ ] Set it to **Always accept**, send again. **Expect:** no question on the PC; "Accepted automatically" while it runs.
- [ ] Never only affects files coming *in*: with the PC on Never for the Mac, send from the PC to the Mac. **Expect:** it works as usual.
- [ ] Quit SeamDrop on the PC and open it again. **Expect:** the setting is as you left it.
- [ ] Also try **"Always accept from MacBook Pro"** on the PC's accept screen: ticking it there and pressing Accept changes the setting to Always accept.

## 8b. A tiny file still arrives visibly (Mission 02)
- [ ] With the PC on **Always accept**, send a very small file (a few KB) from the Mac.
- **Expect, PC:** no progress bar flashing by. For about a second the window says "Receiving from MacBook Pro" with the orange light filling the window edge, then "Received from MacBook Pro" with the arrival flash. **Mac:** the blue light runs round the edge for about a second, then "Sent to Desk PC".
- [ ] Same with **Reduce motion** on (macOS: System Settings → Accessibility → Display; Windows: Settings → Accessibility → Visual effects → Animation effects off). **Expect:** the same order, with a still lit edge instead of moving light, and a gentle pulse when it lands.
- [ ] A big file still shows the progress bar after a moment, as before.

## 9. Windows network set to Public
- [ ] On the PC: Settings → Network & internet → Ethernet → your network → **Public network**. Restart SeamDrop on the PC.
- [ ] Open Settings (gear) → **Connection check** on the PC.
- **Expect:** a line like *"Network" is a Public network, and SeamDrop is allowed there*, plus Windows Firewall allows SeamDrop. Sending both ways still works.
- [ ] Optional, to see the failure path: Windows Security → Firewall & network protection → Advanced settings → Inbound Rules → right-click **SeamDrop** (both) → Disable. The connection check should say Windows Firewall has no rule for SeamDrop, with a **Fix it** button, and the PC still reaches the Mac (it connects out). Press **Fix it** and click **Yes** on the Windows question. **Expect:** "Fixed. Windows Firewall allows SeamDrop again." and, within about 15 s, the check passes. (Fix it replaces the disabled rules with new ones.)
- [ ] **A Block rule (from pressing Cancel on Windows' question).** In PowerShell as administrator: `New-NetFirewallRule -DisplayName "SeamDrop block test" -Direction Inbound -Action Block -Program "C:\Program Files\SeamDrop\seamdrop.exe"` (use the path SeamDrop is installed at). **Expect:** the connection check shows "A Windows Firewall rule blocks SeamDrop" with **Fix it**. Press it, answer **No** on the Windows question: "Nothing changed: the administrator question was answered No." Press it again and answer **Yes**: "Fixed…", and the block rule is gone (`Get-NetFirewallRule -DisplayName "SeamDrop block test"` finds nothing).

## 10. Window closed: notification, then the accept window
- [ ] On the PC close SeamDrop's window (the ✕). It stays in the tray. From the Mac, send a file.
- **Expect:** a Windows notification "MacBook Pro wants to send"; the tray icon gets an orange dot. Clicking the notification opens a small window with **Accept / Decline**. (Notifications only work in the installed build.)
- [ ] Same on the Mac: close the window (it stays in the menu bar), send from the PC. A macOS notification appears (allow notifications if asked); clicking it opens the accept window. The menu bar item also has **Review incoming files…**.

## 11. An awkward name arrives safely
- [ ] Send `Résumé: Q3?.pdf` from the Mac to the PC.
- **Expect:** on the PC it's saved as **`Résumé_ Q3_.pdf`**. The done screen says "1 name changed so it works on Windows." Details on the PC shows `rename Résumé: Q3?.pdf → Résumé_ Q3_.pdf  ‘:’ and ‘?’ aren’t allowed in Windows names`.

## 12. The send list: collect, then send in one go
- [ ] On the Mac, collect things from different places without sending: drop two files from the Desktop, press **Add a folder** and pick a folder from Documents, drop another folder, and drop a file **with the same name** as one already in the list but from another folder (e.g. two different `notes.txt`).
- **Expect:** nothing is sent yet. "Ready to send" lists each item with its size (folders show "Counting…" for a moment, then their file count), and the total under the title. A shortcut/alias shows "Can't be sent" and doesn't count.
- [ ] Remove one item with its ✕. Press **Send**.
- **Expect:** the PC gets **one** offer listing everything; after Accept, everything arrives together. The second `notes.txt` arrives as `notes (2).txt` (nothing overwritten, nothing merged).
- [ ] While a big transfer runs, drop more files on the sender's window. **Expect:** "1 more item waiting in the send list for when this is done." When it's done and you press OK, the list is there with those items, ready for **Send**.
- [ ] **Clear** empties the list without sending anything.

## 13. Disconnect and Connect (stays paired)
- [ ] Paired and Connected. On the Mac open Settings → Paired devices → **Disconnect**.
- **Expect, Mac:** "Disconnected from Desk PC" with a **Connect** key; nothing moves or searches. **Expect, PC:** after a moment it isn't connected, and its Connection check says "MacBook Pro disconnected from this computer" with "On MacBook Pro, press Connect".
- [ ] Quit and reopen SeamDrop on the Mac. **Expect:** still "Disconnected", no code, no searching.
- [ ] Press **Connect**. **Expect:** Connected again on both within a few seconds, no code.
- [ ] Turn the PC off, wait for "Desk PC isn't reachable" on the Mac, press **Disconnect** there. **Expect:** the Mac stops looking (no moving parts), and Connect brings it back once the PC is on.

## 14. Speed on Wi-Fi, against LocalSend
Same setup as always (PC on Ethernet, Mac on Wi-Fi), the Mac in the same spot every run, nothing else downloading. After each 10 GB transfer, Details shows a line like `speed  middle 80 %: 61.4 MB/s over 133.5 s` (the brief's measure: the middle 80 % of the transfer). Note the receiver's number.
- [ ] Mac → PC: alternate SeamDrop and LocalSend with `big10g.bin`, 3 runs each. For LocalSend, time it with a stopwatch from accept to done and divide 10,240 MB by the seconds.
- [ ] PC → Mac: the same.
- **Target:** in each direction, SeamDrop's median ≥ 90 % of LocalSend's.
- [ ] The 5,000-file folder: note the time. (There's no round trip per file: in the automated tests, 2,000 files over a link with a 50 ms round trip take under 10 s; one round trip each would take 100 s.)

| Run | Mac → PC SeamDrop MB/s | Mac → PC LocalSend MB/s | PC → Mac SeamDrop MB/s | PC → Mac LocalSend MB/s |
|---|---|---|---|---|
| 1 | | | | |
| 2 | | | | |
| 3 | | | | |
| median | | | | |

## 15. 3–4 computers (Mission 04 §3)
Any mix of Macs and PCs on the same home network, all updated, all paired with each other (pair each new one from a computer that already has the others: Settings → Paired devices → **Pair another computer**).
- [ ] **The window with several devices.** **Expect:** the header says "3 devices · 3 connected", and a strip lists each device with a tick and its status (blue dot Connected, ring Connecting, orange Not reachable). Untick one: the drop zone says where files go now ("They go to Desk PC and MacBook Air"). Quit SeamDrop on one: its status changes there within a few seconds; nothing else moves.
- [ ] **Send to two at once.** Tick two devices and drop a 1 GB file. **Expect:** two rows, one per device, each with its own bar, percentage and time left; both finish; each receiver asks first (unless set to Always accept).
- [ ] **One at a time per device.** While a transfer to Desk PC runs, send something else to Desk PC. **Expect:** "Desk PC is busy with another transfer. Send this when it's done (one at a time per device)." Sending to another device works at the same time.
- [ ] **Offers from two devices at once.** Send to the same computer from two others at about the same time. **Expect:** the accept window shows one, "1 of 2: the next one shows when you answer this one"; Return accepts, Esc declines.
- [ ] **A drop with two transfers paused.** Two transfers running to two devices; turn this computer's Wi-Fi off for 20 s. **Expect:** both pause and both carry on when it's back. Quit and reopen SeamDrop while both are paused: both are still there, paused, and carry on.
- [ ] **The clipboard with three.** Turn on clipboard sync with two of them (each has to agree). Copy on one: it arrives on the other two once; nothing bounces around. **Pause clipboard sync** from the tray (1 hour): nothing goes either way from this computer; the others still sync with each other.
- [ ] **Remove one.** Settings → Paired devices → **Remove** on one. **Expect:** it says what that means first; afterwards that device says this computer removed it, and the others are unaffected.

