/* The arrival moment (Mission 02 §7.2). A transfer that finishes almost
   instantly still shows the light travelling and the landing for a short
   minimum, and never flashes a progress bar on the way. */
import type { CorePhase } from './bridge'

/** The light plus the landing last at least this long */
export const MOMENT_MS = 900
/** A transfer shorter than this never shows the progress bar */
export const BAR_AFTER_MS = 400

export interface MomentClock {
  id: string | null
  /** The phase this window first saw it in */
  first: CorePhase | null
  /** When it started moving (or, for an instant one, when it was done) */
  movingSince: number | null
  /** When the core said done */
  doneAt: number | null
}

export interface Shown {
  phase: CorePhase | null
  /** Show the progress bar */
  bar: boolean
  /** Look again at this time (ms) */
  until: number | null
  clock: MomentClock
}

export const startClock = (): MomentClock => ({ id: null, first: null, movingSince: null, doneAt: null })

/** What the window shows for the core's transfer at `now` */
export function moment(t: { id: string; phase: CorePhase } | null, prev: MomentClock, now: number): Shown {
  if (!t) return { phase: null, bar: false, until: null, clock: startClock() }
  const clock: MomentClock = prev.id === t.id ? { ...prev } : { id: t.id, first: t.phase, movingSince: null, doneAt: null }
  if (t.phase === 'running' && clock.movingSince === null) clock.movingSince = now
  if (t.phase === 'done' && clock.doneAt === null) {
    clock.doneAt = now
    // Offered → done with nothing in between: the moment starts now
    if (clock.movingSince === null && clock.first !== 'done') clock.movingSince = now
  }
  const since = clock.movingSince

  if (t.phase === 'running' && since !== null) {
    const bar = now - since >= BAR_AFTER_MS
    return { phase: 'running', bar, until: bar ? null : since + BAR_AFTER_MS, clock }
  }
  if (t.phase === 'done' && since !== null && now < since + MOMENT_MS) {
    // Keep the bar only if it had already appeared
    const bar = (clock.doneAt ?? now) - since >= BAR_AFTER_MS
    return { phase: 'running', bar, until: since + MOMENT_MS, clock }
  }
  return { phase: t.phase, bar: t.phase === 'running', until: null, clock }
}
