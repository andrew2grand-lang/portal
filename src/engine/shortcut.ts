/* The Send clipboard shortcut: how each system writes it, and recording a
   new one from a key press. Accelerator form is what the shell registers
   ("Control+Alt+Super+C"). */
import type { Side } from './types'

const MAC: Record<string, string> = { Control: '⌃', Alt: '⌥', Shift: '⇧', Super: '⌘' }
const WIN: Record<string, string> = { Control: 'Ctrl', Alt: 'Alt', Shift: 'Shift', Super: 'Win' }

function keyName(code: string): string {
  if (code.startsWith('Key')) return code.slice(3)
  if (code.startsWith('Digit')) return code.slice(5)
  return code
}

/** "⌃⌥⌘C" on a Mac, "Ctrl+Alt+Shift+C" on Windows */
export function shortcutLabel(accelerator: string, side: Side): string {
  if (!accelerator) return ''
  const parts = accelerator.split('+')
  const key = keyName(parts.pop() ?? '')
  if (side === 'mac') return parts.map((p) => MAC[p] ?? p).join('') + key
  return [...parts.map((p) => WIN[p] ?? p), key].join('+')
}

/** A key press as a shortcut, or null until it has a modifier besides Shift and a key */
export function acceleratorFrom(e: KeyboardEvent, side: Side): string | null {
  if (['Control', 'Alt', 'Shift', 'Meta', 'OS'].includes(e.key)) return null
  const mods = [e.ctrlKey && 'Control', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean) as string[]
  if (!mods.some((m) => m !== 'Shift')) return null
  if (side === 'win' && e.metaKey) return null // the Windows key belongs to Windows
  const code = /^(Key[A-Z]|Digit\d|F\d{1,2})$/.test(e.code) ? e.code : null
  return code ? [...mods, code.startsWith('Key') ? code.slice(3) : code].join('+') : null
}
