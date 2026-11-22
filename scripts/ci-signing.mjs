// Code signing for a CI build, only when its secrets exist (docs/SIGNING.md).
// The workflow passes every signing secret in as S_<NAME> (empty when not
// set). This sets the variables Tauri reads and SIGN_ARGS="--config <file>"
// (through $GITHUB_ENV) for what's complete; without secrets it changes
// nothing: macOS stays ad-hoc signed, Windows unsigned.
import { execFileSync } from 'node:child_process'
import { appendFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

/** What to set up, from the secrets alone (no side effects) */
export function plan(env, platform) {
  const has = (...names) => names.every((n) => (env[`S_${n}`] ?? '').trim() !== '')
  const get = (n) => env[`S_${n}`].trim()
  const vars = {}
  const files = []
  let config = null
  let signed = null
  let what
  let pfx
  if (platform === 'darwin') {
    if (has('APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_SIGNING_IDENTITY')) {
      for (const n of ['APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_SIGNING_IDENTITY']) vars[n] = get(n)
      config = { bundle: { macOS: { signingIdentity: get('APPLE_SIGNING_IDENTITY') } } }
      signed = 'developer-id'
      what = 'Developer ID'
      if (has('APPLE_API_ISSUER', 'APPLE_API_KEY', 'APPLE_API_KEY_P8')) {
        vars.APPLE_API_ISSUER = get('APPLE_API_ISSUER')
        vars.APPLE_API_KEY = get('APPLE_API_KEY')
        files.push({ name: `AuthKey_${get('APPLE_API_KEY')}.p8`, content: get('APPLE_API_KEY_P8'), env: 'APPLE_API_KEY_PATH' })
        what += ', notarised (App Store Connect API key)'
      } else if (has('APPLE_ID', 'APPLE_PASSWORD', 'APPLE_TEAM_ID')) {
        for (const n of ['APPLE_ID', 'APPLE_PASSWORD', 'APPLE_TEAM_ID']) vars[n] = get(n)
        what += ', notarised (Apple ID)'
      } else {
        what += ', not notarised (add the notarisation secrets)'
      }
    } else {
      what = 'ad-hoc (no Apple signing secrets)'
    }
  } else if (platform === 'win32') {
    if (has('AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET', 'AZURE_TENANT_ID', 'AZURE_SIGNING_ENDPOINT', 'AZURE_SIGNING_ACCOUNT', 'AZURE_CERT_PROFILE')) {
      for (const n of ['AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET', 'AZURE_TENANT_ID']) vars[n] = get(n)
      const cmd = `artifact-signing-cli -e ${get('AZURE_SIGNING_ENDPOINT')} -a ${get('AZURE_SIGNING_ACCOUNT')} -c ${get('AZURE_CERT_PROFILE')} -d SeamDrop %1`
      config = { bundle: { windows: { signCommand: cmd } } }
      signed = 'azure'
      what = 'Azure Artifact Signing'
    } else if (has('WINDOWS_SIGN_COMMAND')) {
      config = { bundle: { windows: { signCommand: get('WINDOWS_SIGN_COMMAND') } } }
      signed = 'command'
      what = 'the WINDOWS_SIGN_COMMAND signer'
    } else if (has('WINDOWS_CERTIFICATE', 'WINDOWS_CERTIFICATE_PASSWORD')) {
      pfx = {
        base64: get('WINDOWS_CERTIFICATE'),
        password: get('WINDOWS_CERTIFICATE_PASSWORD'),
        timestampUrl: has('WINDOWS_TIMESTAMP_URL') ? get('WINDOWS_TIMESTAMP_URL') : 'http://timestamp.digicert.com',
      }
      signed = 'certificate'
      what = 'a code-signing certificate (.pfx)'
    } else {
      what = 'unsigned (no Windows signing secrets)'
    }
  }
  return { vars, files, config, signed, what, pfx }
}

function run() {
  const env = process.env
  const out = (name, value) => appendFileSync(env.GITHUB_ENV, `${name}<<__SEAMDROP_EOF__\n${value}\n__SEAMDROP_EOF__\n`)
  const p = plan(env, process.platform)
  const tmp = env.RUNNER_TEMP
  for (const [k, v] of Object.entries(p.vars)) out(k, v)
  for (const f of p.files) {
    const path = join(tmp, f.name)
    writeFileSync(path, f.content, { mode: 0o600 })
    out(f.env, path)
  }
  let config = p.config
  if (p.pfx) {
    // Into the user's certificate store; Tauri signs by thumbprint.
    const file = join(tmp, 'certificate.pfx')
    writeFileSync(file, Buffer.from(p.pfx.base64, 'base64'))
    const ps = `(Import-PfxCertificate -FilePath '${file}' -CertStoreLocation Cert:\\CurrentUser\\My -Password (ConvertTo-SecureString -String $env:PFX_PASSWORD -Force -AsPlainText)).Thumbprint`
    const thumbprint = execFileSync('powershell', ['-NoProfile', '-Command', ps], {
      encoding: 'utf8',
      env: { ...env, PFX_PASSWORD: p.pfx.password },
    }).trim()
    config = { bundle: { windows: { certificateThumbprint: thumbprint, digestAlgorithm: 'sha256', timestampUrl: p.pfx.timestampUrl } } }
  }
  if (config) {
    const file = join(tmp, 'signing.conf.json')
    writeFileSync(file, JSON.stringify(config))
    out('SIGN_ARGS', `--config ${file}`)
  }
  if (p.signed) out('SEAMDROP_SIGNED', p.signed)
  console.log(`::notice::Code signing: ${p.what}`)
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) run()
