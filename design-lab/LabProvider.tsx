import { useEffect, useMemo } from 'react'
import type { ReactNode } from 'react'
import { MotionConfig, useMotionValue } from 'motion/react'
import type { MotionValue } from 'motion/react'
import { useThemeEffect } from '../src/engine/theme'
import type { LogLine, Side, Theme } from '../src/engine/types'
import { linkRate } from '../src/styles/motion'
import { LabContext } from './twoDevices'

interface LabProviderProps {
  theme: Theme
  reducedMotion: boolean
  /** The Details log at the foot of each window */
  logs: Record<Side, LogLine[]>
  /** 0..1, for screens that show progress or a filling edge */
  progress?: MotionValue<number>
  children: ReactNode
}

/** The lab's still world: two paired computers on Wi-Fi with Project
    selected. Each window builds its own view of it (LabWindow). */
export function LabProvider({ theme, reducedMotion, logs, progress, children }: LabProviderProps) {
  const own = useMotionValue(0)
  const rate = useMotionValue(linkRate.wifi)
  const eta = useMotionValue('')
  const settings = useMemo(
    () => ({ theme, reducedMotion, logs, progress: progress ?? own, rate, eta }),
    [theme, reducedMotion, logs, progress, own, rate, eta],
  )

  useThemeEffect(theme)
  useEffect(() => {
    if (reducedMotion) document.documentElement.dataset.reducedMotion = ''
    else delete document.documentElement.dataset.reducedMotion
  }, [reducedMotion])

  return (
    <LabContext value={settings}>
      <MotionConfig reducedMotion={reducedMotion ? 'always' : 'user'}>{children}</MotionConfig>
    </LabContext>
  )
}
