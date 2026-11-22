import { useState } from 'react'
import { ArrowCounterClockwiseIcon, CaretDownIcon, CheckIcon, FolderOpenIcon } from '@phosphor-icons/react'
import { Chip } from '../../src/components/Chip/Chip'
import { Key } from '../../src/components/Key/Key'
import { plural } from '../../src/engine/format'
import { Actions, Head, Pane } from '../../src/window/stage/parts'
import { download } from '../handoff'
import type { Change, Result, ResultState } from '../scenarios'
import { NothingToConvert } from './Consent'
import { resultName } from './labels'
import { Arrive, Diff, Dot } from './parts'
import { useSay } from './say'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

/** What needs you leads, as the headline does; what was proved closes */
const order: ResultState[] = ['needsYou', 'failed', 'couldntCheck', 'proved']

export function Results({ results }: { results: Result[] }) {
  const say = useSay()
  const sorted = order.flatMap((st) => results.filter((r) => r.state === st))
  return (
    <ul className={s.results} aria-label="Results">
      {sorted.map((r, i) => (
        <Arrive key={r.title} i={i}>
          <div className={s.result}>
            <Dot tone={r.state} />
            <span className={s.resultTitle}>{say(r.title)}</span>
            <span className={s.resultState}>{resultName[r.state]}</span>
            <span className={s.resultText}>{say(r.detail)}</span>
          </div>
        </Arrive>
      ))}
    </ul>
  )
}

/** Patch review: every change, its diff, and its own Undo */
function Patches({ changes, initiallyUndone }: { changes: Change[]; initiallyUndone: string[] }) {
  const [undone, setUndone] = useState<string[]>(initiallyUndone)
  const [open, setOpen] = useState<string[]>(changes.slice(0, 2).map((c) => c.id))
  const toggle = (list: string[], id: string) => (list.includes(id) ? list.filter((x) => x !== id) : [...list, id])
  return (
    <ol className={s.changes} aria-label="Changes">
      {changes.map((ch) => {
        const isUndone = undone.includes(ch.id)
        const isOpen = open.includes(ch.id)
        return (
          <li key={ch.id} className={s.change} data-undone={isUndone ? '' : undefined}>
            <div className={s.changeHead}>
              <button
                type="button"
                className={s.changeToggle}
                aria-expanded={isOpen}
                onClick={() => setOpen((o) => toggle(o, ch.id))}
              >
                <span className={s.changeText}>
                  <span className={s.changeFile}>{ch.file}</span>
                  <span className={s.changeSummary}>{isUndone ? `Undone. ${ch.summary}` : ch.summary}</span>
                </span>
                <CaretDownIcon className={s.caret} data-open={isOpen ? '' : undefined} weight="bold" />
              </button>
              {ch.by === 'ai' && <Chip>By the AI</Chip>}
              <Key
                size="sm"
                icon={isUndone ? undefined : <ArrowCounterClockwiseIcon />}
                onClick={() => setUndone((u) => toggle(u, ch.id))}
              >
                {isUndone ? 'Redo' : 'Undo'}
              </Key>
            </div>
            {isOpen && <Diff lines={ch.lines} />}
          </li>
        )
      })}
    </ol>
  )
}

/** 6 · The answer after converting: four honest states, every change reviewable */
export function FinalView({ ctx, override }: { ctx: ScreenCtx; override?: { headline: string; results: Result[] } }) {
  const c = ctx.scenario.convert
  const [exported, setExported] = useState(ctx.state === 'exported')
  const say = useSay()
  if (!c) return <NothingToConvert ctx={ctx} />
  const f = { ...c.final, ...override }
  const reviewing = ctx.state === 'patches' || ctx.state === 'undone'
  const exportPlan = () => {
    download(`${ctx.scenario.report.project.name} hand-off plan.md`, f.plan)
    setExported(true)
  }
  return (
    <Pane>
      <Head title={f.headline} sub={say(f.sub)} />
      {reviewing ? (
        <>
          <p className={s.subhead}>{plural(c.changes.length, 'change')}. Undo any of them; the checks run again.</p>
          <Patches changes={c.changes} initiallyUndone={ctx.state === 'undone' ? [c.changes[1].id] : []} />
        </>
      ) : (
        <>
          <Results results={f.results} />
          <Key size="sm" className={s.inlineKey} onClick={() => ctx.go('final', 'patches')}>
            Review the {plural(c.changes.length, 'change')}
          </Key>
        </>
      )}
      <Actions>
        {reviewing ? (
          <Key variant="ink" onClick={() => ctx.go('final', 'summary')}>
            Done reviewing
          </Key>
        ) : (
          <Key variant="ink" icon={<FolderOpenIcon />}>
            {ctx.side === 'mac' ? 'Show in Finder' : 'Open in File Explorer'}
          </Key>
        )}
        <Key icon={exported ? <CheckIcon /> : undefined} onClick={exportPlan}>
          {exported ? 'Hand-off plan saved' : 'Export hand-off plan'}
        </Key>
      </Actions>
    </Pane>
  )
}
