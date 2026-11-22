import { useEffect, useRef, useState } from 'react'
import { Key } from '../../src/components/Key/Key'
import { useDevices } from '../twoDevices'
import { formatSeconds, plural } from '../../src/engine/format'
import { Actions, Head, Pane } from '../../src/window/stage/parts'
import type { Report, ScanProgress } from '../report'
import { osName } from '../report'
import { scanLabels } from './labels'
import { Arrive, Dot } from './parts'
import s from './screens.module.css'
import type { RealScan, ScreenCtx } from './types'

type StepId = ScanProgress['step']
const order: StepId[] = ['reading', 'ecosystems', 'code', 'packages', 'report']

export interface StepView {
  id: StepId
  label: string
  state: 'done' | 'running' | 'pending'
  count?: string
  detail?: string
  current?: string | null
}

/** What each step found, once it's done (from the finished report) */
function doneCount(id: StepId, r: Report | null, files: number, parsed: number): string | undefined {
  switch (id) {
    case 'reading':
      return plural(files, 'file')
    case 'ecosystems':
      return r ? r.ecosystems.map((e) => e.name).join(', ') || 'None' : undefined
    case 'code':
      return `${plural(parsed, 'file')} checked`
    case 'packages': {
      const n = r?.packages.reduce((a, p) => a + p.declared, 0) ?? 0
      return r ? plural(n, 'package') : undefined
    }
    case 'report':
      return undefined
  }
}

/** A demo scan frozen at one step, or moving through all of them */
function demoSteps(r: Report, at: number, fraction: number, files: string[], target: string): StepView[] {
  const l = scanLabels(target)
  return order.map((id, i) => {
    const state = i < at ? 'done' : i === at ? 'running' : 'pending'
    const v: StepView = { id, label: l[id], state }
    if (state === 'done') v.count = doneCount(id, r, r.stats.files, r.stats.parsedFiles)
    if (state === 'running') {
      if (id === 'reading') v.count = plural(Math.max(1, Math.round(r.stats.files * fraction)), 'file')
      if (id === 'code') {
        const n = Math.max(1, Math.round(r.stats.parsedFiles * fraction))
        v.count = `${n} of ${r.stats.parsedFiles}`
        v.current = files[Math.min(files.length - 1, Math.floor(fraction * files.length))]
      }
      if (id === 'packages') {
        const n = r.packages.reduce((a, p) => a + p.declared, 0)
        v.count = plural(n, 'package')
        v.current = r.packages[0]?.lockfile ?? r.packages[0]?.source ?? null
      }
    }
    return v
  })
}

/** From the real scanner's --progress lines */
function realSteps(real: RealScan): StepView[] {
  const l = scanLabels(osName[real.target])
  const lastIndex = real.last ? order.indexOf(real.last.step) : -1
  const finished = real.status === 'done'
  return order.map((id, i) => {
    const p = real.steps[id]
    const state = finished || i < lastIndex ? 'done' : i === lastIndex ? 'running' : 'pending'
    const v: StepView = { id, label: l[id], state }
    const files = real.steps.reading?.done ?? 0
    const parsed = real.steps.code?.total ?? 0
    if (state === 'done') v.count = doneCount(id, real.report, files, parsed)
    if (state === 'running' && p) {
      if (id === 'reading') v.count = plural(p.done, 'file')
      if (id === 'code' && p.total) v.count = `${p.done} of ${p.total}`
      v.current = p.current
    }
    return v
  })
}

export function Steps({ steps }: { steps: StepView[] }) {
  return (
    <ol className={s.steps} aria-live="polite">
      {steps.map((step, i) => (
        <Arrive key={step.id} i={i}>
          <div className={s.step} data-state={step.state}>
            <Dot tone={step.state === 'done' ? 'proved' : step.state} />
            <span className={s.stepLabel}>{step.label}</span>
            <span className={s.stepCount}>{step.count}</span>
            {step.current && step.state === 'running' && <span className={`${s.stepDetail} ${s.current}`}>{step.current}</span>}
          </div>
        </Arrive>
      ))}
    </ol>
  )
}

/** Ticks while the demo scan runs, then hands over to the report */
function useDemoClock(active: boolean, done: () => void) {
  const [t, setT] = useState(0)
  const onDone = useRef(done)
  useEffect(() => {
    onDone.current = done
  })
  useEffect(() => {
    if (!active) return
    const start = performance.now()
    let raf = 0
    const tick = () => {
      const elapsed = performance.now() - start
      setT(elapsed)
      if (elapsed < 3600) raf = requestAnimationFrame(tick)
      else onDone.current()
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [active])
  return t
}

/** The time, every 100 ms while a real scan runs */
function useNow(active: boolean) {
  const [now, setNow] = useState(0)
  useEffect(() => {
    if (!active) return
    const id = window.setInterval(() => setNow(performance.now()), 100)
    return () => window.clearInterval(id)
  }, [active])
  return now
}

/** 2 · Scanning: the live step list, counts, the file being read, elapsed time */
export function ScanningView({ ctx }: { ctx: ScreenCtx }) {
  const devices = useDevices()
  const me = devices[ctx.side]
  const real = ctx.state === 'real'
  const live = ctx.state === 'live'
  const r = ctx.scenario.report
  const target = real ? osName[ctx.real.target] : osName[r.target]
  const name = real ? (ctx.real.path.split(/[\\/]/).filter(Boolean).pop() ?? 'this folder') : r.project.name
  const t = useDemoClock(live, () => ctx.go('report', 'summary'))
  const now = useNow(real && ctx.real.status === 'running')

  let steps: StepView[]
  let elapsed: number
  if (real) {
    steps = realSteps(ctx.real)
    elapsed = ((ctx.real.finishedAt ?? now) - ctx.real.startedAt) / 1000
  } else if (live) {
    // Reading 0–0.9 s, ecosystems –1.2, code –2.6, packages –3.1, report –3.6
    const marks = [900, 1200, 2600, 3100, 3600]
    const at = Math.min(4, marks.findIndex((m) => t < m) === -1 ? 4 : marks.findIndex((m) => t < m))
    const from = at === 0 ? 0 : marks[at - 1]
    const fraction = Math.min(1, (t - from) / (marks[at] - from))
    steps = demoSteps(r, at, fraction, ctx.scenario.files, target)
    elapsed = t / 1000
  } else {
    const at = { reading: 0, code: 2, packages: 3 }[ctx.state as 'reading' | 'code' | 'packages'] ?? 2
    steps = demoSteps(r, at, at === 0 ? 0.62 : 0.56, ctx.scenario.files, target)
    elapsed = [0.4, 0, 1.1, 1.6][at]
  }

  const failed = real && ctx.real.status === 'failed'
  return (
    <Pane>
      <Head
        title={failed ? 'That folder can’t be scanned' : `Checking ${name} for ${target}`}
        sub={
          failed
            ? ctx.real.error
            : `Reading it here on ${me.self}. Nothing is run or changed, and nothing leaves ${me.self}.`
        }
      />
      {!failed && <Steps steps={steps} />}
      <Actions leading={!failed && <span className={s.elapsed}>{formatSeconds(Math.max(0, elapsed))}</span>}>
        <Key onClick={() => ctx.go('entry', 'drop')}>{failed ? 'Choose another folder' : 'Stop'}</Key>
      </Actions>
    </Pane>
  )
}
