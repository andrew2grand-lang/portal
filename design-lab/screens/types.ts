import type { ReactNode } from 'react'
import type { LogLine, Side } from '../../src/engine/types'
import type { GlowMode } from '../../src/engine/views'
import type { Report, ScanProgress, TargetOs } from '../report'
import type { Scenario } from '../scenarios'

/** A scan of a real folder through the lab's bridge (portal-scan CLI) */
export interface RealScan {
  status: 'idle' | 'running' | 'done' | 'failed'
  path: string
  target: TargetOs
  online: boolean
  steps: Partial<Record<ScanProgress['step'], ScanProgress>>
  last: ScanProgress | null
  report: Report | null
  error: string | null
  startedAt: number
  finishedAt: number | null
}

export interface ScreenCtx {
  scenario: Scenario
  /** The screen being drawn (its id) */
  screen: string
  /** The window it's drawn in */
  side: Side
  state: string
  go: (screen: string, state?: string) => void
  real: RealScan
  startDemoScan: () => void
  /** Scan the real folder again with the online package check */
  checkOnline?: () => void
}

export interface ScreenDef {
  id: string
  title: string
  states: { id: string; label: string; needsReal?: boolean }[]
  /** Which computer's window shows it: the one sending, or the one receiving */
  on: 'sender' | 'receiver'
  glow?: (ctx: ScreenCtx) => GlowMode
  log?: (ctx: ScreenCtx) => Omit<LogLine, 'id'>[]
  render: (ctx: ScreenCtx) => ReactNode
}
