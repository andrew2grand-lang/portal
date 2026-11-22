/* What the window says about updates (Mission 03 §3), as pure functions. */
import { APP_NAME } from '../platform'
import type { UpdateInfo } from './types'
import { names } from './words'

/** What the notice says, or null when it stays out of the way. `paused`:
    the devices a paused transfer waits for (they continue after a restart). */
export function noticeFor(
  u: UpdateInfo | undefined,
  paused: string[] = [],
): { title: string; sub?: string; keys: 'offer' | 'later' | 'failed' | null } | null {
  if (!u) return null
  const next = `${APP_NAME} ${u.version ?? ''}`.trim()
  // The first paragraph only: the notice is one calm line, never a wall of text
  const lead = u.notes?.split(/\n\s*\n/)[0].trim()
  const whatsNew = lead ? `What’s new: ${lead}` : undefined
  const transfers = paused.length > 1 ? 'transfers' : 'transfer'
  const kept = paused.length ? `The paused ${transfers} with ${names(paused)} continue${paused.length > 1 ? '' : 's'}` : ''
  const offerSub = [whatsNew, kept && `${kept} after the restart.`].filter(Boolean).join(' ') || undefined
  switch (u.state) {
    case 'ready':
      return u.dismissed ? null : { title: `${next} is ready`, sub: offerSub, keys: 'offer' }
    case 'available':
      return u.dismissed ? null : { title: `${next} is available`, sub: offerSub, keys: 'offer' }
    case 'waiting':
      return {
        title: `${APP_NAME} restarts to update when the transfer finishes`,
        sub: `${APP_NAME} waits for the transfer with ${u.peer}, then installs ${u.version}. Nothing is lost.`,
        keys: 'later',
      }
    case 'downloading':
      return u.requested ? { title: `Downloading ${next}… ${u.percent ?? 0}%`, sub: whatsNew, keys: null } : null
    case 'installing':
      return { title: `Installing ${next}`, sub: `${APP_NAME} restarts in a moment.${kept ? ` ${kept} after it.` : ''}`, keys: null }
    case 'failed':
      return u.requested ? { title: 'The update didn’t install', sub: u.message, keys: 'failed' } : null
    default:
      return null
  }
}

/** The key that installs: it names the app, so nobody thinks the computer restarts */
export function restartLabel(u: UpdateInfo): string {
  return u.state === 'ready' ? `Restart ${APP_NAME} to update` : `Update and restart ${APP_NAME}`
}

const pad = (n: number) => String(n).padStart(2, '0')

/** One sentence on where updates stand, said plainly */
export function updateLine(u: UpdateInfo): string {
  const next = `${APP_NAME} ${u.version ?? ''}`.trim()
  switch (u.state) {
    case 'idle':
      return 'Checks for updates by itself every few hours.'
    case 'checking':
      return 'Checking for updates…'
    case 'upToDate': {
      if (!u.checkedAt) return 'Up to date.'
      const d = new Date(u.checkedAt)
      return `Up to date. Checked at ${pad(d.getHours())}:${pad(d.getMinutes())}.`
    }
    case 'available':
      return `${next} is available.`
    case 'downloading':
      return `Downloading ${next}… ${u.percent ?? 0}%`
    case 'ready':
      return `${next} is ready. Restart ${APP_NAME} to install it.`
    case 'waiting':
      return `Restarts to install ${u.version} when the transfer with ${u.peer} finishes.`
    case 'installing':
      return `Installing ${next}…`
    case 'failed':
      return u.message ?? 'Couldn’t check for updates.'
  }
}

