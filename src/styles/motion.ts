import type { Transition } from 'motion/react'

/* Motion tokens, the JS half of tokens.css.
   Default is critically damped (no overshoot); bounce is reserved for keys
   springing back, because a key press carries momentum. */
export const spring = {
  ui: { type: 'spring', bounce: 0, duration: 0.42 },
  layout: { type: 'spring', bounce: 0.08, duration: 0.45 },
  press: { type: 'spring', bounce: 0, duration: 0.1 },
  release: { type: 'spring', bounce: 0.45, duration: 0.32 },
} satisfies Record<string, Transition>

export const easeOut = [0.16, 1, 0.3, 1] as const

/* State-to-state "materialize": surfaces arrive with blur + rise, not a plain fade */
export const materialize = {
  initial: { opacity: 0, y: 10, filter: 'blur(6px)' },
  animate: { opacity: 1, y: 0, filter: 'blur(0px)' },
  exit: { opacity: 0, y: -6, filter: 'blur(4px)' },
}
export const fadeOnly = {
  initial: { opacity: 0 },
  animate: { opacity: 1 },
  exit: { opacity: 0 },
}

/* Edge glow: seconds per lap around the window */
export const glowLap = {
  wifi: 2.8,
  cable: 1.05,
  ethernet: 1.6,
  other: 2.8,
  working: 3.6,
}

/* Simulated link rates (bytes per second). Demo durations derive from these. */
export const linkRate = {
  wifi: 64e6,
  cable: 410e6,
  ethernet: 110e6,
  other: 64e6,
}
