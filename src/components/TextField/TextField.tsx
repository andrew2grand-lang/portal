import { useId } from 'react'
import type { InputHTMLAttributes, ReactNode } from 'react'
import s from './TextField.module.css'

interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'value'> {
  label: ReactNode
  value: string
  onChange: (value: string) => void
  /** Enter key */
  onSubmit?: () => void
  hint?: ReactNode
  error?: string
  /** Addresses and paths read better in the mono face */
  mono?: boolean
}

/** A sunken field in the tray style of the keys: label above, hint or error below */
export function TextField({ label, value, onChange, onSubmit, hint, error, mono, className, ...rest }: TextFieldProps) {
  const id = useId()
  const noteId = `${id}-note`
  return (
    <div className={className ? `${s.field} ${className}` : s.field}>
      <label className={s.label} htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className={s.input}
        data-mono={mono ? '' : undefined}
        value={value}
        aria-invalid={error ? true : undefined}
        aria-describedby={error || hint ? noteId : undefined}
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && onSubmit) {
            e.preventDefault()
            onSubmit()
          }
        }}
        {...rest}
      />
      {(error || hint) && (
        <p id={noteId} className={s.note} data-error={error ? '' : undefined} role={error ? 'alert' : undefined}>
          {error ?? hint}
        </p>
      )}
    </div>
  )
}
