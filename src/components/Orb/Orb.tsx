/* Thinking orb (the thinking-orbs package, MIT): shown only while files
   move, beside the percentage. Monochrome dots on a canvas that follow the
   page theme (`data-theme`); still under reduced motion (OS or dev panel)
   and whenever there's nothing to show moving. */
import { useReducedMotionConfig } from 'motion/react'
import { ThinkingOrb } from 'thinking-orbs'
import type { OrbState } from 'thinking-orbs'
import s from './Orb.module.css'

export type { OrbState }

export function Orb({ state, size = 32, paused = false }: { state: OrbState; size?: 20 | 32 | 64; paused?: boolean }) {
  const reduced = useReducedMotionConfig()
  // Decorative: the percentage beside it says what's happening
  return <ThinkingOrb state={state} size={size} paused={paused || !!reduced} className={s.orb} aria-hidden />
}
