# Releasing SeamDrop

A release is a git tag `vX.Y.Z` (semantic versions). The tag starts `.github/workflows/release.yml`, which builds both apps, signs the update packages, and publishes everything to the public repo **[andrew2grand-lang/seamdrop-releases](https://github.com/andrew2grand-lang/seamdrop-releases)**. Installed apps read `latest.json` from there (`releases/latest/download/latest.json`) every 4 hours and 10 seconds after they start.

The code repo stays private; the releases repo holds only release files and a README, so installed apps never need a token.

## Steps

1. **Pick the version.** Patch (`0.3.1`) for fixes, minor (`0.4.0`) for new features. The protocol version (`PROTOCOL_VERSION` in `crates/portal-core/src/consts.rs`) is separate: bump it only when old and new apps can no longer talk, and say so in the notes ("Update both computers").
2. **Set it everywhere** (Cargo, Tauri, npm, the lockfiles):
   ```bash
   node scripts/version.mjs 0.4.0
   ```
3. **Write the notes** in `CHANGELOG.md` under `## 0.4.0`: two or three short lines in plain words. They're shown in the app's update notice ("What's new: …") and on the release page.
4. **Commit and tag:**
   ```bash
   git commit -am "Release 0.4.0"
   git tag v0.4.0
   git push && git push origin v0.4.0
   ```
5. **Wait for the release run** (Actions → release, about 15 minutes). It checks the tag matches the version and the notes exist, builds macOS (universal) and Windows, and signs the update packages with the updater key.
6. **Publishing:**
   - With the `RELEASES_TOKEN` secret (see "Needs the owner" in `docs/PROGRESS.md`), the run publishes by itself.
   - Without it, the run ends with "Built and signed, not published". From a Mac with the GitHub CLI logged in:
     ```bash
     npm run release:publish -- v0.4.0
     ```
     It downloads that run's files, writes `latest.json`, creates the release in seamdrop-releases, and checks that `latest.json` now says 0.4.0.
7. **Check it:** open the release page; the notes end with the files' SHA-256 checksums and the update key's fingerprint (also in `SHA256SUMS.txt`). The files are `SeamDrop_0.4.0_universal.dmg` and `SeamDrop_0.4.0_x64-setup.exe` (for people), the `.app.tar.gz`, `.sig` and `latest.json` (for the updater). An installed older SeamDrop shows "SeamDrop 0.4.0 is ready" within 4 hours, or at once with Settings → About → Check for updates.

## Test builds (betas)

A tag `vX.Y.Z-beta.N` (say `v0.5.0-beta.1`) is a test build. The steps are the same (`node scripts/version.mjs 0.5.0-beta.1`, notes under `## 0.5.0-beta.1`, tag, push, wait for the run, publish), but publishing differs:

- The release goes up as a GitHub **pre-release**, never marked latest, with `beta.json` instead of `latest.json`.
- `beta.json` on the releases repo's `main` branch is replaced: that file's address is the **beta feed**, compiled into the app (`BETA_FEED` in `src-tauri/src/updates.rs`, `https://raw.githubusercontent.com/andrew2grand-lang/seamdrop-releases/main/beta.json`). GitHub may serve the old file for a few minutes.
- The **stable feed** (`releases/latest/download/latest.json`) ignores pre-releases. The script checks it says the same before and after, and stops with an error if not.

In the app, **Settings → About → Get test builds** (off by default) checks both feeds and takes the higher version, so a beta user still gets the stable release when it comes (0.5.0 is above 0.5.0-beta.2). Turning it off never goes back: the updater only ever offers a version above the running one, so you stay on the beta until a stable release passes it.

**Version numbers in the installers.** macOS takes the version as it is (`CFBundleShortVersionString` 0.5.0-beta.1). Windows needs numbers in the installer's file version, so the bundler gives the NSIS installer `VIProductVersion` 0.5.0.0 (it drops the `-beta.1`), while its product version and the name stay `0.5.0-beta.1`. MSI would refuse a pre-release version; we don't build MSI.

## What the release run needs

| Secret (private repo) | What it is |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | The updater's private key (minisign, made with `tauri signer generate`). Its public key is in `src-tauri/tauri.conf.json` → `plugins.updater.pubkey`. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Its password. |
| `RELEASES_TOKEN` | A fine-grained token that can write to seamdrop-releases only (Needs the owner). Optional: without it, publish from a Mac as above. |
| Apple / Windows signing secrets | Optional, for signed apps: `docs/SIGNING.md`. |

**If the updater's private key is lost, installed apps can't be updated any more**: they only accept packages signed with it. People would have to download and install a new version by hand. Keep the backup (Needs the owner).

## How updates behave in the app

- Checks quietly 10 s after starting and every 4 hours. Nothing appears unless there's an update.
- **Install updates automatically** (Settings → About, on by default): downloads and verifies in the background, then shows "SeamDrop 0.4.0 is ready. What's new: …" with **Restart SeamDrop to update** and **Later**. Off: it shows "is available" with **Update and restart SeamDrop**.
- Never restarts by itself, and never during a transfer: after **Restart SeamDrop to update** is pressed during one, it says "SeamDrop restarts to update when the transfer finishes" and does so when it ends. A paused transfer doesn't block the restart: it continues after it.
- The signature is checked before anything is installed; the signed version must match the announced one (`requireSignedVersion`). Failures say what happened in plain words ("Couldn't reach the update server…", "The update didn't download correctly, so it was thrown away. Nothing on this computer changed…") with **Try again** and **OK**, and the current version keeps running.
- Windows installs per machine, so each update shows the administrator prompt (the installer runs in passive mode: a progress bar, no questions).
- Support: starting the app with `--update-now` checks and installs at once (as if Restart SeamDrop to update was pressed).

## Local builds

`npx tauri build` also makes the signed update package, so it needs the key:

```bash
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/seamdrop-updater.key)" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat ~/.tauri/seamdrop-updater.password)" npx tauri build
```

Without the key, build without update packages: `npx tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'`.
