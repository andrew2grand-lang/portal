// Publishes a release to the public repo andrew2grand-lang/seamdrop-releases:
// the installers, the signed update packages, and latest.json (what the app
// checks). Used by CI (release.yml, with RELEASES_TOKEN) and, until that token
// exists, from a Mac with the GitHub CLI logged in (docs/RELEASING.md):
//
//   node scripts/publish-release.mjs v0.4.0              download that tag's release-run artifacts, publish
//   node scripts/publish-release.mjs v0.4.0 --dir <dir>  publish files already in <dir> (CI)
//   node scripts/publish-release.mjs v0.4.0 --dry-run    build latest.json and list what would go up
//
// A test build (v0.5.0-beta.1, Mission 05 §8.2) goes up as a GitHub
// pre-release, never marked latest, with beta.json instead of latest.json;
// then beta.json in the repo's main branch (the beta feed's fixed URL) is
// replaced. The stable feed, releases/latest/download/latest.json, ignores
// pre-releases, and this checks it still says what it said before.
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { copyFileSync, mkdtempSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const REPO = 'andrew2grand-lang/seamdrop-releases'
const args = process.argv.slice(2)
const tag = args.find((a) => /^v\d+\.\d+\.\d+(-beta\.\d+)?$/.test(a))
const dirArg = args.includes('--dir') ? args[args.indexOf('--dir') + 1] : null
const dryRun = args.includes('--dry-run')
if (!tag) {
  console.error('usage: node scripts/publish-release.mjs vX.Y.Z[-beta.N] [--dir <artifacts>] [--dry-run]')
  process.exit(2)
}
const version = tag.slice(1)
const beta = version.includes('-beta.')
const BETA_FEED = `https://raw.githubusercontent.com/${REPO}/main/beta.json`
const STABLE_FEED = `https://github.com/${REPO}/releases/latest/download/latest.json`
const run = (cmd, a, opts = {}) => execFileSync(cmd, a, { encoding: 'utf8', ...opts })

// The source tree must say this version (and have its notes).
run('node', ['scripts/version.mjs', '--check', version], { stdio: 'inherit' })
const notes = run('node', ['scripts/version.mjs', '--notes', version]).trim()
const plain = run('node', ['scripts/version.mjs', '--notes', '--plain', version]).trim()

// 1. The files: from --dir, or from the release workflow's run for this tag.
let dir = dirArg
if (!dir) {
  const runs = JSON.parse(run('gh', ['run', 'list', '--workflow', 'release.yml', '--limit', '30', '--json', 'databaseId,headBranch,conclusion,status']))
  const found = runs.find((r) => r.headBranch === tag)
  if (!found) throw new Error(`No release.yml run for ${tag}. Push the tag first (docs/RELEASING.md).`)
  if (found.status !== 'completed' || found.conclusion !== 'success') {
    throw new Error(`The release run for ${tag} is ${found.status} (${found.conclusion ?? 'running'}). Wait for it to pass.`)
  }
  dir = mkdtempSync(join(tmpdir(), 'seamdrop-release-'))
  run('gh', ['run', 'download', String(found.databaseId), '--dir', dir], { stdio: 'inherit' })
}

function walk(d) {
  return readdirSync(d).flatMap((n) => {
    const p = join(d, n)
    return statSync(p).isDirectory() ? walk(p) : [p]
  })
}
const all = walk(dir)
const pick = (re, what) => {
  const hit = all.find((p) => re.test(p))
  if (!hit) throw new Error(`Missing ${what} in ${dir}`)
  return hit
}
const dmg = pick(/SeamDrop_[\w.-]+_universal\.dmg$/, 'the Mac disk image')
const macUpdate = pick(/SeamDrop\.app\.tar\.gz$/, 'the Mac update package')
const macSig = pick(/SeamDrop\.app\.tar\.gz\.sig$/, 'the Mac update signature')
const exe = pick(/SeamDrop_[\w.-]+_x64-setup\.exe$/, 'the Windows installer')
const exeSig = pick(/SeamDrop_[\w.-]+_x64-setup\.exe\.sig$/, 'the Windows installer signature')
for (const f of [dmg, exe]) {
  if (!f.includes(`_${version}_`)) throw new Error(`${f} isn't version ${version}`)
}

// 2. Names on the release page, and latest.json pointing at them.
const out = mkdtempSync(join(tmpdir(), 'seamdrop-publish-'))
const assets = {
  [`SeamDrop_${version}_universal.dmg`]: dmg,
  [`SeamDrop_${version}_universal.app.tar.gz`]: macUpdate,
  [`SeamDrop_${version}_universal.app.tar.gz.sig`]: macSig,
  [`SeamDrop_${version}_x64-setup.exe`]: exe,
  [`SeamDrop_${version}_x64-setup.exe.sig`]: exeSig,
}
for (const [name, from] of Object.entries(assets)) copyFileSync(from, join(out, name))
const url = (name) => `https://github.com/${REPO}/releases/download/${tag}/${name}`
const mac = { signature: readFileSync(macSig, 'utf8').trim(), url: url(`SeamDrop_${version}_universal.app.tar.gz`) }
const win = { signature: readFileSync(exeSig, 'utf8').trim(), url: url(`SeamDrop_${version}_x64-setup.exe`) }
const latest = {
  version,
  notes: plain,
  pub_date: new Date().toISOString().replace(/\.\d+Z$/, 'Z'),
  // One universal Mac build serves both chips; the -app/-nsis keys are what
  // the updater looks for first (by installer type).
  platforms: {
    'darwin-aarch64': mac,
    'darwin-x86_64': mac,
    'darwin-aarch64-app': mac,
    'darwin-x86_64-app': mac,
    'windows-x86_64': win,
    'windows-x86_64-nsis': win,
  },
}
// A beta's feed file is beta.json: the stable feed's name never appears on a pre-release
const feedName = beta ? 'beta.json' : 'latest.json'
writeFileSync(join(out, feedName), JSON.stringify(latest, null, 2) + '\n')

// Checksums, so a first download can be checked before anything is signed;
// and the update key's fingerprint, which installed apps check updates with.
const sha256 = (p) => createHash('sha256').update(readFileSync(p)).digest('hex')
const sums = Object.keys(assets)
  .filter((n) => !n.endsWith('.sig'))
  .map((n) => `${sha256(join(out, n))}  ${n}`)
  .join('\n')
writeFileSync(join(out, 'SHA256SUMS.txt'), sums + '\n')
const pubkey = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).plugins.updater.pubkey
const keyPrint = createHash('sha256').update(pubkey).digest('hex').slice(0, 32)
const howToCheck = [
  '## Check your download',
  '',
  'On a Mac: `shasum -a 256 SeamDrop_*.dmg`. On Windows (PowerShell): `Get-FileHash .\\SeamDrop_*.exe`. The result should match:',
  '',
  '```',
  sums,
  '```',
  '',
  `Updates are checked against the SeamDrop update key, fingerprint \`${keyPrint}\` (SHA-256 of the public key in the app).`,
].join('\n')
writeFileSync(join(out, 'notes.md'), notes + '\n\n' + howToCheck + '\n')
console.log(`Prepared ${tag} in ${out}:`)
for (const n of readdirSync(out)) console.log(`  ${n}`)
if (dryRun) process.exit(0)

// 3. The release (the tag is made in the releases repo, on its README commit).
// The stable feed before, so a beta can show it left it alone
const feedNow = (url) => {
  try {
    return JSON.parse(run('curl', ['-fsSL', url])).version
  } catch {
    return null
  }
}
const stableBefore = beta ? feedNow(STABLE_FEED) : null
const files = [...Object.keys(assets), feedName, 'SHA256SUMS.txt'].map((n) => join(out, n))
const kind = beta ? ['--prerelease', '--latest=false'] : ['--latest']
run('gh', ['release', 'create', tag, ...files, '--repo', REPO, '--title', `SeamDrop ${version}`, '--notes-file', join(out, 'notes.md'), ...kind], {
  stdio: 'inherit',
})

if (beta) {
  // 4. The beta feed: beta.json on main, replaced (its fixed URL is in the app)
  let sha = null
  try {
    sha = JSON.parse(run('gh', ['api', `repos/${REPO}/contents/beta.json`])).sha
  } catch {
    // the first beta: no file yet
  }
  const body = ['-X', 'PUT', `repos/${REPO}/contents/beta.json`, '-f', `message=Beta feed: ${version}`, '-f', `content=${readFileSync(join(out, 'beta.json')).toString('base64')}`]
  run('gh', ['api', ...body, ...(sha ? ['-f', `sha=${sha}`] : [])], { stdio: 'ignore' })
  const fed = JSON.parse(Buffer.from(JSON.parse(run('gh', ['api', `repos/${REPO}/contents/beta.json`])).content, 'base64').toString()).version
  if (fed !== version) throw new Error(`beta.json says ${fed}, not ${version}`)
  const stableAfter = feedNow(STABLE_FEED)
  if (stableAfter !== stableBefore) throw new Error(`The stable feed changed: ${stableBefore} → ${stableAfter}. Look at the releases page.`)
  console.log(`Published ${version} as a pre-release. The beta feed says ${fed} (${BETA_FEED}; GitHub may serve the old file for a few minutes); the stable feed still says ${stableAfter}.`)
  process.exit(0)
}

// 4. What installed apps will read now.
const check = run('curl', ['-fsSL', STABLE_FEED])
const live = JSON.parse(check).version
if (live !== version) throw new Error(`latest.json still says ${live}; GitHub may need a minute.`)
console.log(`Published: installed apps now see ${version}.`)
