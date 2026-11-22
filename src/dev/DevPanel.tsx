import { useEffect } from 'react'
import type { ReactNode } from 'react'
import { usePortal } from '../engine/context'
import s from './DevPanel.module.css'

/* Developer settings only. Not part of the design: hidden, toggled with the
   backtick key (`). Kept for this browser only. */

function Seg<T extends string | number>({
  value,
  options,
  onChange,
}: {
  value: T
  options: { value: T; label: string }[]
  onChange: (v: T) => void
}) {
  return (
    <span className={s.seg}>
      {options.map((o) => (
        <button key={String(o.value)} type="button" aria-pressed={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </span>
  )
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className={s.row}>
      <span className={s.label}>{label}</span>
      {children}
    </div>
  )
}

export function DevPanel() {
  const { world, dispatch } = usePortal()
  const { dev } = world

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== '`' || e.metaKey || e.ctrlKey || e.altKey) return
      e.preventDefault()
      dispatch({ type: 'dev', patch: { open: !world.dev.open } })
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [dispatch, world.dev.open])

  // Mirror forced reduced motion to CSS
  useEffect(() => {
    if (dev.reducedMotion) document.documentElement.dataset.reducedMotion = ''
    else delete document.documentElement.dataset.reducedMotion
  }, [dev.reducedMotion])

  if (!dev.open) return null
  const set = (patch: Partial<typeof dev>) => dispatch({ type: 'dev', patch })

  return (
    <aside className={s.panel} aria-label="Developer settings">
      <header className={s.header}>
        <span>Developer settings</span>
        <kbd>`</kbd>
      </header>
      <Row label="Theme">
        <Seg
          value={dev.theme}
          options={[
            { value: 'light', label: 'Light' },
            { value: 'dark', label: 'Dark' },
          ]}
          onChange={(theme) => set({ theme })}
        />
      </Row>
      <Row label="Motion">
        <Seg
          value={dev.reducedMotion ? 'reduced' : 'full'}
          options={[
            { value: 'full', label: 'Full' },
            { value: 'reduced', label: 'Reduced' },
          ]}
          onChange={(v) => set({ reducedMotion: v === 'reduced' })}
        />
      </Row>
      {/* Mission 04 §5: hidden for now; the code stays */}
      <Row label="Project tab">
        <Seg
          value={dev.projectTab ? 'shown' : 'hidden'}
          options={[
            { value: 'hidden', label: 'Hidden' },
            { value: 'shown', label: 'Shown' },
          ]}
          onChange={(v) => set({ projectTab: v === 'shown' })}
        />
      </Row>
    </aside>
  )
}
