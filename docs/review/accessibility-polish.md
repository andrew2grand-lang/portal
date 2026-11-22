# Accessibility and visual polish review: Portal, Project Send

Reviewed 1 November 2026 against the design lab at `localhost:5190`. I covered all 34 states (including a real scan) in both window styles and both themes. I made no changes to the product.

## Overview

The base is solid: every control is a named native button or input, Tab order matches the visual order in both window styles (Windows puts the primary button first, and the DOM follows), expandable items expose `aria-expanded`, the conversion stepper gives its state as text, the OS reduced-motion setting is fully respected, nothing scrolls sideways down to 320 px, and body text contrast is high.

The problems come in three groups:

1. **Losing your place.** Focus falls back to the page on every screen change, and nothing announces the new screen, including the consent stops where a conversion waits for an answer. Progress is silent (conversion) or a burst of 30 file counts (scan).
2. **Real data breaks the layout.** Long paths are cut off, "and 24 more" can't be opened, and one long file name squeezes a finding title to one word per line.
3. **A few low-contrast tokens**, plus scroll panes and code blocks that take focus without showing it.

The macOS and Windows styles behave the same for keyboard, focus, contrast and layout; findings apply to both unless noted.

## WCAG 2.1 AA

| Criterion | Result | Evidence |
|---|---|---|
| 1.1.1 Non-text content | Pass | Icon-only buttons have names ("Settings"; the sheet close button, though it is misnamed, see 4.1.2). Decorative icons are `aria-hidden`. |
| 1.3.1 Info and relationships | Partial | Scan step state (done, active, pending) is shown only by an icon. Eight visual subheadings are `<p>`. The Account sheet goes from the lab's h1 straight to h3. |
| 1.3.2 Meaningful sequence | Pass | DOM order matches visual order in both styles, including the mirrored Windows button order. |
| 1.4.1 Use of color | Pass | Status icons always come with words ("Needs you", "Proved", "Couldn't check"). Diffs use −/+ and strikethrough as well as color. |
| 1.4.3 Contrast (minimum) | **Fail** | Tertiary text 2.58–2.95:1 (light). Undone change 1.84–2.58:1. Details peek 4.13–4.18:1. Dark tertiary text 4.32:1. Finding 6, Finding 7. |
| 1.4.10 Reflow | Pass with exceptions | No horizontal page scroll at 320, 380, 520 or 760 px. Code scrolls inside its own block, which is allowed. Long names without spaces are clipped at every width (Finding 4). |
| 1.4.11 Non-text contrast | **Fail** | Pending step circles 1.33:1 (light), 1.41:1 (dark). Focus ring 2.76:1 in light (5.30:1 in dark). Field borders 5.1:1 and status icons 5.9–17:1 pass. |
| 1.4.12 Text spacing | Pass | With WCAG spacing applied, no text clips. The header icon drops below the device name. |
| 2.1.1 Keyboard | Pass with gaps | Every action can be reached with Tab, Enter and Space, and arrow keys scroll the panes. Step summaries appear only in mouse tooltips. Esc doesn't close the account sheet. |
| 2.1.2 No keyboard trap | Pass | Tab leaves the window in all 34 states and in both styles. |
| 2.2.2 Pause, stop, hide | Pass | The only looping motion is progress spinners while work is running. |
| 2.3.1 Three flashes | Pass | Nothing flashes. The consent halo is a slow glow. |
| 2.3.3 Animation from interactions (AAA, requested) | Pass (OS setting) | With `prefers-reduced-motion`, only opacity fades remain, spinners become a slow fade, and the halo stops. The lab switch is only partly equivalent (lab notes). |
| 2.4.3 Focus order | **Fail** | Focus falls to `<body>` on 14 of 14 screen changes tested (Finding 2). |
| 2.4.6 Headings and labels | Partial | The close button is named "Clear". Seven buttons are named just "Undo". |
| 2.4.7 Focus visible | **Fail** | Code diff blocks (0 px changed when focused) and the conversion pane (Finding 5). Every other control shows a ring in both themes. |
| 3.2.1 / 3.2.2 On focus, on input | **Fail** | Pressing an arrow key in the Mode switch replaces the report with the drop screen (Finding 8). |
| 3.3.2 Labels or instructions | Pass | Email and API key fields have visible, linked labels. "Email me a link" is disabled without saying why. |
| 4.1.2 Name, role, value | Partial | Radios, expandable items (`aria-expanded`, `aria-controls`) and meters (`aria-valuenow`) are correct. Close is named "Clear". "Show all 4" has no expanded state. In the Mode radio group, selection doesn't follow focus. |
| 4.1.3 Status messages | **Fail** | The scan list is the only live region, and it is noisy. Conversion progress, consent stops, "Hand-off note saved" and Undo say nothing (Finding 3). |

## Findings

Severity scale: blocks use > confuses > annoys > polish.

### 1. Long paths are cut off, and most places can't be opened at all
**Blocks use** · Blocker report (real scan) · Mac and Windows · light and dark

![Opened finding with 30 places, every row cut off identically](shots/accessibility-polish-01-long-paths-clipped.png)

- **What happens:** I scanned a test project with 20 findings and about 250 places, through "Scan a real folder". Opening "Path longer than Windows likes (30 places)" shows 6 rows. Each one reads `src/very_long_directory_name_that_keeps_going_and_going/anot`: the row is 404 px wide, the text is 1,524 px, and the overflow is clipped. There is no ellipsis, no tooltip, no wrapping and no horizontal scroll, so the file name, the part that matters, never shows. "and 24 more" is plain text, not a control. Every finding with more than 6 places behaves this way. Screen readers do get the full path text.
- **Expected:** Every affected file can be identified and reached from the report.
- **Suggested fix:** Truncate in the middle and keep the file name (`src/…/module_…_number_10.py:10`), with the full path on focus or hover and a copy action, or wrap with `overflow-wrap:anywhere`. Make "and 24 more" a "Show all 30" button that expands in place. Consider grouping places by folder.

### 2. Focus is lost on every screen change, and new screens aren't announced
**Confuses** (keyboard and screen reader) · all flows · Mac and Windows · light and dark

![Consent stop reached by keyboard: nothing has focus](shots/accessibility-polish-03-consent-stop-no-focus.png)

- **What happens:** After Enter on any of the 14 screen-changing controls I tested (Convert with Pro, Stop, Run it, Skip this check, Send 4 files, Review the 7 changes, Done reviewing, See the report, Got it, Choose, Choose another folder, Finish without it, the drop zone, sheet close), `document.activeElement` is `<body>`. The next Tab restarts at the top, and nothing announces the new screen. This hurts most at consent stops, which appear mid-conversion and wait for an answer. In-place changes keep focus correctly (Export, Undo, Show all).
- **Expected:** Focus moves to the new screen's heading or to its first decision, and the screen's name is announced.
- **Suggested fix:** On each screen change, give the `h2` `tabindex="-1"` and move focus to it. For consent stops, move focus to the question heading. Undo and Redo already show the right pattern.

### 3. Progress is either silent or noisy
**Confuses** (screen reader) · Scanning, Conversion, Report, Final report · both styles · both themes

![Scan checklist: done, active and pending are shown only by icons](shots/accessibility-polish-07-scan-steps-state.png)

- **What happens:** The scan checklist is an `<ol aria-live="polite">` whose text changed 30 times in about 3 s during the live demo ("1 file", "2 files", …), each queued for announcement. It never says which step is done, active or pending; that is only a `data-state` attribute and an `aria-hidden` icon (the conversion stepper does say "Plan: now"). The report's arrival, conversion progress ("Running the tests on this Mac (9 of 14)"), "AI repair, attempt 2 of 3", the budget stop, "Hand-off note saved" and "Redo" are all silent.
- **Expected:** A few meaningful announcements per phase, with step state available as text.
- **Suggested fix:** Use one polite `role="status"` region per screen with throttled messages ("Reading files done, 14 files. Checking code, 3 of 6"). Add hidden state text to scan steps, the same way the stepper does. Announce "Hand-off note saved to Downloads" and "Change undone; checks will run again".

### 4. Long file names and project names break the finding row
**Confuses** · Blocker report (real scan) · both styles · both themes

![A 90-character file name squeezes the title into a narrow column and overflows the window](shots/accessibility-polish-02-long-filename-overflow.png)

- **What happens:** When a finding's single location is a 90-character file name, the meta column takes the row and doesn't shrink. "Starts worker processes without a __main__ guard" wraps one word per line in about 90 px, and the meta text and chevron run past the window's right edge and are clipped. A 99-character project name without spaces is cut off in the subtitle ("…ThatCannotWra"), with no ellipsis.
- **Expected:** The title keeps a readable width, and long names truncate with an ellipsis or wrap.
- **Suggested fix:** Give the meta column `max-width: 40%` and `min-width: 0`, and middle-truncate the name (`…the_meta_column.py:3`). Set a minimum width on the title. Use `overflow-wrap:anywhere` on project names in headings and subtitles.

### 5. Scroll panes and code blocks take focus without showing it
**Confuses** (keyboard) · Applying changes, Checking it runs, AI repair, Reviewing changes, A change undone, Planning · both styles · both themes

![Code diff focused by keyboard: no visible ring](shots/accessibility-polish-04-codeblock-no-focus-ring.png)

- **What happens:** Diff blocks (`<ol tabindex=0>`, focusable so they can scroll sideways) are Tab stops. Comparing focused and unfocused screenshots showed **0 px** changed, in all four style and theme combinations. In Planning, the whole scroll pane is a Tab stop, but its box-shadow ring is clipped by the parent: about 70 px change over a 492×434 area, effectively invisible. Everywhere else (buttons, finding rows, Mode radios, inputs, Details, Engine log) a 2 px + 4 px blue ring shows in light and dark.
- **Suggested fix:** Use an inset outline on scroll containers (`outline: 2px solid var(--focus); outline-offset: -2px`).

### 6. Low-contrast tertiary text, peek line, step markers and focus ring
**Annoys** · several screens · both styles · light (mostly) and dark

![Send 4 files: the file sizes use the tertiary token](shots/accessibility-polish-05-tertiary-text-contrast.png)
![Details peek line over the window glow](shots/accessibility-polish-14-details-peek-contrast.png)

| Item | Where | Light | Dark |
|---|---|---|---|
| Tertiary 11–12 px text (#8A91A0 / #6F7788) | File sizes in "Send 4 files"; location line in an opened finding; AI repair log lines | **2.58–2.95:1** | **4.32:1** |
| Details peek, mono 12 px, over the bottom glow | Every screen with a peek line | **4.13–4.17:1** | **4.13–4.18:1** |
| Pending step circle (non-text) | Scan checklist, conversion stepper | **1.33:1** | **1.41:1** |
| Focus ring, rgba(47,107,255,.7) (non-text) | All controls | **2.76:1** | 5.30:1 |
| For reference, passing: muted text, "Paid" badge, status words, placeholders, field borders | | 5.0–7.7:1 | 5.2–7.0:1 |

- **Suggested fix:** Use the muted token (#656C7B, 4.7–5.1:1) for these labels, and lift the dark tertiary token to about #7F8798. Give the peek line a solid backing or use the secondary text color. Draw pending circles with a border of at least 3:1 (#8A91A0 gives about 3.2:1). Use the focus ring at full opacity (#2F6BFF on white is about 4.5:1).

### 7. Undone changes fade to unreadable
**Confuses** · Final report → A change undone · both styles · both themes

![The undone change is drawn at 45% opacity](shots/accessibility-polish-06-undone-row-contrast.png)

- **What happens:** An undone change is drawn at 45% opacity. Its label ("Undone. Build the index path with pathlib") measures 2.13:1 in light and 2.58:1 in dark, and its code measures 1.84:1 and 2.01:1. The label is part of an enabled button, and the user is still reviewing this change and may press Redo.
- **Suggested fix:** Keep text at full contrast. Show the undone state with an "Undone" badge, strikethrough or icon, and use the muted color (at least 4.5:1) for the code.

### 8. Arrow keys in the Mode switch discard the report
**Confuses** · every screen (Mode: Files / Project) · both styles · both themes

![After one press of ←, Files has focus, Project is still selected, and the report is gone](shots/accessibility-polish-11-mode-arrow-discards-report.png)

- **What happens:** On a report, Tab to "Project" and press ←. Focus moves to "Files" with `aria-checked=false` while "Project" stays selected, and the window jumps to "Drop a project folder": the report is gone. → doesn't bring it back. Clicking "Files" has the same effect; the Files tab isn't designed in the lab.
- **Expected:** For radios, selection follows focus. Either way, switching modes shouldn't silently throw away a finished report.
- **Suggested fix:** Make it a tab list (`role="tablist"`, manual activation) or a true radio group, and keep the Project tab's state when switching back. Ask before discarding a report or a running conversion.

### 9. Account sheet: wrong name, no Esc, heading gap
**Annoys** · Sign in, Account, Plans, Own key · both styles · both themes

![Account sheet: the close button is focused, but its accessible name is "Clear"](shots/accessibility-polish-12-account-sheet.png)

- **What happens:** The ✕ button's accessible name is "Clear". Esc does nothing. Closing the sheet sends you to the drop screen with focus on `<body>`. The Account screen has no h2: the person's name is plain text, followed by h3 "AI use" and h3 "Computers", and the initials "SR" are read out.
- **Suggested fix:** Name the button "Close" and make Esc close the sheet. Return focus to the Settings button. Make the person's name the h2 and hide the avatar from screen readers.

### 10. Short windows push the actions out of view
**Annoys** · all screens · both styles · both themes

![760×600: the action bar sits below the fold](shots/accessibility-polish-10-short-window-760x600.png)

- **What happens:** From about 600 px wide up, the window is a fixed 552×780. At 760×600 or 1440×560 its bottom 200–270 px, including the action bar (Convert with Pro, Run it, Send 4 files, Stop), is below the fold, so you scroll the page and then the pane inside it. Narrower, the window grows to its content (a 1,757 px report at 520 wide) and the action bar sits at the very bottom, not pinned.
- **Suggested fix:** Set a minimum window height (about 560 px) and let the pane flex to fill it. In the narrow layout, keep the action bar pinned to the bottom of the viewport.

### 11. Narrow widths cut off step names, and step details are mouse-only
**Annoys** · Conversion (all four states) · both styles · both themes

![380 px: the stepper reads "Sna… Pack Plan Apply Verify Repo… Rep…"](shots/accessibility-polish-08-stepper-truncated-380.png)

- **What happens:** At 380 px and below, step names are cut off ("Sna…", "Ap…", "Repo…", "Rep…"), so the current step, Repair, can't be read, and there is no way to see the full name. Each step's summary ("6 fixes, all from known patterns") exists only as a `title` tooltip: mouse only, not shown for keyboard or touch. One tooltip shows a template placeholder: "…stays on **{there}**." (Planning through AI repair, both styles).
- **Suggested fix:** At narrow widths, show only "Step 6 of 7: Repair" under the dots, or wrap the labels to two rows. Show step summaries as visible text or as an accessible description. Fill in the placeholder.

### 12. The elapsed time wraps next to a stretched Stop button
**Polish** · Scanning · Mac and Windows · light and dark · at 520 px and below

![520 px: "1.1" and "s" on separate lines next to a full-width Stop button](shots/accessibility-polish-09-elapsed-wraps-520.png)

- **Suggested fix:** `white-space: nowrap` on the timer, and keep Stop at its natural width.

### 13. Finding-row focus ring sits on the text and disappears under the action bar
**Polish** · Blocker report · both styles · both themes

![A focused row at the bottom of the pane, with its ring cut off by the action bar](shots/accessibility-polish-13-row-focus-under-action-bar.png)

- **What happens:** The ring starts 3 px left of the first letter, with no inner padding. Rows reached by Tab near the bottom of the pane scroll only to the top edge of the floating action bar, under the bottom fade, so the ring's lower edge is hidden.
- **Suggested fix:** Pad the rows or use a negative outline offset; set `scroll-padding-bottom` on the pane to the action bar height plus the fade.

### 14. Smaller semantics and content polish
**Polish** · as listed · both styles · both themes (screenshot: shot 06, showing the Undo buttons)

- Eight visual subheadings are `<p>` ("1.85 GB won't be sent", "Secrets stay on this Mac", "Pro converts it for you", "3 files couldn't be read", "Planned fixes", "Changes so far", "Checks on this Mac", "What carries over").
- Seven buttons are named only "Undo"; suggest "Undo: Open the page as UTF-8 (app/main.py)".
- "Show all 4" switches to "Show fewer" with no `aria-expanded`; the drop zone's name is the whole paragraph.
- "Email me a link" and "Save key" are disabled with no reason given; in light theme the disabled primary looks almost enabled.
- A real scan can say "0 B won't be sent" next to "OS clutter … 1 file".

## Reduced motion

- **OS setting:** Respected. Only opacity fades remain (opening a finding, changing screens, Details, Mode); layout and height animations, chevron rotation and key-press motion are gone. Spinners become a slow fade, and the consent halo (otherwise about 60 style changes a second) stops. Nothing flashes.
- **Lab switch:** Stops the JavaScript animation, but CSS transitions still run (chevron rotation, Settings key press), and it turns off on reload.

## Lab panel notes (brief)

- The reduced-motion switch and hidden-panel state (`\`) aren't in the URL and reset on reload.
- At 760 px wide and below, the panel stacks above the window (about 2,000 px down) until hidden.
- Forcing Windows style on a Mac → Windows scenario mixes copy ("would break on macOS", then "Checking local-chat for Windows … on this PC").
- "Real scan" entries are disabled before a scan, with no hint why.

## Method

Playwright with Chrome at 1x: 34 states × macOS/Windows × light/dark, plus the Xcode, WPF, Windows Python and documents scenarios. Keyboard: Tab walks from the title bar, logging `activeElement` and diffing each element's pixels focused vs unfocused. Contrast: computed colors (with opacity) blended over the median background pixel of a text-hidden screenshot; non-text contrast from rendered pixels. Motion: `getAnimations()`, style-change counts and per-frame computed styles. Viewports 320–1440 wide, 560–1000 high, plus WCAG text spacing. Two stress projects scanned through the lab: 20 findings, about 250 places and 231-character paths; and a 99-character project name without spaces with a 90-character file name.
