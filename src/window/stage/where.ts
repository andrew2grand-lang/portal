/* Small shared helpers for the stage views: where things go and how folders
   and times are shown. */
import type { Side } from '../../engine/types'

/* OS conventions for the same action */
export const revealLabel: Record<Side, string> = { mac: 'Show in Finder', win: 'Show in File Explorer' }

/** "Downloads/SeamDrop" (the last two parts, in this OS's style) */
export function shortFolder(path: string | undefined, side: Side): string {
  if (!path) return 'Downloads'
  return path.split(/[\\/]/).filter(Boolean).slice(-2).join(side === 'win' ? '\\' : '/')
}

export { targets } from '../../engine/views'

const pad = (n: number) => String(n).padStart(2, '0')
/** "14:05", local time */
export const clock = (ms: number) => {
  const d = new Date(ms)
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`
}
