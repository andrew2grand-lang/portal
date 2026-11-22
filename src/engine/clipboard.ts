/* What the window says about the shared clipboard (Mission 03 §4, Mission
   04 §3–4), as pure functions. Automatic sync runs between two devices only
   when both have said yes. */
import type { ClipNotice, Peer, PeerClip } from './types'

/** Where copies go and what never does, said exactly (Mission 04 §4.4) */
export const PRIVATE_RULE =
  'Goes only to your paired devices, encrypted, never through a server. Anything an app marks as private, as password managers do, is never sent.'

/** Which browsers mark what's copied in a private window, on Windows:
    Edge InPrivate and Chrome incognito as measured (decision 214), Firefox
    and Tor Browser as Mozilla documents (and as seen on a real PC). Other
    browsers may not, so the note says so rather than promise. */
export const BROWSER_NOTE =
  'On Windows, what you copy in a private window of Edge, Chrome or Firefox (Tor Browser too) is marked private, so it isn’t sent. Other browsers may not mark it: copy from them with care.'

/** Automatic sync's rule, said once above the devices */
export const SYNC_RULE = 'Both computers have to turn it on; turning it off on either one stops it.'

/** Where automatic sync with one device stands, in one sentence */
export function syncLine(c: PeerClip | undefined, peer: string, paused: boolean): string {
  if (!c) return ''
  if (c.mine && c.theirs) {
    return paused
      ? `On with ${peer}, paused here: nothing goes either way until the pause ends.`
      : `On: what you copy here or on ${peer} is ready to paste on the other within a second.`
  }
  if (c.mine) return `On here. Nothing syncs until ${peer} turns it on too.`
  if (c.asks) return `${peer} asks to sync. Nothing syncs unless you turn it on.`
  return 'Off.'
}

/** The Clipboard place's one-line status (Mission 05 §3.1): "Syncing with
    macbook one", "Off", or why it waits */
export function syncStatus(paired: Peer[], paused: boolean, until?: number): string {
  const on = paired.filter((p) => p.clip?.mine && p.clip?.theirs)
  if (!on.length) return paired.some((p) => p.clip?.mine) ? 'Sync waits for the other computer to agree.' : 'Automatic sync is off.'
  if (paused) {
    const t = until ? new Date(until) : undefined
    return t ? `Sync paused until ${String(t.getHours()).padStart(2, '0')}:${String(t.getMinutes()).padStart(2, '0')}.` : 'Sync paused until SeamDrop restarts.'
  }
  const away = on.filter((p) => p.status !== 'connected')
  const here = on.filter((p) => p.status === 'connected')
  if (!here.length) return `${listNames(away.map((p) => p.name))} ${away.length > 1 ? 'aren’t' : 'isn’t'} reachable; sync waits.`
  return `Syncing with ${listNames(here.map((p) => p.name))}.`
}

/** One device's sync state in a few words, beside its switch */
export function syncWord(c: PeerClip | undefined, peer: string, paused: boolean): string {
  if (!c) return ''
  if (c.mine && c.theirs) return paused ? 'Paused here' : 'On'
  if (c.mine) return `Waiting for ${peer}`
  if (c.asks) return `${peer} asks`
  return 'Off'
}

/** A copy that wasn't synced, in one sentence (Mission 05 §3.1) */
export function noticeWords(kind: ClipNotice['kind']): { text: string } {
  switch (kind) {
    case 'private':
      return { text: 'Not synced: the app you copied from marked it private.' }
    case 'tooBig':
      return { text: 'Too big for the clipboard.' }
    case 'unreadable':
      return { text: 'Couldn’t read that copy. Try Send clipboard.' }
  }
}

function listNames(list: string[]): string {
  if (list.length <= 1) return list[0] ?? ''
  return `${list.slice(0, -1).join(', ')} and ${list[list.length - 1]}`
}
