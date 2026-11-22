// Checks that every command the UI invokes (src/core/bridge.ts) is registered
// in src-tauri/src/lib.rs and takes the same arguments (camelCase in JS,
// snake_case in Rust; Tauri converts between them).
import { readFileSync } from 'node:fs'

const ts = readFileSync('src/core/bridge.ts', 'utf8')
const lib = readFileSync('src-tauri/src/lib.rs', 'utf8')
const rs = readFileSync('src-tauri/src/bridge.rs', 'utf8')

const registered = new Set([...lib.matchAll(/bridge::(portal_\w+)/g)].map((m) => m[1]))
const rustArgs = new Map(
  [...rs.matchAll(/pub (?:async )?fn (portal_\w+)\(([^)]*)\)/g)].map((m) => [
    m[1],
    m[2]
      .replace(/\w+:\s*State<[^>]*>/g, '')
      .replace(/\w+:\s*AppHandle/g, '')
      .split(',')
      .map((a) => a.trim().split(':')[0].trim())
      .filter((a) => a && a !== 'bridge' && a !== 'app'),
  ]),
)
const snake = (s) => s.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)

let problems = 0
for (const m of ts.matchAll(/invoke<[^>]*>\('(portal_\w+)'(?:,\s*\{([^}]*)\})?\)/g)) {
  const [, cmd, rawArgs] = m
  const args = (rawArgs ?? '').split(',').map((a) => a.trim().split(':')[0].trim()).filter(Boolean)
  if (!registered.has(cmd)) {
    console.error(`✗ ${cmd} is invoked but not registered in lib.rs`)
    problems++
    continue
  }
  const want = (rustArgs.get(cmd) ?? []).slice().sort().join(',')
  const got = args.map(snake).sort().join(',')
  if (want !== got) {
    console.error(`✗ ${cmd}: UI sends {${got}}, Rust expects {${want}}`)
    problems++
  } else {
    console.log(`✓ ${cmd}(${args.join(', ')})`)
  }
}
process.exit(problems ? 1 : 0)
