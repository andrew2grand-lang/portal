import type { DevState } from './types'

export const DEV_KEY = 'portal.dev'

/** The dev panel's choices, kept for the next session (this browser only) */
export function loadDev(): DevState | undefined {
  try {
    const raw = localStorage.getItem(DEV_KEY)
    if (!raw) return undefined
    const saved = JSON.parse(raw) as Partial<DevState>
    return {
      open: false,
      theme: saved.theme === 'dark' ? 'dark' : 'light',
      reducedMotion: !!saved.reducedMotion,
      projectTab: !!saved.projectTab,
    }
  } catch {
    return undefined
  }
}

export function saveDev(dev: DevState) {
  try {
    localStorage.setItem(DEV_KEY, JSON.stringify({ theme: dev.theme, reducedMotion: dev.reducedMotion, projectTab: dev.projectTab }))
  } catch {
    /* not kept: private window or blocked storage */
  }
}
