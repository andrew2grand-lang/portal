import { Key } from '../../src/components/Key/Key'
import { formatSeconds } from '../../src/engine/format'
import { Actions, Head, Pane } from '../../src/window/stage/parts'
import emptyFolder from '../data/Empty folder.json'
import type { Report } from '../report'
import { osName } from '../report'
import { CapReached } from './Accounts'
import { FinalView } from './Final'
import { ReportView } from './Report'
import { scanLabels } from './labels'
import { Steps } from './Scanning'
import type { StepView } from './Scanning'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

/** A huge folder: the scan says how big, and what it skips */
function Huge({ ctx }: { ctx: ScreenCtx }) {
  const target = osName[ctx.scenario.report.target]
  const l = scanLabels(target)
  const steps: StepView[] = [
    { id: 'reading', label: l.reading, state: 'done', count: '48,213 files' },
    { id: 'ecosystems', label: l.ecosystems, state: 'done', count: 'Python, Node, Shell' },
    { id: 'code', label: l.code, state: 'running', count: '1,904 of 6,377', current: 'services/billing/worker/tasks.py' },
    { id: 'packages', label: l.packages, state: 'pending' },
    { id: 'report', label: l.report, state: 'pending' },
  ]
  return (
    <Pane>
      <Head
        title={`Checking monorepo for ${target}`}
        sub="It’s a big one: 31.4 GB. Almost all of it is node_modules and .venv folders, which aren’t sent, so only the project’s own 6,377 code files are checked."
      />
      <Steps steps={steps} />
      <Actions leading={<span className={s.elapsed}>{formatSeconds(6.2)}</span>}>
        <Key>Stop</Key>
      </Actions>
    </Pane>
  )
}

/** Demo report variants built from the scenario's real report */
function withUnreadable(r: Report): Report {
  return {
    ...r,
    stats: { ...r.stats, unreadable: ['data/private/statement-2026-09.pdf', 'data/private/statement-2026-10.pdf', 'notes/locked.md'] },
    notes: [...r.notes, '3 files couldn’t be read; they weren’t checked.'],
  }
}

export function EdgeView({ ctx }: { ctx: ScreenCtx }) {
  const convert = ctx.scenario.convert
  switch (ctx.state) {
    case 'empty':
      return <ReportView ctx={ctx} report={emptyFolder as Report} />
    case 'huge':
      return <Huge ctx={ctx} />
    case 'unreadable':
      return <ReportView ctx={{ ...ctx, state: 'summary' }} report={withUnreadable(ctx.scenario.report)} />
    case 'offline':
      return <ReportView ctx={ctx} report={ctx.scenario.report} />
    case 'budget':
      return <CapReached ctx={ctx} during />
    case 'declined': {
      if (!convert) return <ReportView ctx={ctx} report={ctx.scenario.report} />
      const c = convert.consent
      const started = convert.final.results.map((r) =>
        r.title === 'The app starts' || r.title === 'The server starts' || r.title === 'Exports a month'
          ? { state: 'couldntCheck' as const, title: r.title, detail: `You chose not to run ${c.command}, so this wasn’t tried. Nothing is claimed.` }
          : r,
      )
      const needs = started.filter((r) => r.state === 'needsYou').length
      const target = osName[ctx.scenario.report.target]
      return (
        <FinalView
          ctx={{ ...ctx, state: 'summary' }}
          override={{
            headline: `Installs and passes its checks on ${target}, but it wasn’t started.${needs ? ` ${needs} ${needs === 1 ? 'thing needs' : 'things need'} you.` : ''}`,
            results: started,
          }}
        />
      )
    }
  }
  return (
    <Pane align="center">
      <Head title="Choose an edge state" />
    </Pane>
  )
}
