# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Design prototype rendered in the browser. The shipped product is a cross-platform desktop app for macOS and Windows; this repo is the UI design, intended to be reused inside a desktop shell later.)

## Stack

Vite + React 19 + TypeScript + Motion (`motion/react`), confirmed by the user. Plain CSS with design tokens in one file plus CSS Modules. Fonts self-hosted through Fontsource so they work offline in a desktop shell. Playwright as a dev-only screenshot tool. Desktop shell (Electron or Tauri) is undecided.

## Users

People who work across a Mac and a Windows PC at the same desk or on the same network, and need to move files, or whole code projects, between the two without accounts, cloud drives or USB sticks.

## Product Purpose

SeamDrop moves things between a Mac and a Windows PC.

- Free tier: send files and folders over the home network: Wi-Fi, or a computer plugged into the router.
- Paid tier, "Project": send a code project; it arrives converted and ready to run on the other OS, or, when it cannot convert, with a clear report of the blockers.

Success: the receiving machine can open the files, or run the project, with no extra steps.

## Positioning

Not a sync service or a cloud drive: a direct link between two paired machines, and the only one that converts a project for the other OS and is honest about what cannot be converted.

## Operating Context

Two paired machines, each running SeamDrop. Each one decides, per paired device, what happens to files from it: "Files from <device>: Ask every time · Always accept · Never" (Settings → Devices; Ask every time by default). The usual setup is a PC wired to the router and a Mac on its Wi-Fi; they find each other again by themselves after sleep or a new address, and a connection that drops mid-transfer pauses and continues by itself when they see each other again (for up to a day).

## Capabilities and Constraints

- Link pill: Wi-Fi, Ethernet, or Network.
- Files from <device>: Ask every time, Always accept or Never, per paired device, enforced by the receiving computer's core. When the receiver says Never, the sender is told which computer and which setting to change, nothing is sent, and it sends by itself once that changes.
- Project conversion produces a live checklist and an engine log; native apps (Xcode projects, WPF/.NET) produce a rebuild report with fatal blockers and next actions (Try to replicate, Hand-off plan, Keep report).
- Undecided: login, sign-up, onboarding (must not be designed yet), pricing, final product name.

## Brand Commitments

- Name: SeamDrop (was the working name "Portal"). Same brand on both OSes; each window follows its OS conventions (macOS traffic lights + centred title; Windows caption buttons top-right + left title with app icon).
- No Apple or Windows logos. No logos, wordmarks or brand elements from the reference images.
- Colour story: electric blue and warm orange are the two ends of a transfer, used sparingly.

## Evidence on Hand

None. All device names, file names, sizes, projects and log lines in the prototype are synthetic demonstration data.

## Product Principles

1. Say which side to act on. Every blocked or waiting state names the machine and the switch.
2. Show what the engine did. Conversion is a visible checklist and a readable log, never a black box.
3. Honest outcomes. "Converts, ready to run" and "Rebuild, 3 fatal blockers" are equally first-class results.
4. Native on each OS, one brand across both.

## Accessibility & Inclusion

Visible keyboard focus, WCAG AA contrast, respects prefers-reduced-motion (and reduced transparency), works at narrow widths.
