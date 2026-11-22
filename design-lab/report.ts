/* The scanner's report (portal-scan --json, schemaVersion 1), as TypeScript.
   Mirrors crates/portal-scan/src/report.rs; docs/scan-report.schema.json is
   the contract, and lab.test.ts checks every report the lab renders against it. */

export type State = 'blocker' | 'willNeedAFix' | 'needsYou' | 'couldntCheck' | 'info'
export type Tier = 'static' | 'scripts' | 'apps' | 'systems' | 'native'
export type AiLevel = 'none' | 'rare' | 'sometimes' | 'often' | 'planOnly'
export type TargetOs = 'windows' | 'macos'

export interface Finding {
  id: string
  rule: string
  title: string
  category: string
  state: State
  severity: 'high' | 'medium' | 'low'
  file: string | null
  line: number | null
  evidence: string | null
  why: string
  fixPattern: string
  autoFixable: boolean
  targets: TargetOs[]
}

export interface LeftOutItem {
  path: string
  kind: string
  bytes: number
  files: number
  rebuildable: boolean
}

export interface Report {
  schemaVersion: number
  tool: { name: string; version: string }
  scannedAt: string
  project: { name: string; path: string }
  source: 'macos' | 'windows' | 'linux' | 'other'
  target: TargetOs
  network: { online: boolean; hosts: string[] }
  durationMs: number
  stats: { files: number; bytes: number; parsedFiles: number; skippedFiles: number; unreadable: string[] }
  tier: { tier: Tier; reason: string }
  headline: string
  counts: {
    blocker: number
    willNeedAFix: number
    needsYou: number
    couldntCheck: number
    info: number
    occurrences: number
    omitted: number
  }
  aiUsage: { level: AiLevel; estimate: boolean; reason: string }
  ecosystems: { kind: string; name: string; evidence: string[]; files: number; support: 'deep' | 'summary' }[]
  runtimes: { kind: string; requested: string | null; requestedFrom: string | null; installedHere: string | null }[]
  packages: {
    kind: string
    source: string
    lockfile: string | null
    declared: number
    installed: number
    items: { name: string; version: string | null }[]
  }[]
  runCommands: { kind: string; name: string; command: string; file: string }[]
  leftOut: { bytes: number; files: number; items: LeftOutItem[] }
  secrets: { path: string; kind: string; keys: string[] }[]
  services: { kind: string; name: string; from: string }[]
  findings: Finding[]
  handoff: string | null
  notes: string[]
}

/** One line of `portal-scan --progress` */
export interface ScanProgress {
  step: 'reading' | 'ecosystems' | 'code' | 'packages' | 'report'
  done: number
  total: number
  current: string | null
}

export const stateOrder: State[] = ['blocker', 'willNeedAFix', 'needsYou', 'couldntCheck', 'info']

export const stateName: Record<State, { one: string; group: string }> = {
  blocker: { one: 'Blocker', group: 'Blockers' },
  willNeedAFix: { one: 'Will need a fix', group: 'Will need a fix' },
  needsYou: { one: 'Needs you', group: 'Needs you' },
  couldntCheck: { one: "Couldn't check", group: "Couldn't check" },
  info: { one: 'Info', group: 'Info' },
}

export const tierName: Record<Tier, string> = {
  static: 'Static',
  scripts: 'Scripts',
  apps: 'App',
  systems: 'System',
  native: 'Native app',
}

export const aiLevelName: Record<AiLevel, string> = {
  none: 'None',
  rare: 'Rare',
  sometimes: 'Sometimes',
  often: 'Often',
  planOnly: 'Plan only',
}

export const osName: Record<TargetOs, string> = { windows: 'Windows', macos: 'macOS' }

/** One finding per rule and state, with every place it was seen */
export interface FindingGroup {
  key: string
  first: Finding
  places: Finding[]
}

export function groupFindings(findings: Finding[], state: State): FindingGroup[] {
  const groups = new Map<string, FindingGroup>()
  for (const f of findings) {
    if (f.state !== state) continue
    const key = `${f.rule}:${f.title}`
    const g = groups.get(key)
    if (g) g.places.push(f)
    else groups.set(key, { key, first: f, places: [f] })
  }
  return [...groups.values()]
}

/** "app/main.py:27", "run.sh", or nothing for project-wide findings */
export function place(f: Finding): string | null {
  if (!f.file) return null
  return f.line ? `${f.file}:${f.line}` : f.file
}
