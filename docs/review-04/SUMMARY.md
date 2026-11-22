# Review 04: summary

Four reviewers looked at SeamDrop with fresh eyes (Mission 04 §11). Each was told only "SeamDrop moves files and your clipboard between your computers." and could read the user part of the README. They used the real screens in a browser, against simulated other computers (`http://localhost:5180/?mock=…`), at 552×780, 420×640 and 1280×900, in the Mac and Windows windows, light and dark.

| Reviewer | Report | Blocks use | Confuses | Annoys | Polish |
|---|---|---|---|---|---|
| A beginner (first pairing, one Mac and one PC in different rooms) | [beginner.md](beginner.md) | 0 | 9 | 3 | 3 |
| Four computers at home | [four-computers.md](four-computers.md) | 2 | 6 | 4 | 2 |
| Privacy-minded | [privacy.md](privacy.md) | 0 | 13 | 2 | 1 |
| Developer moving big files | [developer.md](developer.md) | 1 | 10 | 7 | 7 |

Screenshots are in `docs/review-04/shots/` on this Mac only (ignored by git, as in review 03).

Below, the same problem seen by several reviewers is one row. **Real** means the app does it; **simulation** means only the simulated other computer did, and the simulation was fixed too so the next review isn't misled (decision 237). Every fix here was checked again afterwards: the unit and core tests named, and the layout check over every state, size, theme and window (`scripts/overlap.mjs`).

## Blocks use: 3, all fixed

| # | What | Who | Real? | What changed | Checked by |
|---|---|---|---|---|---|
| B1 | Can't find out which items were left out or renamed; the sender never learns which name changed | developer 1 | Real | Done screens list them under "Show the 2 items left out" / "Show the 1 changed name", on both computers; the receiver sends its renames back; Details lists them on both sides (decision 217) | `tests/transfer.rs` sending_skips…, nasty_names…; `buildWorld.test.ts` |
| B2 | No way to send more while transfers run: dropped files went on a list nobody could see | four computers 2 | Real | The send list shows over the transfers until it's sent or cleared; busy, asleep and still-connecting computers are named and left out; transfer screens offer "Send something else" (decision 216) | `buildWorld.test.ts` sending more while transfers run |
| B3 | Pairing a fifth computer by IP left only one device; an already-paired computer showed as "Not paired yet" | four computers 1, developer 8 | Simulation (the core never replaces devices; it also rejected `192.168.1.300`) | The simulation keeps devices and validates like the core. In the app: an already-paired address says so; a new computer opens Pair another computer with its address (decision 236) | layout check; manual check step 4 |

## Confuses: all fixed

| # | What | Who | Real? | What changed |
|---|---|---|---|---|
| C1 | "Paused: lost connection" while the header and tile say "Connected" | beginner 4, developer 2, four computers 5 | Simulation (the core marks the device as connecting or unreachable); the app now also says "is back" if both are true | Decision 220; simulation fixed |
| C2 | Stuck at 100 % with "A few seconds" and Cancel | beginner 5, developer 4 | Simulation never finished; the app said "A few seconds" while the other side checks the last of it | "Finishing…" at 100 % (decision 218); the simulation finishes |
| C3 | The finished speed counts bytes that were already there | developer 3 | Real | Only what crossed the wire counts (decision 218); `tests/resume.rs` finished_files…, `format.test.ts` |
| C4 | The current file never changes | developer 5 | Simulation | It moves now |
| C5 | Details out of order, contradicting the screen; says "Connected" to an unpaired computer; misses losing a device | developer 6, beginner 10, privacy F7, four computers (notes) | Mostly the simulation's fixed lines; real: "Connected" for unpaired, and no line when a device became unreachable | Decision 221 (`tests/transfer.rs` details_doesnt_call…, `tests/reach.rs`); the simulation's lines are in time order and match each screen |
| C6 | Connection check blames the network or Local Network access while devices are connected; an asleep PC reads as a macOS permission problem | developer 7, beginner 3 | Real (an asleep computer answers "no route to host" too) | Decision 222 (`tests/doctor.rs`) |
| C7 | Rename messages contradict each other; the sender shows names it doesn't have | developer 9 | Simulation (the core words both sides by the receiver's rules) | The simulation's renames follow the receiving OS |
| C8 | "2 transfers under way" when none move, or four are listed | developer 10, four computers 4 | Real | Decision 219 (`buildWorld.test.ts`) |
| C9 | Restart to update says nothing about a paused transfer | developer 11 | Real | Decision 233 (`updates.test.ts`) |
| C10 | Stop on a paused row leaves "Stop sending?" on a cancelled row | four computers 3 | Real | Decision 219 |
| C11 | Two offers at once: less detail, buttons swap places | four computers 6 | Real | Same key order as the single screens; the row says where it would be saved (decision 219). The full offer (file list, Always accept) stays on the single screen: listed under "annoys" below |
| C12 | "Doesn't get these now" leaves open whether it gets them later; a connecting computer is treated as off | four computers 7 | Real | Decision 216 |
| C13 | At the smallest size with six devices, the grid crowds out the lists; stacked keys cover them | four computers 8, developer 17 | Real | Decision 229 |
| C14 | No "Add files" until something is added | beginner 1 | Real | Decision 235 |
| C15 | The pairing hand-off between rooms isn't told; "check a code on both screens" isn't what happens | beginner 2, privacy F16 | Real | Decision 223 |
| C16 | "Isn't reachable" leads with permissions, IP numbers and VPNs | beginner 3 | Real | Decisions 222, 235 |
| C17 | Clipboard results are tiny and look the same for success and failure | beginner 6, four computers 11 | Real | Decision 230 |
| C18 | The Shelf mixes sending and receiving; the biggest key sends | beginner 7 | Real | Decision 227 |
| C19 | The small Shelf hides its list; Clear doesn't ask | beginner 8, four computers 10, privacy F9 | Real | Decisions 227, 229 |
| C20 | After Ignore, the app invites pairing with the computer just refused | privacy F1 | Real | Decision 225 |
| C21 | The lockout after wrong codes is hidden in Settings, without who or when | privacy F2 | Real | Decision 224 (`tests/pairing.rs` wrong_codes_from_anyone…) |
| C22 | The computer showing the code doesn't say to cancel if it wasn't you | privacy F3 | Real | Decision 223 |
| C23 | Codes and requests never seem to expire | privacy F4 | Simulation never expired them (the core did after 2 minutes, silently on screen) | A countdown on every step and an "expired" message (decision 223; `tests/pairing.rs` a_code_nobody_types…) |
| C24 | Can't tell two computers with the same name apart when pairing | privacy F5 | Real | Addresses in the nearby list and screen (decision 236) |
| C25 | The Name hint understates who sees the name | privacy F6 | Real | Decision 234 |
| C26 | Private text typed into the Shelf shows in plain text and arrives with a preview | privacy F8 | Real | Decision 226 (`tests/clipboard.rs` text_sent_as_private…; real macOS pasteboard here; real Windows clipboard in CI) |
| C27 | "Not kept (too big)" contradicts the 25 MB image limit | privacy F10 | Simulation's numbers; the app's wording didn't say why | Decision 227 |
| C28 | Whether clipboard history keeps arrivals isn't said | privacy F11 | Real | Said on the Shelf and in Settings; private text is marked (decision 226) |
| C29 | After Remove, the Shelf's items from that computer are left in memory | privacy F12 | Real | Decision 227 (`tests/clipboard.rs` one_item_can_be_removed…) |
| C30 | Whether private browser windows are held back can't be checked | privacy F15 | Real | The Shelf says which browsers mark private windows on Windows (decision 214) and that others may not; the refusal now shows with a warning sign (decision 230) |
| C31 | The simulation accepted wrong codes whose digits add up to 18 | beginner (notes) | Simulation (the core's key exchange fails on any other code; `tests/pairing_code.rs`, `tests/pairing.rs`) | The simulation takes only 048 213 |

## Annoys: some fixed on the way, the rest for later

Fixed: Settings' ✕ over Remove, the Shelf's ✕ over Copy (decision 228); Turn off sync cut off (wraps); "Send feedback" saving a file without saying so (decision 232); the tiles not saying what each computer does (decision 219); "Pair another computer" buried in Settings (decision 234); a wrong code's quiet message and the dead end without Pair again (decision 223); the Shelf's sync sentence repeated per device (decision 227).

Left for a later mission (listed in PROGRESS under "Next mission"; nothing is needed from the owner for them):

| What | Who |
|---|---|
| Disk full: check free space when the offer arrives; pause instead of failing; say how much is needed | developer 12, beginner 11 |
| No Pause key; Cancel doesn't say how much it throws away; Escape doesn't choose Keep | developer 13 |
| A stalled transfer gives no sense of time ("No data for 0:42 · attempt 3"); the 24-hour deadline as a time | developer 14 |
| Rows in the list show no speed and can't be opened; a finished row loses its notes; two offers lose Always accept | developer 15, four computers 6 |
| A not-reachable device in a several-device window has no "Why?" | developer 16 |
| Settings: one long block per device (could be a row that expands); Disconnect shown for an unreachable device | four computers 12 |
| The sync pill above the grid moves it down when sync turns on | four computers 10 |

## Polish: for later

Version numbers on the "older version" screen and in Details (developer 22); the address on the connecting screen (developer 23); the link pill's speed and address on hover (developer 20); mixed units (developer 21); the wide window's empty space (developer 24); "Sending"/"Queued" read on the receiving side by screen readers (developer 25); the resume check's progress (developer 19); device icons by kind, not OS (four computers 13); HEIC photos on Windows (beginner 14); "Skip system files like .DS_Store and Thumbs.db" (beginner 13).

## What all four said works

- Showing the code on the other computer: big digits, "works once", and "Show a code only if you're pairing this computer with … yourself".
- The paused screen's "Is the PC asleep, or SeamDrop closed there? It picks up when it wakes or opens", and the resume story in Details.
- Cancel and Remove ask first, say what they remove, and make the safe choice the main key.
- The grid with a tick per computer, the sentence that names where files go, and the asleep laptop skipped without an error.
- The first "Received" screen with Show in Finder / File Explorer and Change folder.
- Honest time left ("Working out time left…", then coarse words that matched the arithmetic).
