import { linkLabel } from './format'
import { names } from './words'
import type { Peer, Session, World } from './types'

/* What the window shows, derived from the world */
export type ViewKind =
  | 'idle'
  | 'searching'
  | 'nearby'
  | 'pairing'
  | 'connecting'
  | 'offline'
  | 'disconnected'
  | 'incompatible'
  | 'settings'
  | 'connect'
  | 'doctor'
  | 'clipboard'
  /** The Shelf: things you keep (Mission 05 §3.2) */
  | 'shelf'
  /** Help: every explanation in one place (Mission 05 §6) */
  | 'help'
  | 'add'
  /** What happened (Details, open) */
  | 'details'
  /** One transfer: its own screen, as with one device */
  | 'session'
  /** Several transfers: one row each */
  | 'sessions'
  /** Before the core's first answer: nothing yet */
  | 'starting'

/** Transfers this window shows (refused or declined ones are only news to the sender) */
export function shownSessions(w: World): Session[] {
  return w.sessions.filter((s) => s.direction === 'out' || (s.phase !== 'blocked' && s.phase !== 'declined'))
}

/** A device's one transfer slot is taken (one at a time per device) */
export function busyWith(w: World, id: string): boolean {
  return w.sessions.some((s) => s.peer === id && (s.phase === 'transfer' || s.phase === 'paused' || s.phase === 'offer'))
}

/** The ticked devices: where the next files go now, which are busy with a
    transfer, and which aren't connected */
export function targets(w: World): { to: Peer[]; busy: Peer[]; away: Peer[] } {
  const chosen = w.peers.filter((p) => p.paired && w.target.includes(p.id))
  const connected = chosen.filter((p) => p.status === 'connected')
  return {
    to: connected.filter((p) => !busyWith(w, p.id)),
    busy: connected.filter((p) => busyWith(w, p.id)),
    away: chosen.filter((p) => p.status !== 'connected'),
  }
}

/** The send list's title: "Ready to send" only when something can go now;
    else what the ticked computer is busy with (Mission 05 §1.5) */
export function sendListTitle(w: World): string {
  const { to, busy } = targets(w)
  if (to.length || !busy.length) return 'Ready to send'
  if (busy.length > 1) return `Busy with ${names(busy.map((p) => p.name))}.`
  const p = busy[0]
  const x = w.sessions.find((s) => s.peer === p.id && (s.phase === 'transfer' || s.phase === 'paused' || s.phase === 'offer'))
  if (x?.phase === 'paused') return `Busy: a paused transfer with ${p.name}.`
  return x?.direction === 'in' ? `Busy receiving from ${p.name}.` : `Busy sending to ${p.name}.`
}

/** Every computer this one can see or knows, for the nearby list (Mission
    05 §1.8): never one picked for the person. Paired ones are marked (just
    paired: the next can follow); nearby ones first, by name. */
export function nearbyRows(w: World): { peer: Peer; paired: boolean }[] {
  return w.peers
    .filter((p) => !p.paired || p.status === 'connected')
    .map((peer) => ({ peer, paired: peer.paired }))
    .sort((a, b) => Number(a.paired) - Number(b.paired) || a.peer.name.localeCompare(b.peer.name))
}

export function viewFor(w: World): ViewKind {
  if (!w.ready) return 'starting'
  if (w.panel) return w.panel
  if (w.pairing) return 'pairing'
  const sessions = shownSessions(w)
  // Files added while transfers run: the send list shows (an offer waiting
  // for an answer comes first)
  const answer = sessions.some((s) => s.phase === 'offer' && s.direction === 'in')
  if (w.queue.length > 0 && sessions.length > 0 && !answer) return 'idle'
  if (sessions.length === 1) return 'session'
  if (sessions.length > 1) return 'sessions'
  const paired = w.peers.filter((p) => p.paired)
  if (paired.length === 0) {
    const focus = w.focus
    if (focus?.status === 'incompatible') return 'incompatible'
    return focus ? 'nearby' : 'searching'
  }
  // Several devices: the drop zone, with each device's status beside it
  if (paired.length > 1 || paired[0].status === 'connected') return 'idle'
  switch (paired[0].status) {
    case 'connecting':
      return 'connecting'
    case 'unreachable':
      return 'offline'
    case 'disconnected':
      return 'disconnected'
    case 'incompatible':
      return 'incompatible'
    case 'revoked':
      return 'nearby'
    default:
      return 'idle'
  }
}

export type GlowMode = 'off' | 'sending' | 'receiving' | 'working'

/** Sending while anything leaves; receiving while anything arrives. Nothing
    moving (stalled, paused): no travelling light */
export function glowFor(w: World): GlowMode {
  const moving = w.sessions.filter((s) => s.phase === 'transfer' && s.linkNote !== 'stalled')
  if (moving.some((s) => s.direction === 'out')) return 'sending'
  if (moving.some((s) => s.direction === 'in')) return 'receiving'
  return 'off'
}

/** Changes once per delivery: drives the arrival flash */
export function arrivalFor(w: World): string | null {
  const done = w.sessions.filter((s) => s.direction === 'in' && s.phase === 'done')
  return done.at(-1)?.id ?? null
}

/** The transfer list's heading, counting what's on screen:
    "2 moving · 1 paused · 1 done", "2 waiting for you" */
export function listTitle(sessions: Session[]): string {
  const count = (f: (s: Session) => boolean) => sessions.filter(f).length
  const parts: [number, string][] = [
    [count((s) => s.phase === 'transfer'), 'moving'],
    [count((s) => s.phase === 'paused'), 'paused'],
    [count((s) => s.phase === 'offer' && s.direction === 'in'), 'waiting for you'],
    [count((s) => s.phase === 'offer' && s.direction === 'out'), 'waiting to be accepted'],
    [count((s) => s.phase === 'done'), 'done'],
    [count((s) => s.phase === 'declined' || s.phase === 'blocked'), 'declined'],
    [count((s) => s.phase === 'failed'), 'stopped'],
  ]
  return (
    parts
      .filter(([n]) => n > 0)
      .map(([n, what]) => `${n} ${what}`)
      .join(' · ') || 'Transfers'
  )
}

/** The drop zone's one line (beta.2): where dropped files go, and over what
    ("To Desk PC · over Wi-Fi", "To Desk PC and MacBook Air", "To 4 computers") */
export function dropLine(to: Pick<Peer, 'name' | 'link'>[]): string {
  if (!to.length) return ''
  const who = to.length > 3 ? `${to.length} computers` : names(to.map((p) => p.name))
  const links = new Set(to.map((p) => p.link))
  const [only] = [...links]
  const over = links.size === 1 && only && only !== 'other' ? ` · over ${linkLabel[only]}` : ''
  return `To ${who}${over}`
}

