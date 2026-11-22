---
version: 1
slug: "src-app-tsx"
primary_target: "src/App.tsx"
related_targets: []
---

Scope: the Portal app window, shown twice side by side (macOS + Windows) on a plain canvas. Visitor mode: Operate.
Audience/job: someone moving files or a code project between a Mac and a Windows PC. Every state from the brief must be reachable through the app's own controls; testing extras live in a hidden dev panel (backtick).

## Direction contract

THESIS: A transfer is light passing between two machines. The window itself is the indicator: its edge carries the light out of the sender and fills the receiver. Refuses the category default of a progress card floating in a generic settings-style window.

OWN-WORLD: Airy cool-white window ground with a faint blue-to-orange wash that settles into a rose-lilac horizon; frosted white stage in a double bezel; ink-navy text; Mona Sans wide-bold headings; Geist Mono only for the engine log and paths; soft mechanical keycaps in a recessed tray, the selected key backlit blue (Send), orange (Receive) or split (Both). Dark: near-black and deep navy, horizon glow, same keys glowing.

STORY: The user sees which machine they're paired with and how (Wi-Fi or Cable, offline), picks a role and mode, drops something, and watches it leave one window and land in the other, ending in a clear outcome: received, ready to run, or a rebuild report.

FIRST VIEWPORT: Two windows, about 520px each, centred with a 56px gap. Inside each: OS chrome, the paired device name set large in wide bold, link pill on the right, the role key tray plus the Files/Project switch, then the stage (drop zone) taking the remaining height, and the Details log disclosure at the bottom. The primary action is the drop zone.

FORM: Brief-pinned direction, no concept roll. The user pinned the world in their own words: "Light mode is the primary design... soft airy background with a subtle blue-to-orange mesh/gradient wash, frosted white surfaces, dark text, one gradient-outlined accent element"; dark "near-black and deep navy surfaces... light that glows from edges"; buttons "a softer, less prominent take on the mechanical-keycap reference". Impeccable's rule: a brief-pinned direction beats the roll. Signature interaction: the window-edge glow (comet around the sender's edge, perimeter fill and arrival flash on the receiver). Motion grammar: critically damped springs, materialize-style state crossfades, keys spring on press.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
