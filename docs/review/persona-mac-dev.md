# Persona review: Mac developer shipping a local-AI app to a client's PC

## Who I am and what I tried

I'm a Mac developer. I built "localmind" (FastAPI, an MLX runner, Ollama over HTTP at `localhost:11434`, a Vite web UI, `scripts/start.sh` and `setup.sh` that use `brew`/`source`/`sed -i ''`, a `.env` with OpenAI/Anthropic/HF keys, a 2 GB `.gguf` in a gitignored `models/`). It has to run on a client's Windows PC by Friday. I scanned it for real with `portal-scan` (offline, `--online`, `--json`) and the lab's "Scan a real folder", then walked the demo conversion screens. I also made a copy with keys hard-coded in `app/keys.py` and an SSH key in `config/deploy_key`.

## Task 1: Scan for Windows and judge the findings

**Finished:** partly · **Steps:** CLI 1; lab 3 (paste path, For Windows, Scan) + scrolling · [shot](shots/persona-mac-dev-01-real-report.png)

The headline was fast and blunt, which I liked: "2 things block this on Windows, 8 need a fix and 1 needs you." MLX, `fcntl`, `open`, `SIGHUP`, `aux.txt`, the colon file name, the symlink, `rm -rf` in npm and `make` were all right, with file:line evidence. Then I started counting what my client would actually hit:

- **My start path wasn't checked.** `make dev` was flagged, but the script it runs (`scripts/start.sh`: `brew services`, `source .venv/bin/activate`, `open`) got nothing. "How it's run here" never lists it. When I copied it to `run.sh` *and* mentioned `./run.sh` in the README, "Run script has no Windows version" appeared. So it depends on where the script lives.
- **Ollama was invisible.** I call it over HTTP and `OLLAMA_HOST` is in `.env`. Only after I added the `ollama` pip package did "Needs Ollama running" show up.
- **uvloop:** offline, not mentioned at all. With `--online` it's "will need a fix … if it builds there at all". uvloop doesn't support Windows, so for me that's a blocker.
- Also missed: `/tmp/localmind.lock` (no `/tmp` on Windows), `pbcopy`, `fsevents` in devDependencies, the launchd `.plist`, and a `/Users/dana/...` path in `.env`. Yet the lock file got flagged for UTF-8, even though it's never written.
- `--online` said "mlx isn't on PyPI (a private or renamed package)". mlx *is* on PyPI; my pin `0.18.0` doesn't exist. My mistake, but the wording made me doubt the tool, not my pin.

## Task 2: Check what happens to secrets and big folders

**Finished:** partly · **Steps:** 2 (scroll, open "Secrets…") · [shot](shots/persona-mac-dev-02-secrets-models.png)

"Secrets stay on this Mac … Portal only read their names, never the values" is exactly what I wanted to read. Then I checked it:

- `app/keys.py` with a literal `sk-proj-…` and `hf_…` token, plus `config/deploy_key` (an OpenSSH private key), weren't mentioned anywhere. Both would be sent. The Details log still says "secret values were not read".
- `.env.local` and `config/secrets.json` show up under "Secrets stay on this Mac" but not in the "2.15 GB won't be sent" list. So which one is right? `OPENAI_API_KEY` is listed twice ("holds 6 keys").
- I found three different promises: "never sends secrets without asking" (so it can, if asked?), "Secrets stay on this Mac", and "Portal never copies secrets".
- `models/ 2.15 GB` is filed under "ignored by the project or clutter". It's my model, and the app is useless without it. Nothing tells me how to get it to the PC. I also can't see what *will* be sent: my 50 MB `data/embeddings.sqlite` goes along silently.

## Task 3: Decide whether to trust and pay for the conversion

**Finished:** no · **Steps:** 3 (Convert with Pro, Plans, Own key) · [shot](shots/persona-mac-dev-04-convert-jump.png), [shot](shots/persona-mac-dev-08-plans.png)

The Pro box says "The 8 fixes above happen on Desk PC, then Portal checks that it installs, passes its tests and starts." What about my 2 blockers? Will Pro swap MLX, or do I pay $12 and still be blocked? I have no tests either. "AI use … Often" comes with no dollar figure. "The AI" is never named, so I can't tell whose AI gets my code or whether it's kept.

When I pressed "Convert with Pro" there was no sign-in and no plan step. The window flipped to Windows chrome titled "MacBook Pro" and asked "Run it?" for `uvicorn … --port 8000` "from run.sh, line 4". My project has no `run.sh`. I know the lab says conversion is a demo, but I couldn't tell whether I or my client approves this on the PC.

## Task 4: Review what the conversion changed and undo one (demo)

**Finished:** yes · **Steps:** 2 (Review the 7 changes, Undo) · [shot](shots/persona-mac-dev-07-undone.png)

This is the best part. The diffs are real code, every change has an Undo, and "By the AI" marks the one change the AI made rather than a known fix. The repair log ("Attempt 1 … The test passed, but the index on MacBook Pro uses / names … Undone.") made me trust it more. Still:

- After Undo the headline still says "Runs on Windows" and "7 changes". It says "the checks run again", but I see no re-check status.
- Added lines are tinted red/orange, and I read them as deletions at first.
- The changes live in `C:\Users\Sam\Projects\local-chat`. I can't get them back into my Mac repo as a patch or branch. "Open in File Explorer" is the only exit.

## Task 5: Hand the remaining work to the client

**Finished:** partly · **Steps:** 1 · [shot](shots/persona-mac-dev-03-handoff-saved.png), [shot](shots/persona-mac-dev-06-final.png)

"Install Ollama on this PC. Then run ollama pull llama3.2" is exactly what a client needs. But "Export hand-off note" just turns into "Hand-off note saved". I don't know where it went, what format it's in or what's in it, so I can't check it before forwarding. The CLI has no way to write one (`"handoff": null` in the JSON). The final headline "Runs on Windows" sits above "Answers from the model: Couldn't check". For a local-AI app, the answers are the whole app.

## Findings

**1. Hard-coded keys and private keys are missed while the screen promises secrets stay home.** *Blocks use.* Report, "Secrets stay on this Mac" ([shot](shots/persona-mac-dev-02-secrets-models.png)). Expected: `app/keys.py` (`sk-proj-…`, `hf_…`) and `config/deploy_key` flagged and kept back. Happened: neither is mentioned, and both are sent. *Suggest:* scan file contents for key patterns and PEM headers. Scope the promise to "secrets in .env-style files", and show a count of files checked.

**2. The real start path and Ollama aren't detected.** *Blocks use.* Report and CLI ([shot](shots/persona-mac-dev-01-real-report.png)). Expected: findings for `scripts/start.sh`/`setup.sh` and "Needs Ollama". Happened: the script was found only at `run.sh` with a README mention, and Ollama only via the pip package. *Suggest:* follow Makefile/npm targets into the scripts they call, treat every `*.sh` as a run script, and detect `11434`/`OLLAMA_HOST`/`ollama serve`.

**3. Pro's promise is silent on blockers.** *Confuses.* Report, "Pro converts it for you" ([shot](shots/persona-mac-dev-02-secrets-models.png)). Expected: "Pro fixes 8; MLX and fcntl need X". Happened: "The 8 fixes above". *Suggest:* say per blocker whether Pro handles it, and drop "passes its tests" when there are none.

**4. Big folders the app needs are called clutter, and there's no "will be sent" list.** *Confuses.* Same screen. *Suggest:* mark `models/` (weights, `.gguf`) as "needed at runtime, not sent", offer to send it or add it to the hand-off, and add an expandable list of what will be sent, with sizes.

**5. Secret lists and promises disagree.** *Confuses.* Same screen. `.env.local` and `secrets.json` are missing from "won't be sent", a key is listed twice, and there are three different wordings. *Suggest:* one table (file, keys, sent: no) and one sentence of policy.

**6. "Convert with Pro" skips account and plan, and switches machines without saying so.** *Confuses.* Consent screen ([shot](shots/persona-mac-dev-04-convert-jump.png)). *Suggest:* show sign-in/plan first, and a one-line "Now on Desk PC: Sam approves here" before a PC-side consent.

**7. "The AI" is unnamed.** *Confuses.* Send files to the AI?, Plans ([shot](shots/persona-mac-dev-05-send-ai.png)). *Suggest:* name the provider and say whether code is kept or trained on. List which providers work with Own key.

**8. The final headline overclaims for AI apps.** *Confuses.* Final report ([shot](shots/persona-mac-dev-06-final.png)). *Suggest:* "Installs and starts on Windows; model answers not checked" when the core check is Couldn't check.

**9. Hand-off note and plan are saved somewhere unknown.** *Annoys.* ([shot](shots/persona-mac-dev-03-handoff-saved.png)) *Suggest:* show the path and a Reveal/Preview, say the format, and add `portal-scan --handoff out.md`.

**10. Undo gives no re-check feedback, additions are red, and nothing comes back to the Mac.** *Annoys.* Reviewing changes ([shot](shots/persona-mac-dev-07-undone.png)). *Suggest:* update the headline and counts after Undo and show "re-checking…", color additions green, and add "Export as patch / git branch".

**11. Severity and wording nits.** *Polish.* uvloop should be a blocker, not "if it builds there at all". Say "version 0.18.0 not found on PyPI" instead of "isn't on PyPI (private or renamed)". "Tier: Apps" clashes with the paid tier. The "Paid" badge sits on the tab whose first action is the free scan. "Needs you 1" while the row says "3 places". The AI-use estimate flipped Often→Sometimes when I deleted one symlink. UTF-8 got flagged on a lock file while its `/tmp` path was missed.
