# Persona review: non-technical office worker

## Who I am and what I tried

I work in an office. I need to get our "Dossier Clients – Hélène" folder from my Mac to my colleague's Windows PC. It holds Word and PDF files (some in subfolders like "Factures 2026" and "Contrats"), meeting photos (.JPG, .jpeg, one .HEIC), a payments spreadsheet, a "À LIRE.txt" note, and "renommer factures.sh", a script Marc left that renames the invoices. I don't know what Python, a package or a "dependency" is. I want the folder to arrive, the script to keep working, my clients' files to stay private, and I don't want to pay for something I'll do once.

I built that folder (plus a copy where the script is Python, `renommer_factures.py`), scanned both with the lab's "Scan a real folder", and walked the lab's screens. I had to use the lab panel to reach the plan and AI screens, because nothing inside Portal led me there.

## Task 1: Files or Project?

**Finished: partly · 3 steps**

Portal opened on **Project** with a "Paid" badge. My folder isn't a "project". To me it's just files, so I clicked **Files**. Nothing happened. I clicked the gear to look for help. Nothing. I clicked "Details" and got an empty grey bar. Then I read the drop zone: "Free scan" sits next to the "Paid" badge, so I couldn't tell whether this tab costs money.

- Confused by: a "Paid" tab that says "Free scan"; a Files tab that doesn't respond.
- Words I didn't know: *project, Pro conversion, Desk PC* (is that my colleague's PC?).
- Expected: to be asked "What are you sending?" Got: a tab that was already picked for me, and the other tab doesn't work.

![Entry](shots/persona-nontechnical-01-entry.png)

## Task 2: Will Marc's script still work on the PC?

**Finished: yes, but I got the wrong answer · 4 steps**

Scan of the `.sh` folder: **"No problems found for Windows."** I opened the one item, "Scripts marked executable", and read: *"Windows doesn't keep the Unix 'executable' flag. Nothing breaks on Windows…"*. So I'd tell my colleague the script works. I don't think it does. Windows can't just run a `.sh` file, and the script opens a Mac folder and runs `open`. In the lab's demo project, Portal itself flags a `run.sh` as "Run script has no Windows version". My script got a green light.

The Python copy said **"No blockers found. 2 things will need a fix."** That's good news and bad news in the same sentence. The fix says *"Build the path from the project folder… Path(__file__).parent / "data", Path.home(), or path.join(__dirname, "data") in Node."* I can't do anything with that.

- Words I didn't know: *Unix, executable flag, chmod +x, blocker, Path(__file__), __dirname, Node, API, library swap, renommer_factures.py:3* (the ":3" means a line number?).
- Expected: "This script won't run on Windows. Here's what that means for you." Got: a reassurance for the `.sh` file and programmer instructions for the `.py` file.

![Shell scan](shots/persona-nontechnical-03-script-finding.png)
![Python fix](shots/persona-nontechnical-04-python-fix.png)

## Task 3: Send the folder

**Finished: no · 2 steps**

On the report I pressed **Send as files**. Nothing happened: no "where should it go", no progress bar, no "arrived". The Files tab doesn't respond either. I never saw what my colleague gets, or where it lands on her PC. The report also never mentions my documents or photos. It only talks about the script and "0 B won't be sent". Zero bytes of *what*? Did something of mine get left out?

- Words I didn't know: *OS clutter, .DS_Store, Thumbs.db, Static, hand-off note*.
- Expected: "11 documents and 3 photos will open fine on Windows. Send?" Got: a silent button.

![Shell scan report](shots/persona-nontechnical-02-scan-shell.png)

## Task 4: Do I need to pay?

**Finished: partly · 4 steps (only via the lab panel)**

On the Python copy I pressed **Convert with Pro**, hoping to see a price. Instead I landed on "This project wants to run `uvicorn app.main:app --port 8000`. Run it?". That isn't my project, and the window now said "MacBook Pro". I found prices only by using the lab's "Plans" link: Free $0, Pro **$12 a month** "with up to $20 of AI use", Own key $5 a month. I want to fix one small script once, and every option is a monthly plan. "$20 of AI use" means nothing to me: is one fix $0.05 or $15?

- Words I didn't know: *conversion, AI use, allowance, Own key, AI key, API key, keychain, AI provider*.
- Expected: a price on the button, or "fixing this script ≈ one-time $X". Got: a jump to a stranger's project, and monthly plans on a separate screen.

![Convert jumps](shots/persona-nontechnical-05-convert-with-pro.png)
![Plans](shots/persona-nontechnical-06-plans.png)

## Task 5: Do my clients' files leave my computer?

**Finished: partly · 3 screens**

The scanning screen says "nothing leaves this Mac", and that's the sentence I wanted. But the scan took under a second, so I barely saw it. The report itself only says "Nothing was run or changed." It also tells me "AI use for a conversion (estimate): None" on a folder of invoices. Why is AI being mentioned at all? The AI screen says "Send 4 files to the AI… No secrets", but it doesn't say *which* AI, which company, or whether they keep my files. To me, my invoices are secrets. I also can't tell whether files go straight to the PC over our Wi-Fi or through the internet.

- Words I didn't know: *the AI* (whose?), *.env, .venv, .gitignore, net offline*.
- Expected: "Your files stay on this Mac and go straight to Desk PC." Got: that promise only on a screen that flashes by.

![Scanning](shots/persona-nontechnical-08-scanning.png)
![AI consent](shots/persona-nontechnical-07-ai-consent.png)

## Findings

### 1. A Mac-only shell script is reported as "No problems found". Severity: blocks use
- **Screen:** Blocker report (real scan of my `.sh` folder). [shot](shots/persona-nontechnical-03-script-finding.png)
- **Happened:** "No problems found for Windows", and the script finding says "Nothing breaks on Windows."
- **Expected:** a warning that this script won't run on the PC as it is.
- **Suggest:** flag any `.sh` as "won't run on Windows" (as `run.sh` is in the demo project), and say in plain words what stops working ("the invoice-renaming script").

### 2. The Files tab, the gear and "Send as files" don't respond. Severity: blocks use
- **Screen:** Entry and report. [shot](shots/persona-nontechnical-01-entry.png)
- **Happened:** clicks did nothing, so I had no way to actually send.
- **Expected:** a send flow that ends with "arrived on Desk PC".
- **Suggest:** prototype the Files path, or mark it "not in this demo".

### 3. Fixes are written for programmers. Severity: confuses
- **Screen:** Finding opened (Python copy). [shot](shots/persona-nontechnical-04-python-fix.png)
- **Happened:** code snippets, file:line references, Node advice for a Python file.
- **Expected:** "This script looks for Marc's Mac folder; on the PC it needs to point to the new folder."
- **Suggest:** lead with a plain-language line, and tuck the code under "For whoever maintains it".

### 4. Mixed verdicts. Severity: confuses
- **Screen:** Report headline. [shot](shots/persona-nontechnical-04-python-fix.png)
- **Happened:** "No blockers found. 2 things will need a fix."
- **Expected:** one clear answer, e.g. "Your files will arrive fine. 1 script needs changes to work."
- **Suggest:** separate the files verdict from the script verdict.

### 5. The report ignores my documents and photos. Severity: confuses
- **Screen:** Report. [shot](shots/persona-nontechnical-02-scan-shell.png)
- **Happened:** only the script and "0 B won't be sent" are mentioned.
- **Expected:** "11 documents and 3 photos open on Windows", plus anything that might not (an iPhone `.HEIC` photo, for example).
- **Suggest:** add a short "your files" line, and hide "0 B won't be sent" when it's zero.

### 6. "Project · Paid" next to "Free scan". Severity: confuses
- **Screen:** Entry. [shot](shots/persona-nontechnical-01-entry.png)
- **Suggest:** move "Paid" from the tab to the Pro conversion box only.

### 7. Price only after a detour, and only monthly. Severity: confuses
- **Screen:** Convert with Pro / Plans. [shot](shots/persona-nontechnical-06-plans.png)
- **Happened:** no price on the button. The lab jumped to an unrelated project's "Run it?" screen. Plans are monthly, and AI is priced in dollars of "AI use".
- **Suggest:** show the expected cost of *this* fix on the button, and consider a one-off option. Translate "AI use" into "about N fixes like this".

### 8. Privacy message is fleeting, and "the AI" is unnamed. Severity: confuses
- **Screens:** Scanning, report, AI consent. [shot](shots/persona-nontechnical-07-ai-consent.png)
- **Suggest:** repeat "nothing left this Mac" on the report. On the AI screen, name the company and say whether files are kept. Say how files travel to the PC (direct over Wi-Fi, or through the internet).

### 9. AI is mentioned for a folder with nothing to fix. Severity: annoys
- **Screen:** Report (documents scenario), "AI use for a conversion (estimate): None." [shot](shots/persona-nontechnical-09-documents-report.png)
- **Suggest:** hide the AI line when there's nothing to convert.

### 10. Jargon in visible places. Severity: polish
"Finding ecosystems", "Static", "OS clutter", "net offline" in Details, "hand-off note". **Suggest:** plain words, or a hover explanation.
