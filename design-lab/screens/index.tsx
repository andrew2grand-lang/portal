import { osName } from '../report'
import { AccountsView } from './Accounts'
import { ConsentView } from './Consent'
import { ConvertingView } from './Converting'
import { EdgeView } from './Edge'
import { EntryView } from './Entry'
import { FinalView } from './Final'
import { RebuildView } from './Rebuild'
import { ReportScreen } from './Report'
import { ScanningView } from './Scanning'
import type { ScreenCtx, ScreenDef } from './types'

type Line = ReturnType<NonNullable<ScreenDef['log']>>[number]
const line = (time: string, tag: string, text: string, tone?: Line['tone']): Line => ({ time, tag, text, tone, count: 1 })

/** The Details log for scan screens: what the scanner did, in its own words */
function scanLog(ctx: ScreenCtx): Line[] {
  const r = ctx.state === 'real' && ctx.real.report ? ctx.real.report : ctx.scenario.report
  return [
    line('09:41:00', 'scan', `${r.project.path} → ${osName[r.target]}`),
    line('09:41:00', 'read', `${r.stats.files} files, ${r.stats.parsedFiles} parsed, ${r.stats.skippedFiles} skipped`),
    line('09:41:01', 'found', r.ecosystems.map((e) => e.name).join(', ') || 'nothing to run'),
    line('09:41:01', 'safe', 'nothing was run; secret values were not read'),
    line('09:41:01', 'net', r.network.online ? `online: ${r.network.hosts.join(', ')}` : 'offline: nothing looked up', 'dim'),
  ]
}

/** {here}/{there} in demo text, from the window's side (logs aren't in a window context) */
function sayFor(ctx: ScreenCtx, text: string) {
  const here = ctx.side === 'mac' ? 'this Mac' : 'this PC'
  const there = ctx.side === 'mac' ? 'Desk PC' : 'MacBook Pro'
  return text.replaceAll('{here}', here).replaceAll('{there}', there)
}

function convertLog(ctx: ScreenCtx): Line[] {
  const r = ctx.scenario.report
  const c = ctx.scenario.convert
  if (!c) return scanLog(ctx)
  const host = ctx.scenario.from === 'mac' ? 'macbook-pro.local' : 'desk-pc.local'
  // While converting, the log stops at the step that's running
  const upTo =
    ctx.screen === 'converting'
      ? { plan: 3, apply: 4, verify: 5, repair: 7 }[ctx.state]
      : ctx.screen === 'consent'
        ? { command: 5, ai: 6 }[ctx.state]
        : undefined
  return [
    line('09:44:12', 'recv', `${r.project.name} from ${host}`),
    line('09:44:12', 'snapshot', sayFor(ctx, c.done.snapshot ?? '')),
    line('09:44:13', 'pack', sayFor(ctx, c.done.pack ?? '')),
    line('09:44:13', 'plan', sayFor(ctx, c.done.plan ?? '')),
    line('09:44:14', 'apply', sayFor(ctx, c.done.apply ?? '')),
    line('09:44:31', 'verify', c.checks.map((x) => `${x.label.toLowerCase()}: ${x.state}`).join('; ')),
    ...(c.repair.trying ? [line('09:44:40', 'ai', `attempt ${c.repair.attempt} of ${c.repair.of}: ${c.repair.trying}`, 'warn')] : []),
  ].slice(0, upTo)
}

export const screens: ScreenDef[] = [
  {
    id: 'entry',
    title: 'Project tab',
    on: 'sender',
    states: [
      { id: 'drop', label: 'Drop a project' },
      { id: 'over', label: 'Dragging a folder over' },
    ],
    render: (ctx) => <EntryView ctx={ctx} />,
  },
  {
    id: 'scanning',
    title: 'Scanning',
    on: 'sender',
    states: [
      { id: 'live', label: 'Live demo scan' },
      { id: 'reading', label: 'Reading files' },
      { id: 'code', label: 'Checking code' },
      { id: 'packages', label: 'Checking packages' },
      { id: 'real', label: 'Real scan', needsReal: true },
    ],
    log: scanLog,
    render: (ctx) => <ScanningView ctx={ctx} />,
  },
  {
    id: 'report',
    title: 'Blocker report',
    on: 'sender',
    states: [
      { id: 'summary', label: 'The answer' },
      { id: 'expanded', label: 'A finding opened' },
      { id: 'exported', label: 'Hand-off note exported' },
      { id: 'real', label: 'Real scan', needsReal: true },
    ],
    log: scanLog,
    render: (ctx) => <ReportScreen ctx={ctx} />,
  },
  {
    id: 'consent',
    title: 'Consent stops',
    on: 'receiver',
    states: [
      { id: 'command', label: 'Run a command?' },
      { id: 'ai', label: 'Send files to the AI?' },
    ],
    log: convertLog,
    glow: () => 'working',
    render: (ctx) => <ConsentView ctx={ctx} />,
  },
  {
    id: 'converting',
    title: 'Conversion',
    on: 'receiver',
    states: [
      { id: 'plan', label: 'Planning' },
      { id: 'apply', label: 'Applying changes' },
      { id: 'verify', label: 'Checking it runs' },
      { id: 'repair', label: 'AI repair, attempt 2 of 3' },
    ],
    log: convertLog,
    glow: (ctx) => (ctx.scenario.convert ? 'working' : 'off'),
    render: (ctx) => <ConvertingView ctx={ctx} />,
  },
  {
    id: 'final',
    title: 'Final report',
    on: 'receiver',
    states: [
      { id: 'summary', label: 'The answer' },
      { id: 'patches', label: 'Reviewing changes' },
      { id: 'undone', label: 'A change undone' },
      { id: 'exported', label: 'Hand-off plan exported' },
    ],
    log: convertLog,
    render: (ctx) => <FinalView ctx={ctx} />,
  },
  {
    id: 'rebuild',
    title: 'Rebuild verdict',
    on: 'sender',
    states: [
      { id: 'verdict', label: 'Verdict' },
      { id: 'plan', label: 'Migration plan' },
    ],
    log: scanLog,
    render: (ctx) => <RebuildView ctx={ctx} />,
  },
  {
    id: 'accounts',
    title: 'Accounts and plans',
    on: 'sender',
    states: [
      { id: 'signin', label: 'Sign in' },
      { id: 'account', label: 'Account' },
      { id: 'plans', label: 'Plans' },
      { id: 'gift', label: 'A $5 gift' },
      { id: 'ownkey', label: 'Own key' },
      { id: 'cap', label: 'Allowance used up' },
    ],
    render: (ctx) => <AccountsView ctx={ctx} />,
  },
  {
    id: 'edge',
    title: 'Edge states',
    on: 'sender',
    states: [
      { id: 'empty', label: 'Empty folder' },
      { id: 'huge', label: 'Huge folder' },
      { id: 'unreadable', label: 'Unreadable files' },
      { id: 'offline', label: 'Offline: couldn’t check' },
      { id: 'budget', label: 'AI budget used up' },
      { id: 'declined', label: 'Consent declined' },
    ],
    log: scanLog,
    render: (ctx) => <EdgeView ctx={ctx} />,
  },
]

/** Edge states that happen on the receiving computer */
export const receiverStates = new Set(['edge:budget', 'edge:declined'])

export function screenById(id: string | null): ScreenDef {
  return screens.find((s) => s.id === id) ?? screens[0]
}
