import { useRef } from 'react'
import type { KeyboardEvent } from 'react'
import { motion } from 'motion/react'
import { spring } from '../../styles/motion'
import s from './Choice.module.css'

interface ChoiceProps<T extends string> {
  /** Unique per instance, so each thumb slides on its own */
  id: string
  /** Names the group for assistive tech ("Files from MacBook Pro") */
  label: string
  value: T
  options: { value: T; label: string }[]
  onChange: (value: T) => void
}

/** One of a few choices, with the mode switch's sliding thumb */
export function Choice<T extends string>({ id, label, value, options, onChange }: ChoiceProps<T>) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([])
  const onKeyDown = (e: KeyboardEvent, index: number) => {
    const step = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0
    if (!step) return
    e.preventDefault()
    const next = (index + step + options.length) % options.length
    onChange(options[next].value)
    buttons.current[next]?.focus()
  }
  return (
    <div className={s.track} role="radiogroup" aria-label={label}>
      {options.map((o, i) => {
        const selected = o.value === value
        return (
          <button
            key={o.value}
            ref={(el) => {
              buttons.current[i] = el
            }}
            type="button"
            role="radio"
            aria-checked={selected}
            tabIndex={selected ? 0 : -1}
            className={s.option}
            onClick={() => onChange(o.value)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {selected && <motion.span layoutId={`choice-${id}`} className={s.thumb} transition={spring.layout} />}
            <span className={s.label}>{o.label}</span>
          </button>
        )
      })}
    </div>
  )
}
