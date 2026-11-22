import s from './Spinner.module.css'

/** Small working indicator; slows to a soft pulse under reduced motion */
export function Spinner({ size = 16, label }: { size?: number; label?: string }) {
  return (
    <span
      className={s.spinner}
      style={{ width: size, height: size }}
      role={label ? 'status' : undefined}
      aria-label={label}
      aria-hidden={label ? undefined : true}
    />
  )
}
