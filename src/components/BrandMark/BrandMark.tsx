import { useId } from 'react'

/** Placeholder app mark for SeamDrop: a rounded frame lit
    from blue (sending end) to orange (receiving end). */
export function BrandMark({ size = 16 }: { size?: number }) {
  const id = `pm${useId().replace(/[^a-zA-Z0-9]/g, '')}`
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden focusable="false">
      <defs>
        <linearGradient id={id} x1="6" y1="6" x2="26" y2="26" gradientUnits="userSpaceOnUse">
          <stop offset="0" stopColor="var(--blue-500)" />
          <stop offset="1" stopColor="var(--orange-500)" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="9" fill="var(--gray-900)" />
      <rect x="8.5" y="8.5" width="15" height="15" rx="5" fill="none" stroke={`url(#${id})`} strokeWidth="3.4" />
    </svg>
  )
}
