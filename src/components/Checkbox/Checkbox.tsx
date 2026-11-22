import type { ReactNode } from 'react'
import { CheckIcon } from '@phosphor-icons/react'
import s from './Checkbox.module.css'

interface CheckboxProps {
  checked: boolean
  onChange: (checked: boolean) => void
  children: ReactNode
}

export function Checkbox({ checked, onChange, children }: CheckboxProps) {
  return (
    <label className={s.check}>
      <input
        type="checkbox"
        className={s.input}
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className={s.box} aria-hidden>
        <CheckIcon weight="bold" />
      </span>
      <span className={s.text}>{children}</span>
    </label>
  )
}
