import { APP_NAME } from '../../src/platform'
import { useEffect, useState } from 'react'
import { Chip } from '../../src/components/Chip/Chip'
import { Key } from '../../src/components/Key/Key'
import { useReducedMotionPref } from '../../src/engine/context'
import { plural } from '../../src/engine/format'
import { Actions, Head, Pane } from '../../src/window/stage/parts'
import { osName } from '../report'
import type { Change, Check, Conversion, StepId } from '../scenarios'
import { usage } from '../scenarios'
import { NothingToConvert } from './Consent'
import { Arrive, Diff, Dot, Meter } from './parts'
import { useSay } from './say'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

const pipeline: { id: StepId; label: string }[] = [
  { id: 'snapshot', label: 'Snapshot' },
  { id: 'pack', label: 'Pack' },
  { id: 'plan', label: 'Plan' },
  { id: 'apply', label: 'Apply' },
  { id: 'verify', label: 'Verify' },
  { id: 'report', label: 'Report' },
  { id: 'repair', label: 'Repair agent' },
]

/** Which step runs in each demo state; repair runs while Verify waits on it */
const runningIn: Record<string, StepId> = { plan: 'plan', apply: 'apply', verify: 'verify', repair: 'repair' }

/** The seven steps in one row; what each did is in the Details log */
function Pipeline({ c, running }: { c: Conversion; running: StepId }) {
  const at = pipeline.findIndex((p) => p.id === running)
  const repairing = running === 'repair'
  return (
    <ol className={s.stepper} aria-label="Conversion steps">
      {pipeline.map((p, i) => {
        let state: 'done' | 'running' | 'pending'
        if (p.id === 'repair') state = repairing ? 'running' : 'pending'
        else if (repairing) state = p.id === 'verify' ? 'running' : i < 4 ? 'done' : 'pending'
        else state = i < at ? 'done' : i === at ? 'running' : 'pending'
        const word = state === 'done' ? 'done' : state === 'running' ? 'now' : p.id === 'repair' ? 'only if a check fails' : 'next'
        return (
          <li key={p.id} data-state={state} data-optional={p.id === 'repair' ? '' : undefined} title={state === 'done' ? c.done[p.id] : undefined}>
            <Dot tone={state === 'done' ? 'proved' : state} size="sm" />
            <span className={s.stepperLabel}>
              {p.id === 'repair' ? 'Repair' : p.label}
              <span className="sr-only">: {word}</span>
            </span>
          </li>
        )
      })}
    </ol>
  )
}

/** Changes appear as they're written; the newest reveals line by line */
function Changes({ changes, live }: { changes: Change[]; live: boolean }) {
  const reduced = useReducedMotionPref()
  const last = changes[changes.length - 1]
  const [upTo, setUpTo] = useState(live && !reduced ? 0 : last.lines.length)
  useEffect(() => {
    if (!live || reduced || upTo >= last.lines.length) return
    const id = window.setTimeout(() => setUpTo((n) => n + 1), 260)
    return () => window.clearTimeout(id)
  }, [live, reduced, upTo, last.lines.length])
  return (
    <ol className={s.changes} aria-label="Changes so far">
      {changes.map((ch, i) => (
        <Arrive key={ch.id} i={i}>
          <div className={s.change}>
            <div className={s.changeHead}>
              <span className={s.changeFile}>{ch.file}</span>
              <span className={s.changeSummary}>{ch.summary}</span>
              {ch.by === 'ai' && <Chip>By the AI</Chip>}
              {ch.added && <Chip>New file</Chip>}
            </div>
            <Diff lines={ch.lines} upTo={ch === last ? upTo : undefined} />
          </div>
        </Arrive>
      ))}
    </ol>
  )
}

function Checks({ checks, running }: { checks: Check[]; running: number }) {
  const say = useSay()
  return (
    <ul className={s.results} aria-label="Checks on this computer">
      {checks.map((ch, i) => {
        const state = i < running ? ch.state : i === running ? 'running' : 'pending'
        return (
          <li key={ch.label} className={s.result}>
            <Dot tone={state === 'proved' || state === 'failed' ? state : state} />
            <span className={s.resultTitle}>{ch.label}</span>
            {ch.detail && i <= running && <span className={s.resultText}>{state === 'running' ? 'Running…' : say(ch.detail)}</span>}
          </li>
        )
      })}
    </ul>
  )
}

export function AiPanel({ c }: { c: Conversion }) {
  const r = c.repair
  const say = useSay()
  return (
    <section className={s.ai} aria-label="AI repair">
      <div className={s.aiHead}>
        <Dot tone="running" />
        <span className={s.aiTitle}>AI repair</span>
        <span className={s.aiAttempt}>
          Attempt {r.attempt} of {r.of}
        </span>
      </div>
      <p className={s.noteText}>{r.problem}</p>
      <p className={s.aiTrying}>Trying: {r.trying}</p>
      {r.earlier.length > 0 && (
        <ul className={s.earlier}>
          {r.earlier.map((e) => (
            <li key={e.attempt}>
              <Dot tone="failed" size="sm" />
              <span>
                Attempt {e.attempt}: {e.tried}
              </span>
              <span>{say(e.result)}</span>
            </li>
          ))}
        </ul>
      )}
      <ul className={s.files} aria-label="What the AI is doing">
        {r.log.map((l) => (
          <li key={l}>
            <span>{l}</span>
          </li>
        ))}
      </ul>
      <Meter spent={usage.spent + r.spentHere} cap={usage.cap} label={`This conversion $${r.spentHere.toFixed(2)} · this month`} />
    </section>
  )
}

/** While planning: each fix, where it goes, and who writes it */
function Planned({ c }: { c: Conversion }) {
  const fixes = c.changes.filter((ch) => ch.by === 'pattern')
  return (
    <ul className={s.results} aria-label="Planned fixes">
      {fixes.map((ch, i) => (
        <Arrive key={ch.id} i={i}>
          <div className={s.result}>
            <Dot tone="willNeedAFix" />
            <span className={s.resultTitle}>{ch.summary}</span>
            <span className={s.resultState}>Known fix</span>
            <span className={`${s.resultText} ${s.mono}`}>{ch.file}</span>
          </div>
        </Arrive>
      ))}
      {c.forTheAi && (
        <Arrive i={fixes.length}>
          <div className={s.result}>
            <Dot tone="needsYou" />
            <span className={s.resultTitle}>{c.forTheAi}</span>
            <span className={s.resultState}>For the AI</span>
            <span className={s.resultText}>No known fix covers this. {APP_NAME} asks before sending any file.</span>
          </div>
        </Arrive>
      )}
    </ul>
  )
}

/** 5 · Conversion on the receiving computer: every step in plain words */
export function ConvertingView({ ctx }: { ctx: ScreenCtx }) {
  const c = ctx.scenario.convert
  const say = useSay()
  if (!c) return <NothingToConvert ctx={ctx} />
  const r = ctx.scenario.report
  const running = runningIn[ctx.state] ?? 'apply'
  const status = running === 'repair' ? 'A check failed. The AI is fixing it, then the checks run again.' : c.doing[running]
  const applied = running === 'apply' ? c.changes.filter((ch) => ch.by === 'pattern').slice(0, 3) : c.changes.filter((ch) => ch.by === 'pattern')
  return (
    <Pane>
      <Head title={`Converting ${r.project.name} for ${osName[r.target]}`} sub={say(status ?? '')} />
      <Pipeline c={c} running={running} />
      {running === 'plan' && (
        <>
          <p className={s.subhead}>Planned fixes</p>
          <Planned c={c} />
        </>
      )}
      {running === 'verify' && (
        <>
          <p className={s.subhead}>Checks on {say('{here}')}</p>
          <Checks checks={c.checks} running={2} />
        </>
      )}
      {(running === 'apply' || running === 'verify') && (
        <>
          <p className={s.subhead}>{running === 'apply' ? 'Changes so far' : `${plural(applied.length, 'change')} made`}</p>
          <Changes changes={applied} live={running === 'apply'} />
        </>
      )}
      {running === 'repair' && (
        <>
          <AiPanel c={c} />
          <Changes changes={c.changes.filter((ch) => ch.by === 'ai')} live />
        </>
      )}
      <Actions>
        <Key>Stop</Key>
      </Actions>
    </Pane>
  )
}
