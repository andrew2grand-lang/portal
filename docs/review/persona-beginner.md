# Persona review: beginner on a Windows gaming PC

## Who I am and what I tried

I game on Windows and use Discord. I don't really code. My friend Sam sent me a small Python app from his Mac, with a `requirements.txt` and a `run.sh`. Will it run on my PC, and what do I have to do? I'm scared of breaking something.

I set the lab to **Windows** and followed the "Python local-AI app · Mac → Windows" story. I also built a project like Sam's (Flask, `uvloop`, `.env`, `run.sh`, a hard-coded `/Users/sam/...` path) and scanned it with the lab's real-folder scan and `portal-scan`.

## Task 1: What is this, and is it free?

**Finished:** partly · **Steps:** 3 reads, 2 clicks (Files tab, gear; neither did anything)

The big title says **MacBook Pro**, and for a second I thought Portal had mixed up which computer I'm on. The tab I need is labelled **Project** with a **Paid** badge, so I almost didn't open it. Underneath, though, a card says **Free scan**. So is it paid or free? Scanning is free and fixing is "Pro", I worked out, but I found the price only via the lab panel. "Project Send" never appears in the window.

**Words I didn't know:** Pro conversion; the "Wi-Fi" pill (is it a button?).

## Task 2: Will my friend's project work on my PC?

**Finished:** partly · **Steps:** about 7 (switch to Windows, click the drop zone, which only plays the demo, then find the real scan, enter the path, scan, tick online, scan again)

On my PC the drop zone says *"tells you what would break on **macOS**"*: the wrong direction, since I want to run it **here**. I could only check for Windows through the lab's "For Windows" switch, and then the report told me to *"Add them on **MacBook Pro** yourself"* and that *"the 4 fixes above happen on MacBook Pro"*. It also said a path *"only exists on this computer"*, when it really exists only on Sam's.

The headline *"No blockers found. 4 things will need a fix"* relaxed me. With the online package check, a fifth item appeared: *"uvloop has no ready-made build for Windows … if it builds there at all"*. To me that sounds like it might not install. The headline still said "No blockers".

With a fenced-code README, the same project got two extra findings ("Run script has no Windows version", "Run command uses python3"). My indented README got neither. Neither flagged `source .venv/bin/activate`, the first step I'd copy.

## Task 3: Understand the result and what to do next

**Finished:** partly · **Steps:** 5 clicks to open findings, plus scrolling inside the card

Every fix is written for a programmer: *"Move the code that starts the pool into `main()` and call it under `if __name__ == "__main__":`"*. The only line I could act on was *"A Pro conversion does this for you on Windows."* **Export hand-off note** said *"Hand-off note saved"*. Saved where? What is it? Maybe something for Sam?

**Words I didn't know:** `__main__` guard, UTF-8, "glues paths with '/'", PyPI, wheels, compiling from source, Microsoft C++ Build Tools, lockfile, chmod +x, pathlib, Docker, "dependency ollama", "Finding ecosystems".

## Task 4: Deal with what it says "needs you"

**Finished:** no · **Steps:** 2 clicks, 3 reads

*"Install Ollama on Windows, or run it with Docker."* There's no link, and I don't know what Docker is. For the secrets: *"Move them yourself, or set them up again on Windows (a password manager helps)."* What is OPENAI_API_KEY? Does it cost money? Do I ask Sam for it? The final report says *"Then run ollama pull llama3.2"*, but run it where? I know *what*, not *how*.

## Task 5: Decide whether to pay

**Finished:** partly · **Steps:** 1 click, then the lab's Plans and Account screens

I clicked **Convert with Pro** expecting a price or sign-in, and went straight to *"This project wants to run uvicorn…"*, and Details already said *"apply 6 changes in 5 files"*. Did I just pay? Did it already change my files? Plans says Pro is **$12 a month** with "up to $20 of AI use". More than the price? And monthly, for one project? The Account screen is **Sam Rivera**'s, and it says *"MacBook Pro and Desk PC. Pro works on both."* So does Sam pay for my PC? I couldn't tell.

## Task 6: Approve things while it fixes the project on my PC

**Finished:** yes · **Steps:** 3 clicks (Run it, then the final report, then Review the 7 changes)

*"Run a command?"*: the obvious button is **Run it**, the command is gibberish, and *"runs on this PC as you"* sounds scary. "If you skip it … nothing else changes" calmed me down. The AI screen says *"passage names use \ instead of /"* (what are passage names?), and the cost line hides behind the buttons. The final report is the best screen: *"Runs on Windows. 2 things need you."*, **Proved** checks, **Undo** on every change. But it never tells me how to start the app myself.

## Findings

1. **Blocks use: Drop screen (Windows).** [shot](shots/persona-beginner-01-drop-windows.png), [shot](shots/persona-beginner-02-real-scan-offline-details-open.png)
   *Happened:* On my PC, Portal only checks a project for the *other* computer. When I forced a Windows check through the lab, the report sent me to fix things on the MacBook Pro.
   *Expected:* "Will this run on this PC?" for a project I already have.
   *Fix:* Add a "Check for: this PC / MacBook Pro" choice. Word "this computer", "fixes happen on", and "add them on" from the point of view of the computer the project will run on.

2. **Confuses: Report headline.** [shot](shots/persona-beginner-03-real-scan-online.png), [shot](shots/persona-beginner-04-uvloop-opened.png)
   *Happened:* "No blockers found" while a package "may not build there at all". In an offline scan (the default) it's hidden under a collapsed "Couldn't check".
   *Expected:* Something up top like "1 package might not install on Windows."
   *Fix:* Treat a package with no Windows build as a blocker, or as "might block". Ask to check packages before the headline, or say "No blockers found *so far*".

3. **Confuses: Project tab.** [shot](shots/persona-beginner-01-drop-windows.png)
   *Happened:* A "Paid" badge sits on the tab that holds the free scan.
   *Fix:* Remove the badge, or say "Free check · Pro fixes".

4. **Confuses: Window header.** [shot](shots/persona-beginner-01-drop-windows.png)
   *Happened:* On my PC the big title is "MacBook Pro", and I wondered if Portal thought I was on a Mac.
   *Fix:* Label it "Connected to MacBook Pro" or "Sending to".

5. **Confuses: Finding details.** [shot](shots/persona-beginner-04-uvloop-opened.png)
   *Happened:* The fixes are code instructions full of jargon.
   *Fix:* Add one plain line per finding ("What this means for you") and a non-coder path ("Ask Sam to…", or "Pro does this").

6. **Confuses: "Needs you" items, blocker report and final report.** [shot](shots/persona-beginner-05-needs-you.png), [shot](shots/persona-beginner-08-final-answer.png)
   *Happened:* No link to Ollama, no clue where API keys come from or what they cost, "run ollama pull" with no place to run it.
   *Fix:* Add a "Get Ollama" button, "Ask the person who sent this for these keys", copyable commands that say where to type them (PowerShell), and a **Start the app** button on the final report.

7. **Confuses: Convert with Pro.** [shot](shots/persona-beginner-06-run-a-command.png)
   *Happened:* Straight to a run prompt: no price, no sign-in, no summary, and Details already applying changes.
   *Fix:* Confirm first: "Convert on this PC: changes 6 files, each undoable, uses your Pro plan ($12/mo). Continue / Cancel."

8. **Confuses: Run a command?** [shot](shots/persona-beginner-06-run-a-command.png)
   *Happened:* An unexplained command, "as you", and the risky choice is the primary button.
   *Fix:* Lead with plain words ("Start the app for 2 minutes to see if it opens"), warn that Windows may ask about the firewall, and give both buttons equal weight.

9. **Annoys: Plans and Account.** [shot](shots/persona-beginner-09-plans.png), [shot](shots/persona-beginner-10-account.png)
   *Happened:* "$20 of AI use" inside a $12 plan, monthly only, and Sam's account covers both computers. Who pays when a friend sends a project? The window's gear did nothing.
   *Fix:* Explain AI use plainly, consider a one-time conversion, and say whose plan pays on each computer.

10. **Annoys: Hand-off note.** [shot](shots/persona-beginner-11-handoff-saved.png)
    *Happened:* "Hand-off note saved", but no location and no explanation.
    *Fix:* "Saved to Downloads · Show in File Explorer · Send this to Sam so he can fix it."

11. **Annoys: Details drawer.** [shot](shots/persona-beginner-02-real-scan-offline-details-open.png)
    *Happened:* Once opened, it stays open across screens and squeezes the report to one row; on the drop screen it's an empty bar.
    *Fix:* Collapse it on screen change, or keep half the height for the report.

12. **Confuses: Scanner, README formatting.** CLI output; no screenshot.
    *Happened:* Run steps in an indented README produced no run-script or python3 findings; a fenced README did. `source .venv/bin/activate` was never flagged.
    *Fix:* Read indented code blocks too, and flag activate lines that only work on a Mac.

13. **Polish: Send files to the AI?** [shot](shots/persona-beginner-07-send-to-ai.png)
    *Happened:* The cost line is clipped under the buttons, "passage names" is jargon, and it's unclear whose $20 this is.
    *Fix:* Keep the cost visible above the buttons and name the plan owner.
