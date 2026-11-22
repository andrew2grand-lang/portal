import { describe, expect, it } from 'vitest'
import type { CorePhase } from './bridge'
import { BAR_AFTER_MS, MOMENT_MS, moment, startClock } from './moment'

/** Feeds the core's phases at given times and returns what the window shows */
function run(timeline: [number, CorePhase][], at: number) {
  let clock = startClock()
  let shown = moment(null, clock, 0)
  for (const [t, phase] of timeline) {
    if (t > at) break
    shown = moment({ id: 'x', phase }, shown.clock, t)
    clock = shown.clock
  }
  return moment({ id: 'x', phase: timeline.filter(([t]) => t <= at).at(-1)![1] }, clock, at)
}

describe('the arrival moment', () => {
  it('holds a near-instant transfer for the moment, without a progress bar', () => {
    const timeline: [number, CorePhase][] = [
      [0, 'offered'],
      [1000, 'running'],
      [1040, 'done'],
    ]
    expect(run(timeline, 1040)).toMatchObject({ phase: 'running', bar: false, until: 1000 + MOMENT_MS })
    expect(run(timeline, 1000 + MOMENT_MS - 1)).toMatchObject({ phase: 'running', bar: false })
    expect(run(timeline, 1000 + MOMENT_MS)).toMatchObject({ phase: 'done', until: null })
  })

  it('plays the moment even when the core went straight from offered to done', () => {
    const timeline: [number, CorePhase][] = [
      [0, 'offered'],
      [500, 'done'],
    ]
    expect(run(timeline, 500)).toMatchObject({ phase: 'running', bar: false, until: 500 + MOMENT_MS })
    expect(run(timeline, 500 + MOMENT_MS)).toMatchObject({ phase: 'done' })
  })

  it('shows the bar only once a transfer has run a little while', () => {
    const timeline: [number, CorePhase][] = [[0, 'running']]
    expect(run(timeline, BAR_AFTER_MS - 1)).toMatchObject({ phase: 'running', bar: false, until: BAR_AFTER_MS })
    expect(run(timeline, BAR_AFTER_MS)).toMatchObject({ phase: 'running', bar: true, until: null })
  })

  it('a quick transfer that showed its bar keeps it through the rest of the moment', () => {
    const timeline: [number, CorePhase][] = [
      [0, 'running'],
      [600, 'done'],
    ]
    expect(run(timeline, 600)).toMatchObject({ phase: 'running', bar: true, until: MOMENT_MS })
    expect(run(timeline, MOMENT_MS)).toMatchObject({ phase: 'done' })
  })

  it('a long transfer lands as soon as it’s done', () => {
    const timeline: [number, CorePhase][] = [
      [0, 'running'],
      [5000, 'done'],
    ]
    expect(run(timeline, 5000)).toMatchObject({ phase: 'done', until: null })
  })

  it('doesn’t replay the moment for a transfer that was already done when the window opened', () => {
    expect(run([[0, 'done']], 0)).toMatchObject({ phase: 'done', until: null })
  })

  it('leaves other phases alone', () => {
    for (const phase of ['offered', 'failed', 'declined', 'blocked'] as CorePhase[]) {
      expect(run([[0, phase]], 0)).toMatchObject({ phase, until: null })
    }
  })

  it('starts fresh for the next transfer', () => {
    let s = moment({ id: 'a', phase: 'running' }, startClock(), 0)
    s = moment({ id: 'a', phase: 'done' }, s.clock, 50)
    s = moment({ id: 'b', phase: 'offered' }, s.clock, 60)
    expect(s).toMatchObject({ phase: 'offered', until: null })
    s = moment({ id: 'b', phase: 'done' }, s.clock, 5000)
    expect(s).toMatchObject({ phase: 'running', until: 5000 + MOMENT_MS })
  })
})
