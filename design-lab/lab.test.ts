/* The lab's promises: every report it renders is real scanner output that
   matches the published schema, every screen in the mission is there, demo
   data never carries a secret value, and the scan bridge only takes real
   folders from the lab's own page. */
import { readdirSync, readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join, resolve } from 'node:path'
import { describe, expect, test } from 'vitest'
import { handoffMarkdown } from './handoff'
import type { Report } from './report'
import { scenarios } from './scenarios'
import { screens } from './screens'
import { folderFrom, sameOrigin } from './server/scanBridge'

const repo = resolve(import.meta.dirname, '..')
const schema = JSON.parse(readFileSync(join(repo, 'docs/scan-report.schema.json'), 'utf8'))

type Node = Record<string, unknown>

/** The JSON Schema keywords schemars writes, checked strictly: unknown
    properties are errors too, so the lab and the scanner can't drift. */
function validate(value: unknown, node: Node, path: string, errors: string[]) {
  if (typeof node.$ref === 'string') {
    const name = node.$ref.replace('#/$defs/', '')
    return validate(value, (schema.$defs as Record<string, Node>)[name], path, errors)
  }
  const types = node.type === undefined ? [] : ([] as string[]).concat(node.type as string)
  const typeOf = (v: unknown) =>
    v === null ? 'null' : Array.isArray(v) ? 'array' : Number.isInteger(v) ? 'integer' : typeof v
  const actual = typeOf(value)
  const fits = (t: string) => t === actual || (t === 'number' && actual === 'integer')
  if (types.length && !types.some(fits)) errors.push(`${path}: ${actual}, expected ${types.join('|')}`)
  if (Array.isArray(node.enum) && !node.enum.includes(value)) errors.push(`${path}: ${String(value)} not in enum`)
  if (typeof node.minimum === 'number' && typeof value === 'number' && value < node.minimum) errors.push(`${path}: below minimum`)
  if (actual === 'object' && node.properties) {
    const props = node.properties as Record<string, Node>
    const obj = value as Record<string, unknown>
    for (const key of (node.required as string[]) ?? []) if (!(key in obj)) errors.push(`${path}.${key}: missing`)
    for (const [key, v] of Object.entries(obj)) {
      if (props[key]) validate(v, props[key], `${path}.${key}`, errors)
      else errors.push(`${path}.${key}: not in the schema`)
    }
  }
  if (actual === 'array' && node.items) (value as unknown[]).forEach((v, i) => validate(v, node.items as Node, `${path}[${i}]`, errors))
}

const reports: [string, unknown][] = [
  ...readdirSync(join(repo, 'design-lab/data')).map((f): [string, unknown] => [
    `design-lab/data/${f}`,
    JSON.parse(readFileSync(join(repo, 'design-lab/data', f), 'utf8')),
  ]),
  ...readdirSync(join(repo, 'crates/portal-scan/tests/golden')).map((f): [string, unknown] => [
    `golden/${f}`,
    JSON.parse(readFileSync(join(repo, 'crates/portal-scan/tests/golden', f), 'utf8')),
  ]),
]

describe('reports', () => {
  test.each(reports)('%s matches the scanner’s schema', (_, report) => {
    const errors: string[] = []
    validate(report, schema, 'report', errors)
    expect(errors).toEqual([])
  })

  test('the validator catches a report that drifted', () => {
    const bad = structuredClone(reports[0][1]) as Record<string, unknown>
    ;(bad as { counts: { blocker: unknown } }).counts.blocker = 'two'
    bad.extra = true
    const errors: string[] = []
    validate(bad, schema, 'report', errors)
    expect(errors).toEqual(['report.counts.blocker: string, expected integer', 'report.extra: not in the schema'])
  })

  test('every demo scenario is a real scan of a demo project', () => {
    const projects = readdirSync(join(repo, 'design-lab/projects'))
    for (const s of scenarios) expect(projects).toContain(s.report.project.name)
  })

  test('demo projects survive a Windows checkout and keep their line endings', () => {
    const reserved = /^(con|prn|aux|nul|com\d|lpt\d)$/i
    const bad: string[] = []
    const walk = (dir: string) => {
      for (const e of readdirSync(dir, { withFileTypes: true })) {
        const rel = join(dir, e.name).slice(repo.length + 1)
        if (/[<>:"|?*\\]/.test(e.name) || reserved.test(e.name.split('.')[0]) || rel.length > 180) bad.push(rel)
        if (e.isDirectory()) walk(join(dir, e.name))
      }
    }
    walk(join(repo, 'design-lab/projects'))
    expect(bad).toEqual([])
    expect(readFileSync(join(repo, '.gitattributes'), 'utf8')).toContain('design-lab/projects/** -text')
  })

  test('demo reports and hand-off notes never carry a secret value', () => {
    const env = readFileSync(join(repo, 'design-lab/projects/local-chat/dot_env'), 'utf8')
    const values = env.split('\n').filter(Boolean).map((l) => l.split('=')[1])
    const chat = scenarios.find((s) => s.id === 'local-chat')!.report
    const text = JSON.stringify(chat) + handoffMarkdown(chat as Report)
    for (const v of values) expect(text).not.toContain(v)
    expect(chat.secrets[0].keys).toEqual(['OPENAI_API_KEY', 'HF_TOKEN', 'CHAT_MODEL'])
  })
})

describe('screens', () => {
  test('every screen in the mission is in the lab, in order', () => {
    expect(screens.map((s) => s.id)).toEqual([
      'entry',
      'scanning',
      'report',
      'consent',
      'converting',
      'final',
      'rebuild',
      'accounts',
      'edge',
    ])
  })

  test('the states the mission names are all there', () => {
    const states = (id: string) => screens.find((s) => s.id === id)!.states.map((s) => s.id)
    expect(states('edge')).toEqual(['empty', 'huge', 'unreadable', 'offline', 'budget', 'declined'])
    expect(states('accounts')).toEqual(['signin', 'account', 'plans', 'gift', 'ownkey', 'cap'])
    expect(states('consent')).toEqual(['command', 'ai'])
    expect(states('scanning')).toContain('real')
    expect(states('report')).toContain('real')
  })

  test('both computers start a scenario, so every screen appears in both window styles', () => {
    expect(new Set(scenarios.map((s) => s.from))).toEqual(new Set(['mac', 'win']))
    expect(scenarios.filter((s) => s.rebuild).map((s) => s.from).sort()).toEqual(['mac', 'win'])
    expect(scenarios.filter((s) => s.convert).map((s) => s.from).sort()).toContain('win')
  })
})

describe('scan bridge', () => {
  test('takes absolute paths and ~ paths to real folders only', () => {
    expect(folderFrom(repo)).toBe(repo)
    expect(folderFrom('~')).toBe(homedir())
    expect(folderFrom('design-lab')).toBeNull()
    expect(folderFrom(join(repo, 'package.json'))).toBeNull()
    expect(folderFrom('/no/such/folder')).toBeNull()
    expect(folderFrom(null)).toBeNull()
    expect(folderFrom('\\\\server\\share')).toBeNull()
    expect(folderFrom('//server/share')).toBeNull()
  })

  test('refuses requests from other sites', () => {
    expect(sameOrigin({ headers: { 'sec-fetch-site': 'same-origin' } })).toBe(true)
    expect(sameOrigin({ headers: {} })).toBe(true)
    expect(sameOrigin({ headers: { 'sec-fetch-site': 'cross-site' } })).toBe(false)
    expect(sameOrigin({ headers: { 'sec-fetch-site': 'same-site' } })).toBe(false)
    // Older browsers send no Sec-Fetch-Site, but do send Origin
    expect(sameOrigin({ headers: { origin: 'http://localhost:5190', host: 'localhost:5190' } })).toBe(true)
    expect(sameOrigin({ headers: { origin: 'https://evil.example', host: 'localhost:5190' } })).toBe(false)
    expect(sameOrigin({ headers: { origin: 'http://localhost:5180', host: 'localhost:5190' } })).toBe(false)
  })
})
