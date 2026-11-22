import type { ReactNode } from 'react'
import s from './Chip.module.css'

export type ChipTone = 'neutral' | 'send' | 'land' | 'danger'

interface ChipProps {
  tone?: ChipTone
  icon?: ReactNode
  children: ReactNode
  className?: string
}

/** Pill tag for status and link type */
export function Chip({ tone = 'neutral', icon, children, className }: ChipProps) {
  return (
    <span className={className ? `${s.chip} ${className}` : s.chip} data-tone={tone}>
      {icon && (
        <span className={s.icon} aria-hidden>
          {icon}
        </span>
      )}
      <span>{children}</span>
    </span>
  )
}
