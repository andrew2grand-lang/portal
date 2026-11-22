---
name: SeamDrop
description: A direct link between a Mac and a Windows PC; the window edge carries the light of a transfer.
colors:
  send-blue: "#2f6bff"
  send-blue-deep: "#1f55e0"
  send-blue-pale: "#93b4ff"
  land-orange: "#ff7a2e"
  land-orange-glow: "#ff9452"
  land-orange-deep: "#b84a0c"
  land-orange-pale: "#ffbf94"
  gradient-midpoint-lilac: "#9b7cf0"
  danger-red: "#c22f35"
  danger-red-dark: "#ff8a8e"
  ink-navy: "#141824"
  slate-text: "#4a5160"
  muted-text: "#656c7b"
  log-graphite: "#3a404c"
  canvas-mist: "#e7e9ee"
  window-white: "#f8f9fc"
  horizon-lilac: "#f2ecf4"
  hairline: "rgb(20 26 40 / 0.08)"
  frost-glass: "rgb(255 255 255 / 0.56)"
  frost-glass-strong: "rgb(255 255 255 / 0.86)"
  canvas-void: "#030407"
  window-navy: "#0b0e17"
  navy-surface: "#11151f"
  frost-text: "#eceff5"
  frost-text-2: "#a3abbc"
  frost-text-3: "#808899"
typography:
  display:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "3.25rem"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.035em"
    fontVariation: "\"wdth\" 115"
    fontFeature: "\"tnum\""
  headline:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 800
    lineHeight: 1.06
    letterSpacing: "-0.035em"
    fontVariation: "\"wdth\" 115"
  title:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "1.375rem"
    fontWeight: 720
    lineHeight: 1.22
    letterSpacing: "-0.022em"
    fontVariation: "\"wdth\" 108"
  body:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 430
    lineHeight: 1.45
    letterSpacing: "-0.004em"
    fontVariation: "\"wdth\" 100"
  body-small:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 430
    lineHeight: 1.45
  label:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 620
    lineHeight: 1
    letterSpacing: "-0.006em"
    fontVariation: "\"wdth\" 104"
  label-micro:
    fontFamily: "Mona Sans Variable, ui-sans-serif, system-ui, sans-serif"
    fontSize: "0.6875rem"
    fontWeight: 620
    lineHeight: 1
    letterSpacing: "0.01em"
  mono:
    fontFamily: "Geist Mono Variable, ui-monospace, SF Mono, Menlo, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
    lineHeight: 1.65
rounded:
  window-mac: "20px"
  window-win: "10px"
  stage: "28px"
  stage-inset: "6px"
  stage-plate: "22px"
  card: "16px"
  tray: "15px"
  key: "12px"
  key-sm: "10px"
  tile: "12px"
  tile-sm: "10px"
  drop-tile: "20px"
  focus: "8px"
  check: "6px"
  pill: "999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "20px"
  "6": "24px"
  "7": "28px"
  "8": "32px"
  "10": "40px"
  "12": "48px"
  "14": "56px"
  window-pad: "24px"
  canvas-gap: "56px"
components:
  key-plain:
    backgroundColor: "#ffffff"
    textColor: "{colors.ink-navy}"
    typography: "{typography.label}"
    rounded: "{rounded.key}"
    padding: "0 16px"
    height: "40px"
  key-plain-sm:
    rounded: "{rounded.key-sm}"
    padding: "0 12px"
    height: "32px"
  key-plain-lg:
    padding: "0 20px"
    height: "44px"
  key-ink:
    backgroundColor: "#161a25"
    textColor: "#f7f8fb"
    typography: "{typography.label}"
    rounded: "{rounded.key}"
    padding: "0 16px"
    height: "40px"
  key-lit-blue:
    backgroundColor: "#eef3ff"
    textColor: "{colors.send-blue-deep}"
    rounded: "{rounded.key}"
  key-lit-orange:
    backgroundColor: "#fff3eb"
    textColor: "{colors.land-orange-deep}"
    rounded: "{rounded.key}"
  key-lit-dual:
    backgroundColor: "#f5f3fb"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.key}"
  key-tray:
    backgroundColor: "rgb(20 26 40 / 0.05)"
    rounded: "{rounded.tray}"
    padding: "5px 5px 4px"
  mode-switch-option:
    textColor: "{colors.slate-text}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "0 14px"
    height: "32px"
  mode-switch-option-selected:
    textColor: "{colors.ink-navy}"
  paid-pill:
    textColor: "{colors.slate-text}"
    typography: "{typography.label-micro}"
    rounded: "{rounded.pill}"
    padding: "0 7px"
    height: "18px"
  chip-neutral:
    backgroundColor: "{colors.frost-glass-strong}"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.pill}"
    padding: "0 12px 0 10px"
    height: "28px"
  chip-send:
    backgroundColor: "rgb(47 107 255 / 0.1)"
    textColor: "{colors.send-blue-deep}"
    rounded: "{rounded.pill}"
  chip-land:
    backgroundColor: "rgb(255 122 46 / 0.11)"
    textColor: "{colors.land-orange-deep}"
    rounded: "{rounded.pill}"
  chip-danger:
    backgroundColor: "rgb(217 61 66 / 0.09)"
    textColor: "{colors.danger-red}"
    rounded: "{rounded.pill}"
  checkbox:
    backgroundColor: "#ffffff"
    rounded: "{rounded.check}"
    size: "20px"
  stage-shell:
    backgroundColor: "rgb(255 255 255 / 0.3)"
    rounded: "{rounded.stage}"
    padding: "6px"
  stage-plate:
    backgroundColor: "{colors.frost-glass}"
    rounded: "{rounded.stage-plate}"
    padding: "28px"
  icon-tile:
    backgroundColor: "#ffffff"
    textColor: "{colors.slate-text}"
    rounded: "{rounded.tile}"
    size: "48px"
  log-terminal:
    backgroundColor: "rgb(20 26 40 / 0.045)"
    textColor: "{colors.log-graphite}"
    typography: "{typography.mono}"
    rounded: "{rounded.card}"
    padding: "12px 14px"
---

# Design System: SeamDrop

All values live in `src/styles/tokens.css` (CSS) and `src/styles/motion.ts` (springs, lap speeds). Components read the semantic tokens (`--surface-*`, `--text-*`, `--key-*`, `--glow-*`), so the look changes from those two files. The frontmatter above records the light theme (the primary) plus the dark-theme anchors; tokens.css is the source when they disagree.

## Overview

**Creative North Star: "Light Between Two Machines"**

A transfer is light passing from one window to the other. The window itself is the indicator: during a transfer its edge carries a blue comet out of the sender and fills the receiver with orange, ending in an arrival flash. Everything else stays quiet so that light reads: airy cool-white ground with a faint blue-to-orange wash settling into a rose-lilac horizon, a frosted white stage in a double bezel, ink-navy text, and soft mechanical keycaps sitting in a recessed tray.

Dark is the same world at night: a near-black canvas, deep-navy window ground with a blue horizon glow rising from the bottom edge, the same keys backlit more strongly, and the edge glow reading as real light. Density is calm desktop-utility: one window at 552px, one stage, a short header, a log that stays folded until asked for.

Each window is native to its OS (traffic lights and centred title on macOS, left title with app mark and caption buttons on Windows; window radius and action order follow the OS) while the brand inside is identical.

**Key Characteristics:**
- Blue is the sending end, orange the receiving end; neutrals carry everything else.
- The window edge is the progress indicator; the stage bar is the precise readout.
- Buttons are softened keycaps with real travel, a skirt, a cast shadow and a backlight.
- Wide-bold Mona Sans headings, normal-width body, Geist Mono only for machine output.
- Frosted surfaces that go solid under reduced transparency; motion that becomes still light under reduced motion.

## Colors

Cool neutrals and frosted white carry the interface; electric blue and warm orange appear only where one end of a transfer is meant.

### Primary
- **Sending Blue** (send-blue): the sending end. Edge-glow comet, Send key backlight and LED, drag-over drop tile, live dot, focus ring hue, start of the progress gradient. Text uses the deeper **Sending Blue Deep** (send-blue-deep) in light and **Pale Send** (send-blue-pale) in dark for contrast.

### Secondary
- **Landing Orange** (land-orange): the receiving end. Receiving perimeter fill, working orbit, arrival flash, Receive key backlight, conversion "fix" marks, end of the progress gradient. The comet's tail warms to **Glow Orange** (land-orange-glow). Text uses **Landing Orange Deep** (land-orange-deep) in light and **Pale Land** (land-orange-pale) in dark.

### Tertiary
- **Midpoint Lilac** (gradient-midpoint-lilac): never used alone. It is only the midpoint where blue becomes orange, in the progress fill and the gradient outline.
- **Danger Red** (danger-red; dark: danger-red-dark): fatal blockers, error log lines, the danger chip. Always on a soft red tint, never as a fill.

### Neutral
- **Ink Navy** (ink-navy): primary text, ink-key face in light, the brand mark ground (the mark reads `--gray-900`, `--blue-500` and `--orange-500` from tokens).
- **Slate Text** (slate-text): secondary text, titlebar text, unselected switch options.
- **Muted Text** (muted-text): tertiary text, units, metadata, disabled labels.
- **Log Graphite** (log-graphite): log body text in light.
- **Canvas Mist** (canvas-mist): the desk the windows sit on in light; the browser theme-color follows `--canvas` in both themes.
- **Window White** to **Horizon Lilac** (window-white, horizon-lilac): the window ground's vertical base, under the blue (top-left), orange (bottom-right) and lilac/rose washes.
- **Frost Glass** / **Frost Glass Strong**: the stage plate and the floating pills (link pill, neutral chip).
- **Hairline**: 1px rings that replace borders on every surface.
- Dark anchors: **Canvas Void** (canvas-void), **Window Navy** (window-navy), **Navy Surface** (navy-surface, the solid fallback for glass), **Frost Text** 1/2/3.

### Named Rules
**The Two Ends Rule.** Blue means the sending end and orange means the receiving end, everywhere: keys, chips, tiles, glow, log lines. Never use either as decoration or as a generic "primary" colour. A screen with no transfer meaning should be almost entirely neutral.

**The Blend Is a Journey Rule.** The blue-to-orange gradient (via lilac) only appears where something travels from one end to the other: the progress fill, the edge-glow comet, the arrival flash, the brand mark's stroke, and the Paid pill's outline. It is never a background fill on a control.

**The One Outline Rule.** Among controls and tags, exactly one element carries the gradient outline: the Paid pill. The window edge glow is light that marks state and only shows during a transfer; it does not count toward this rule and is not an excuse for a second gradient-bordered control.

## Typography

**Display Font:** Mona Sans Variable, using the width axis (fallback ui-sans-serif, system-ui)
**Body Font:** Mona Sans Variable at normal width
**Label/Mono Font:** Geist Mono Variable (fallback ui-monospace, SF Mono, Menlo)

**Character:** One family stretched to two voices: headings run wide and heavy with tight tracking, like hardware labelling; body stays normal width and slightly tight. The mono is the engine talking.

### Hierarchy
- **Display** (800, 52px, 1.0, width 115%, tabular figures): the transfer percentage only, with its unit set at title size in muted text.
- **Headline** (800, 28px, 1.06, width 115%): the paired device name in the window header. Drops to 22px below 520px.
- **Title** (720, 22px, 1.22, width 108%, balanced wrap): stage state titles ("Sending to Desk PC", "Drop files or folders").
- **Body** (430, 14px, 1.45): sentences, item names (520 weight), checklist rows. Subtitles cap at 46ch, drop-zone copy at 34ch.
- **Body small** (430, 13px): metadata, sizes, rates, link pill, chip text (520).
- **Label** (620, 13px, width 104%): keycaps and switch options. 14px on large keys.
- **Label micro** (620, 11px, +0.01em): the Paid pill only.
- **Mono** (400, 12px, 1.65): the Details log, its one-line peek, file paths and the running-command line.

### Named Rules
**The Machine Voice Rule.** Geist Mono appears only for what the machine produced or will execute: log lines, paths, commands. Never for labels, numbers in the UI or decoration; UI numbers use Mona Sans with tabular figures.

**The Wide Means Heading Rule.** The width axis goes above 104% only for the three heading roles. Body text always sits at 100%.

## Layout

The canvas centres two windows (552 x 780px each) side by side with a 56px gap; below 1240px they stack in one column with a 48px gap, and below 600px the canvas padding drops to 24px/16px. Each window is a fixed-height column (780px): OS chrome (52px macOS, 40px Windows), then a body padded 8px top, 24px sides, 20px bottom with 20px gaps holding the device header, the controls row (the mode switch), the stage taking all remaining height, and the Details disclosure. The window frame never resizes. Inside the stage, each state is a pane: a scrolling body (28px sides and top, 20px bottom, 20px gaps) whose lower 22px fades out, and, for top-aligned states, an action row docked beneath it (0 28px 24px) so actions never scroll away. Short states centre their content and actions together in the plate.

Below 520px the window becomes auto-height (the stage keeps a 352px minimum), side padding becomes 16px, the link pill drops beneath the device name, pane bodies pad 20px (16px bottom) and the docked row 0 20px 20px, and action keys stack full width.

Spacing is a 4px grid (4 to 56). Stage panes use 28px padding and 20px gaps; tight clusters use 6 to 12px.

### Named Rules
**The Docked Actions Rule.** A state's actions stay docked below its scrolling body; content scrolls, the action row does not. Only centred short states keep actions inline with their content.

**The Native Order Rule.** Action groups are right-aligned on both OSes, passed primary-first: macOS places the primary key rightmost, Windows places it leftmost. Never hard-code one order.

## Elevation & Depth

A layered hybrid: ambient, very soft cast shadows plus inset highlights and 1px hairline rings, over translucent frosted glass (22px blur, 160% saturate). Nothing has a hard border; edges are drawn with `0 0 0 1px` hairlines and a 1px top highlight. Physical depth is reserved for things you press (keys, the checkbox) and things that hold content (tiles, the stage).

### Shadow Vocabulary
- **Window** (`--window-shadow`): four-layer lift of the app window off the canvas; much deeper in dark.
- **Stage** (`--shadow-stage`): long, faint drop under the double-bezel shell.
- **Shadow 1 / 2** (`--shadow-1`, `--shadow-2`): hairline contact shadow for floating pills and the stage plate.
- **Tile** (`--shadow-tile`): top highlight, hairline ring, a 2px solid lip below, and a soft drop; makes icon tiles read as small keys.
- **Key stack** (`--key-bevel`, `--key-skirt`, `--key-drop`, `--key-drop-latched`): bevel on the face, a coloured skirt 2px below the cap, and a cast shadow that compresses on press and when latched.
- **Tray** (`--tray-shadow`): inset shadow for recessed wells (switch tracks).
- **Inset** (`--inset-shadow`): the shallower recess of the progress track and the log terminal.
- **Check drop** (`--check-drop`): the checkbox's small cast shadow.
- **Edge glow**: a masked ring on the window edge, an inner wash clipped to the radius, and an 18px (22px dark) blurred halo outside the window.

### Named Rules
**The Light Is Not a Shadow Rule.** Coloured glow is light from a source: under a lit key, around a window during a transfer, inside the drop portal. Never use a coloured box-shadow as general elevation.

**The Glass Goes Solid Rule.** Under `prefers-reduced-transparency`, glass and shell surfaces become solid (white/#f1f3f7 light, navy-800/navy-850 dark) and blur is removed. Any new frosted surface must read from `--surface-glass`/`--glass-blur` so it inherits this.

## Shapes

Rounded, soft and nested. Corner radii follow one hierarchy: windows follow their OS (macOS 20px, Windows 10px); the stage is the largest radius inside the app (28px) and its inner plate is the stage radius minus the 6px bezel gap (22px), as is the drop zone; the log terminal is a 16px card; the key tray is 15px so 12px keys nest inside its 5px padding; keys and the 48px icon tiles share 12px; small keys, 36px item tiles and the Details toggle share 10px; the 68px drop tile is 20px; the default focus ring rounds at 8px; the checkbox is 6px; chips, the link pill, the mode switch, the progress bar, notices and the Paid pill are full pills. Circles are kept for status: state check icons, blocker icons, the live dot, the LED, traffic lights and the drop portal rings.

### Named Rules
**The Nested Radius Rule.** An inner surface's radius equals its container's radius minus the gap between them. Never put a larger radius inside a smaller one.

## Components

### Keys (buttons)
Soft mechanical keycaps: tactile, quiet until selected.
- **Shape:** 12px radius (10px small); heights 32 / 40 / 44px with 12 / 16 / 20px side padding; icon-only keys are square.
- **Construction:** five layers, bottom to top: backlight glow, cast drop, skirt (the visible 2px of height), cap, bevelled face carrying icon, label and optional LED.
- **Plain:** white-to-pale face, Ink Navy label. The default for secondary actions.
- **Ink:** high-contrast primary (dark face in light, light face in dark). One per action group: Accept, Open in Downloads, Send, Try to replicate.
- **Lit:** backlit and latched. Light rises from under the cap in the key's tone (blue send, orange receive, dual split for Both), the skirt picks up the colour, a 4px LED glows in the corner, and the face sits 1.5px lower than its neighbours. Used for the open settings key and a signature completed action ("running").
- **Press:** the cap travels 2px down on a fast critically damped spring (0.1s) and the drop fades to 35%; release springs back with bounce 0.45 over 0.32s. Hover lifts brightness 1.5% (12% in dark).
- **Focus:** the focus ring is drawn on the face, outside the bevel.
- **Disabled:** label to Muted Text, face desaturated, drop at 40%.

### Mode switch and choices
- **Mode switch:** a pill track with the same inset shadow; a raised pill thumb slides between Files and Project on a layout spring. Selected option Ink Navy, others Slate. When locked, unselected options fall to Muted Text.
- **Paid pill:** 18px pill inside the Project option, glass fill, 1px gradient outline (blue, lilac, orange), 11px label.
- **Choice** (Mission 02): the mode switch's track and thumb at 28px for a few exclusive options, e.g. "Files from MacBook Pro: Ask every time · Always accept · Never" in Settings → Paired devices, under a 13px semibold Ink Soft label. Arrow keys move the choice. (The Send / Both / Receive role tray is gone: each computer now decides per device what happens to files from it.)

### Chips
- **Style:** 28px full pill, frosted strong glass with top highlight and hairline ring, 13px medium text, optional 15px icon in Slate.
- **Tones:** send, land and danger swap to the soft tint of that colour (`--send-soft`, `--land-soft`, `--danger-soft`) with the matching 1px inset ring (`--send-ring`, `--land-ring`, `--danger-ring`), and the icon takes the text colour. The link pill in the header is the same construction at 30px.

### Stage (cards / containers)
- **Double bezel:** a translucent shell (28px, 6px padding, white ring, stage shadow) holding a frosted plate (22px, glass with blur, top highlight, hairline, shadow 1). The only double-bezel surface in the app.
- **Panes:** 28px padding, 20px gaps; short states centre in the stage like the drop zone, long states top-align with actions pinned to the bottom.
- **State changes:** "materialize": the new pane arrives from 10px below with 6px blur and fades in; the old leaves 6px up with 4px blur.

### Icon tiles
48px (header, state; tile radius) / 36px (items; tile-sm radius) / 68px (drop tile; drop-tile radius) raised tiles using the tile gradient and tile shadow; icon in Slate, or in send/land text colour when the state belongs to one end.

### Drop zone
The idle stage is the primary action: a centred tile inside three faint concentric hairline rings (masked out below the tile) over the soft blue-then-orange portal light (`--portal-glow`). Hover lifts the tile 2px; drag-over lifts it 4px, turns it Sending Blue, widens the rings and tints them with `--send-ring-strong`. Keyboard focus draws an inset ring (`--focus-inset`) on the plate's own radius.

### Progress
A 52px Display percentage, an 8px pill bar on a faint inset track (`--inset-shadow`) filled with the blue-lilac-orange gradient, and a tabular metadata row (size, rate, time left). Item rows below show queued (50% opacity), active (blue spinner) and done.

### Details log (mini terminal)
A folded disclosure at the window foot: icon, "Details", and a one-line mono peek of the latest log line. Open, it is a 16px recessed card (`--inset-shadow` plus the log ring) with mono lines on a three-column grid (time, tag, text), toned ok (blue), warn (orange), err (red) and dim; a 14px fade masks the top of the scroller.

### Checkbox
A 20px mini keycap with 6px radius: raised plain face over its skirt and `--check-drop` when empty, ink face with a check when checked, presses 1px.

### Edge glow (signature)
Lives in the window shell, outside the clipped window, so the ring follows the OS window radius exactly and the halo blooms outside.
- **Sending:** a comet orbits the edge, bright blue head warming to an orange tail. One lap takes 2.8s on Wi-Fi and faster on a wired link; speed eases into changes rather than jumping.
- **Receiving:** the edge fills clockwise from the top in orange as bytes land, with a hot pale tip; the ring and inner wash brighten with progress.
- **Working:** a slow warm orbit (3.6s a lap) while the receiver converts a project.
- **Arrival:** the whole edge flashes once (1.9s), orange through blue, with a wider halo.
- **Reduced motion:** no travel; a still gradient-lit ring whose brightness still tracks progress, and a gentler 1.6s arrival pulse.

### Text field (Mission 01)
Sunken like the key tray, so it reads as a place to type into: 44px, `--radius-tile`, `--surface-sunken` with `--inset-shadow` and a hairline ring (stronger on hover, `--danger-ring` when invalid), label above in Body Small semibold Ink Soft, hint or error below (error in `--text-danger`, announced). Addresses and paths use the mono face.

### Connection screens (Mission 01)
Built only from existing parts: Pane, Head, StateTile, Waiting, Actions, Key, Checkbox.
- **Doctor list:** the blocker row (22px round icon, title, plain fix) toned by status: pass Sending Blue on `--send-soft`, fail red on `--danger-soft`, likely Receiving Orange on `--land-soft`, unknown Ink Soft on `--surface-sunken`. A check that a settings page fixes carries one small plain key under its fix. Problems sort first.
- **Pairing code:** six digits in two groups of three, 46px heavy, `--stretch-display`, tabular figures; dims to Ink Muted once confirmed here while waiting for the other side.
- **Sections (settings):** an all-caps 12px Ink Muted label over its controls; sections after the first get a hairline rule and 16px above.
- **Rescue actions** (Try again, Connect by IP) dock below the scrolling checklist like every other action row.
- **Send list ("Ready to send"):** dropping or picking files no longer sends at once; they collect here. Head with the totals ("214 files · 633 MB · to Desk PC"), the standard item rows (tile, name, meta) each ending in a small plain ✕ key; a row that's still being counted shows a small spinner and "Counting…" and keeps Send disabled; an unreadable item or link is dimmed with "Can't be sent". Under the list, a sunken strip ("Drop more here, or" + small **Add files** / **Add a folder** keys) that turns `--send-soft` with Sending Blue text while something is dragged over the window. Docked actions: **Send** (ink, ↗) and **Clear**. With an empty list the stage is the usual drop zone, which says files wait until you press Send.
- **Thinking orb (transfers only):** one small thinking orb (`thinking-orbs`, 32 px, monochrome dots that follow `data-theme`) sits just right of the big percentage while files move: **working** when sending, **weaving** when receiving. It holds still when the transfer stalls (connection interrupted) and under reduced motion. Nowhere else: searching, pairing and waiting keep the calm spinner and tiles, and nothing moves while SeamDrop can't reach the other computer.
- **Disconnected:** "Disconnect" (on "isn't reachable", and in Settings under "Files from …") stops SeamDrop connecting to a paired device, either way, until **Connect**. The stage shows a calm state: StateTile with a broken-link glyph, "Disconnected from Desk PC", one ink **Connect** key. The header says "Disconnected". The device stays paired: no code to connect again.
- **Refused (Mission 02):** when the receiver's "Files from <this computer>" is Never, the sender's stage says "Desk PC doesn’t accept files from this Mac", names the setting to change on the other computer, lists what's waiting (dimmed) with a calm "Waiting for Desk PC…", and offers **Cancel**. A neutral StateTile with a prohibit glyph: it's information, not an error. The receiver shows nothing; its log notes the refusal.
- **Arrival moment (Mission 02):** a transfer that's done in under 0.4 s never shows the progress bar: the stage shows "Sending to / Receiving from" with the items, the edge light plays (comet on the sender; on the receiver the edge fills in 0.6 s), and the moment lasts at least 0.9 s before "Sent to" / "Received from" and the arrival flash. Under reduced motion the same order with the still ring and the gentle pulse.
- **Connection interrupted:** while a running transfer moves nothing for 3 s, the existing link-notice pill (sunken, Ink Soft, Wi-Fi-slash icon) sits under the progress bar saying "Connection interrupted. Waiting for it to come back…", and "Waiting" replaces the time left. It goes away by itself when bytes move again.

### Real windows (Mission 01)
In the app the window is the viewport: no outer shadow or halo (nothing can be drawn outside a real window), the edge ring sits just inside the edge at the OS corner radius (`--radius-window-native-mac` 16px, `--radius-window-native-win` 8px). macOS keeps the drawn title and gets the OS traffic lights over it; Windows is undecorated and the drawn caption buttons are live. A small plain settings key sits in the title bar (right on macOS, left of the caption buttons on Windows). The compact accept window is the stage plate alone on the window ground, with native chrome.

### Named Rules
**The Settled Spring Rule.** Interface motion uses critically damped springs (bounce 0, about 0.42s) and `cubic-bezier(0.16, 1, 0.3, 1)` easing. Overshoot is reserved for physical travel: a key's release springs back (bounce 0.45) because a press carries momentum, and the sliding switch thumb settles with a barely perceptible layout spring (bounce 0.08). Nothing else overshoots.

**The Still Light Rule.** Under reduced motion every travelling or orbiting effect becomes a static state carrying the same information (still ring, breathing spinner, no live-dot pulse, stage states cross-fade without blur or rise); CSS transitions collapse to 1ms.

## Do's and Don'ts

### Do:
- **Do** change the look in `src/styles/tokens.css` and `src/styles/motion.ts`; components read semantic tokens only, including radii, tone rings and inset shadows. Add a token rather than a literal value in a module.
- **Do** use blue for anything about the sending end and orange for anything about the receiving end, including log lines, chips and tile icons.
- **Do** build every new button as a Key (plain, ink or lit) and give each action group at most one ink key.
- **Do** order actions primary-first and let the OS decide which side the primary sits on; dock them below the scrolling pane body.
- **Do** nest radii as container minus gap (stage 28px, plate 22px).
- **Do** draw edges with 1px hairline rings and a top highlight instead of borders.
- **Do** keep every frosted surface on `--surface-glass` and `--glass-blur` so it goes solid under reduced transparency.
- **Do** give every travelling or orbiting effect a still equivalent under reduced motion.

### Don't:
- **Don't** use blue or orange as decoration, or as a generic brand fill on a neutral control.
- **Don't** add a second gradient-outlined control or tag; the Paid pill is the one.
- **Don't** set UI labels, numbers or headings in Geist Mono; it is for logs, paths and commands only.
- **Don't** use the width axis above 104% for body text.
- **Don't** add overshoot beyond a key's release and the switch thumb's slight layout spring.
- **Don't** use Apple or Windows logos, or any brand elements from reference material; the brand mark is the only identity asset.
- **Don't** put a larger radius inside a smaller one, or make the stage anything other than the largest in-app radius.
