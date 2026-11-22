import { useRef } from 'react'
import type { KeyboardEvent } from 'react'
import { motion } from 'motion/react'
import type { Mode } from '../../engine/types'
import { spring } from '../../styles/motion'
import s from './ModeSwitch.module.css'

const modes: { value: Mode; label: string; paid?: boolean }[] = [
  { value: 'files', label: 'Files' },
  { value: 'project', label: 'Project', paid: true },
]

interface ModeSwitchProps {
  /** Unique per window, so each thumb slides independently */
  id: string
  value: Mode
  onChange: (mode: Mode) => void
  /** Locked while a transfer is in flight */
  locked?: boolean
}

/** Files | Project segmented switch with a sliding thumb */
export function ModeSwitch({ id, value, onChange, locked = false }: ModeSwitchProps) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([])

  const onKeyDown = (e: KeyboardEvent, index: number) => {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key) || locked) return
    e.preventDefault()
    const next = (index + 1) % modes.length
    onChange(modes[next].value)
    buttons.current[next]?.focus()
  }

  return (
    <div className={s.track} role="radiogroup" aria-label="Mode" data-locked={locked || undefined}>
      {modes.map((m, i) => {
        const selected = m.value === value
        return (
          <button
            key={m.value}
            ref={(el) => {
              buttons.current[i] = el
            }}
            type="button"
            role="radio"
            aria-checked={selected}
            aria-disabled={locked || undefined}
            tabIndex={selected ? 0 : -1}
            className={s.option}
            onClick={() => !locked && onChange(m.value)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {selected && <motion.span layoutId={`mode-thumb-${id}`} className={s.thumb} transition={spring.layout} />}
            <span className={s.label}>{m.label}</span>
            {m.paid && <span className={s.paid}>Paid</span>}
          </button>
        )
      })}
    </div>
  )
}
