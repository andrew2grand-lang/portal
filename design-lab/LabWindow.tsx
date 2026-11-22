import { useMemo } from 'react'
import type { ReactNode } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { GearSixIcon } from '@phosphor-icons/react'
import { EdgeGlow } from '../src/components/EdgeGlow/EdgeGlow'
import { Key } from '../src/components/Key/Key'
import { ModeSwitch } from '../src/components/ModeSwitch/ModeSwitch'
import { PortalContext } from '../src/engine/context'
import type { Side } from '../src/engine/types'
import type { GlowMode } from '../src/engine/views'
import { easeOut, fadeOnly, materialize } from '../src/styles/motion'
import { AppWindow } from '../src/window/AppWindow'
import { Foot } from '../src/window/Details'
import { PeerHeader } from '../src/window/PortalWindow'
import pw from '../src/window/PortalWindow.module.css'
import st from '../src/window/stage/Stage.module.css'
import { WindowContext } from '../src/window/WindowContext'
import { useLab, worldFor } from './twoDevices'

interface LabWindowProps {
  side: Side
  /** Changes when the screen or state changes, so the stage materialises */
  viewKey: string
  glow?: GlowMode
  arrival?: string | null
  settingsOpen?: boolean
  onMode?: (mode: 'files' | 'project') => void
  children: ReactNode
}

/** One SeamDrop window as the app draws it (after Mission 02's fix: no role
    keys), with the Project tab selected and a lab screen on the stage. */
export function LabWindow({ side, viewKey, glow = 'off', arrival = null, settingsOpen, onMode, children }: LabWindowProps) {
  const lab = useLab()
  const world = useMemo(() => worldFor(side, lab), [side, lab])
  const portal = useMemo(
    () => ({ world, dispatch: () => {}, live: () => ({ progress: lab.progress, rate: lab.rate, eta: lab.eta }) }),
    [world, lab],
  )
  const reduced = lab.reducedMotion || (typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches)
  const t = reduced ? fadeOnly : materialize
  return (
    <PortalContext value={portal}>
    <WindowContext value={side}>
      <AppWindow
        side={side}
        accessory={
          <Key
            size="sm"
            variant={settingsOpen ? 'lit' : 'plain'}
            icon={<GearSixIcon />}
            aria-label="Settings"
            aria-pressed={!!settingsOpen}
          />
        }
        glow={<EdgeGlow mode={glow} link="wifi" arrival={arrival} />}
      >
        <PeerHeader peer={world.focus} />
        <div className={pw.controls}>
          <ModeSwitch id={`lab-${side}`} value="project" onChange={(m) => onMode?.(m)} />
        </div>
        <div className={st.shell}>
          <div className={st.core}>
            <AnimatePresence mode="popLayout" initial={false}>
              <motion.div
                key={viewKey}
                className={st.view}
                initial={t.initial}
                animate={t.animate}
                exit={t.exit}
                transition={{ duration: reduced ? 0.2 : 0.4, ease: easeOut }}
              >
                {children}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>
        <Foot />
      </AppWindow>
    </WindowContext>
    </PortalContext>
  )
}
