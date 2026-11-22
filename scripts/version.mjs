// The app's version lives in four files; this keeps them the same.
// A version is X.Y.Z, or X.Y.Z-beta.N for a test build (Mission 05 §8.2).
//   node scripts/version.mjs 0.4.0          set it everywhere (then commit)
//   node scripts/version.mjs --check 0.4.0  exit 1 unless every file says 0.4.0
//   node scripts/version.mjs --notes 0.4.0  print that version's notes from CHANGELOG.md
//                                           (--plain: one paragraph, for the update notice)
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'

const SEMVER = /^\d+\.\d+\.\d+(-beta\.\d+)?$/
const args = process.argv.slice(2)
const flag = (f) => args.includes(f)
const version = args.find((a) => !a.startsWith('--'))
if (!version || !SEMVER.test(version)) {
  console.error('usage: node scripts/version.mjs [--check|--notes [--plain]] X.Y.Z[-beta.N]')
  process.exit(2)
}

const files = {
  'Cargo.toml': {
    get: (s) => s.match(/\[workspace\.package\][^[]*?\nversion = "([^"]+)"/)?.[1],
    set: (s, v) => s.replace(/(\[workspace\.package\][^[]*?\nversion = ")[^"]+"/, `$1${v}"`),
  },
  'src-tauri/tauri.conf.json': {
    get: (s) => JSON.parse(s).version,
    set: (s, v) => s.replace(/("version": ")[^"]+(")/, `$1${v}$2`),
  },
  'package.json': {
    get: (s) => JSON.parse(s).version,
    set: (s, v) => s.replace(/("version": ")[^"]+(")/, `$1${v}$2`),
  },
  'package-lock.json': {
    get: (s) => JSON.parse(s).version,
    set: (s, v) => {
      const j = JSON.parse(s)
      j.version = v
      if (j.packages?.['']) j.packages[''].version = v
      return JSON.stringify(j, null, 2) + '\n'
    },
  },
}

/** The CHANGELOG.md section for `v`: the lines under "## v" up to the next "## " */
function notes(v) {
  const text = readFileSync('CHANGELOG.md', 'utf8')
  const start = text.indexOf(`\n## ${v}\n`)
  if (start < 0) return null
  const rest = text.slice(start + `\n## ${v}\n`.length)
  const end = rest.search(/\n## /)
  return (end < 0 ? rest : rest.slice(0, end)).trim()
}

if (flag('--notes')) {
  const n = notes(version)
  if (!n) {
    console.error(`CHANGELOG.md has no "## ${version}" section`)
    process.exit(1)
  }
  // The notice shows one or two lines: bullets become sentences.
  console.log(flag('--plain') ? n.split('\n').map((l) => l.replace(/^[-*]\s+/, '').trim()).filter(Boolean).join(' ') : n)
  process.exit(0)
}

let wrong = 0
for (const [file, f] of Object.entries(files)) {
  const text = readFileSync(file, 'utf8')
  const now = f.get(text)
  if (flag('--check')) {
    if (now !== version) {
      console.error(`✗ ${file} says ${now}, not ${version}`)
      wrong++
    }
  } else if (now !== version) {
    writeFileSync(file, f.set(text, version))
    console.log(`${file}: ${now} → ${version}`)
  }
}
if (flag('--check')) {
  if (!notes(version)) {
    console.error(`✗ CHANGELOG.md has no "## ${version}" section`)
    wrong++
  }
  process.exit(wrong ? 1 : 0)
}
// Cargo.lock records the workspace crates' versions too.
execFileSync('cargo', ['update', '--workspace', '--offline'], { stdio: 'inherit' })
if (!notes(version)) console.log(`Now add a "## ${version}" section to CHANGELOG.md.`)
