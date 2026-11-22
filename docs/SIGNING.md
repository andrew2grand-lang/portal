# Signing SeamDrop

Today SeamDrop is **ad-hoc signed on macOS** and **unsigned on Windows**. That's why people see "unidentified developer" (Mac) and "Windows protected your PC" (SmartScreen), and why the README's install steps have workarounds. CI is ready to sign properly: add the secrets below and the next build is signed. Nothing else changes; without the secrets, builds stay as they are.

How it works: `build.yml` and `release.yml` run `scripts/ci-signing.mjs` before building. It looks at which signing secrets exist, sets the variables Tauri reads, and passes a signing config to the build. Each run's log says what it did ("Code signing: Developer ID, notarised (App Store Connect API key)" or "ad-hoc (no Apple signing secrets)").

Signing is separate from the **updater key** (`docs/RELEASING.md`): that one proves updates come from us and is already in use; these prove the app comes from us to macOS and Windows.

## macOS: Developer ID + notarisation

**What to buy:** the [Apple Developer Program](https://developer.apple.com/programs/), **US$99 a year** (about £79). As an individual, the app shows your name; as an organisation, the company's (needs a D-U-N-S number, free, takes a few days).

**What it fixes:** macOS opens SeamDrop without the Terminal step, and keeps its Local Network permission across updates (an ad-hoc signed app looks like a new app after every update, so macOS may ask again). It also lets the private key move to the Keychain later.

**Make the certificate** (once, on a Mac): Xcode → Settings → Accounts → your team → Manage Certificates → **+** → **Developer ID Application**. Then in Keychain Access, right-click it → Export → `.p12` with a password.

**Secrets** (GitHub → the private repo → Settings → Secrets and variables → Actions → New repository secret):

| Secret | Value |
|---|---|
| `APPLE_CERTIFICATE` | `base64 -i DeveloperID.p12 \| pbcopy`, then paste |
| `APPLE_CERTIFICATE_PASSWORD` | the `.p12` password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Sam Example … (ABCDE12345)` (from `security find-identity -v -p codesigning`) |

**Notarisation**, one of:

| Option | Secrets |
|---|---|
| App Store Connect API key (recommended) | `APPLE_API_ISSUER` (Issuer ID), `APPLE_API_KEY` (Key ID), `APPLE_API_KEY_P8` (the contents of `AuthKey_<id>.p8`). Make it at App Store Connect → Users and Access → Integrations → Team Keys, role Developer. |
| Apple ID | `APPLE_ID`, `APPLE_PASSWORD` (an [app-specific password](https://account.apple.com), not your real one), `APPLE_TEAM_ID` |

**Check a build:** download the `.dmg` from the run or the release, open it, then:

```bash
codesign -dv --verbose=2 /Volumes/SeamDrop/SeamDrop.app   # Authority=Developer ID Application: …
spctl -a -vv /Volumes/SeamDrop/SeamDrop.app               # accepted, source=Notarized Developer ID
xcrun stapler validate /Volumes/SeamDrop/SeamDrop.app      # The validate action worked!
```

CI checks the first and warns if the app isn't notarised.

## Windows: two ways

Since June 2023, code-signing certificates' private keys must live in a hardware token or a cloud service, so "a .pfx file in a secret" only works for older certificates. The practical choices for CI:

### A. Azure Artifact Signing (formerly Trusted Signing): cheapest, made for CI

- **Cost:** Basic plan **US$9.99 a month** (5,000 signatures), plus an Azure account.
- **Who can use it:** organisations in the US, Canada, the EU and the **UK** (the company must be verifiable, e.g. at Companies House; Microsoft has at times limited new sign-ups to organisations with a few years of history). **Individuals only in the US and Canada**, so not as a UK individual. Check [the current eligibility](https://learn.microsoft.com/azure/artifact-signing/) before paying.
- **Set up:** create an Artifact Signing account and a Public Trust certificate profile (identity validation takes days), then an App Registration (Entra ID) with the **Artifact Signing Certificate Profile Signer** role on the account, and a client secret.
- **Secrets:** `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`, `AZURE_SIGNING_ENDPOINT` (e.g. `https://weu.codesigning.azure.net`), `AZURE_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE`. CI then installs `artifact-signing-cli` and Tauri signs the app and installer with it.

### B. A standard (OV) code-signing certificate

- **Cost:** about **£200–£450 a year** from a CA (Sectigo, DigiCert, SSL.com, GlobalSign), for an individual or a company. EV certificates cost more and no longer skip SmartScreen's reputation check, so OV is enough.
- **In CI** the key has to be reachable from the runner, so pick a CA with a cloud signing service (SSL.com eSigner, DigiCert KeyLocker, or the certificate in Azure Key Vault) and put its signing command in the secret `WINDOWS_SIGN_COMMAND`, with `%1` where the file goes, e.g. an AzureSignTool or CodeSignTool command line. The CA's documentation gives the exact command.
- **Older certificate as a .pfx** (issued before June 2023): `WINDOWS_CERTIFICATE` (`certutil -encode certificate.pfx cert.txt`, the base64 text), `WINDOWS_CERTIFICATE_PASSWORD`, and optionally `WINDOWS_TIMESTAMP_URL` (your CA's; default `http://timestamp.digicert.com`).

If several are set, CI uses Azure first, then `WINDOWS_SIGN_COMMAND`, then the `.pfx`.

**What it fixes:** the "Unknown publisher" in the administrator prompt becomes the signer's name. SmartScreen's "Windows protected your PC" fades as the signed app builds a download reputation (that takes some downloads, whichever certificate you buy).

**Check a build:** right-click `SeamDrop_x.y.z_x64-setup.exe` → Properties → **Digital Signatures** (signer, timestamp), or in PowerShell:

```powershell
Get-AuthenticodeSignature .\SeamDrop_0.4.0_x64-setup.exe | Format-List Status, SignerCertificate
```

`Status : Valid`. CI checks this whenever Windows signing is set up.

## What it costs, all in

| | Per year |
|---|---|
| Apple Developer Program | US$99 (≈ £79) |
| Azure Artifact Signing (UK company) | ≈ US$120 (≈ £95) |
| or an OV certificate | ≈ £200–£450 (+ the CA's cloud signing fee, if any) |
