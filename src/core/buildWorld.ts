/* Pure mapping from the core's snapshot to the World every view renders.
   Devices are keyed by id; this computer is one of them (Mission 04 §3). */
import { genericDoctor } from '../engine/words'
import type {
  DevState,
  Device,
  DeviceStatus,
  LogLine,
  Mode,
  Panel,
  Peer,
  Phase,
  QueuedItem,
  ReachInfo,
  Session,
  UpdateInfo,
  World,
} from '../engine/types'
import type { CoreLogLine, CoreOs, CorePeer, CorePhase, CoreReach, CoreShortcut, CoreSnapshot, CoreTransfer, CoreUpdate } from './bridge'

/** UI-only state the core doesn't know about */
export interface CoreUi {
  panel: Panel | null
  /** Help opened from a "How … works" link: the section to show first */
  helpTopic?: string
  connect: { busy: boolean; error?: string }
  dev: DevState
  mode: Mode
  openAtLogin: boolean
  dragOver: boolean
  /** Sends that didn't start, in plain words (shown until dismissed) */
  sendErrors: string[]
  /** The file moving right now, per transfer */
  currentFile: Record<string, string>
  /** The send list (lives here, not in the core, until Send is pressed) */
  queue: QueuedItem[]
  /** Updates, from the shell (null until it answers) */
  update: CoreUpdate | null
  /** The Send clipboard shortcut, from the shell */
  shortcut: CoreShortcut | null
  /** What the last Send clipboard said (with files: offered, not sent) */
  clipResult?: { ok: boolean; text: string; at: number; files?: string[] }
  /** Devices whose pairing request was ignored here (this session) */
  ignored?: string[]
  /** The firewall "Fix it": running, or what it said */
  fix?: { busy: boolean; ok?: boolean; text?: string }
  /** Where things go, as ticked here, until the core has saved it */
  target?: string[]
  /** A private Shelf item shown for a few seconds (never saved anywhere) */
  revealed?: { id: string; text: string }
  /** Undo after a removal from the Shelf, until then (epoch ms) */
  shelfUndoUntil?: number
}

export function defaultUi(dev?: DevState): CoreUi {
  return {
    panel: null,
    connect: { busy: false },
    dev: dev ?? { open: false, theme: 'light', reducedMotion: false, projectTab: false },
    mode: 'files',
    openAtLogin: false,
    dragOver: false,
    sendErrors: [],
    currentFile: {},
    queue: [],
    update: null,
    shortcut: null,
  }
}

const osNames: Record<CoreOs, { os: string; short: string; self: string; kind: Device['kind'] }> = {
  macos: { os: 'macOS', short: 'Mac', self: 'this Mac', kind: 'mac' },
  windows: { os: 'Windows', short: 'PC', self: 'this PC', kind: 'pc' },
  linux: { os: 'Linux', short: 'computer', self: 'this computer', kind: 'other' },
  other: { os: 'Computer', short: 'computer', self: 'this computer', kind: 'other' },
}

export function device(id: string, name: string, os: CoreOs): Device {
  return { id, name, ...osNames[os] }
}

/** Before the core answers; only ever shown mid-sentence */
const placeholderMe: Device = { id: '', name: 'This computer', os: '', short: 'computer', self: 'this computer', kind: 'other' }

function statusOf(p: CorePeer): DeviceStatus {
  if (p.incompatible) return 'incompatible'
  if (p.revoked) return 'revoked'
  if (!p.paired) return 'nearby'
  if (p.disconnected) return 'disconnected'
  if (p.online) return 'connected'
  // Only the core says "isn't reachable" (after its grace period and real failed attempts)
  return p.reach?.state === 'unreachable' ? 'unreachable' : 'connecting'
}

const reachSteps: Record<CoreReach['step'], string> = {
  looking: 'Looking on this network',
  trying_known: 'Trying the address it was last seen at',
  trying_found: 'Found it on this network, connecting',
}

export function reachInfo(p: CorePeer): ReachInfo | undefined {
  const r = p.reach
  if (!r) return undefined
  return { doing: reachSteps[r.step], lastFailure: r.lastFailure ?? undefined, attempts: r.attempts }
}

function peerOf(p: CorePeer, myProto: number): Peer {
  return {
    ...device(p.id, p.name, p.os),
    picture: p.picture ?? null,
    paired: p.paired,
    status: statusOf(p),
    link: p.online && p.link ? p.link : undefined,
    reach: reachInfo(p),
    incoming: p.incoming,
    accepts: p.accepts ?? undefined,
    // Theirs newer than ours: this computer is the one to update
    outdated: p.incompatible ? (p.incompatible > myProto ? 'self' : 'peer') : undefined,
    addr: p.addr ?? undefined,
    clip: p.clip ?? undefined,
    shelf: p.shelf ?? undefined,
  }
}

/** Paired first (connected first), then nearby; stable by name within each */
function order(a: Peer, b: Peer): number {
  const rank = (p: Peer) => (p.paired ? (p.status === 'connected' ? 0 : 1) : 2)
  return rank(a) - rank(b) || a.name.localeCompare(b.name)
}

/** The device single-device screens talk about */
export function focusOf(peers: Peer[], pairingPeer?: string): Peer | undefined {
  return (
    peers.find((p) => p.id === pairingPeer) ??
    (peers.filter((p) => p.paired).length === 1 ? peers.find((p) => p.paired) : undefined) ??
    peers.find((p) => p.paired && p.status === 'connected') ??
    peers.find((p) => p.paired) ??
    peers.find((p) => p.status === 'incompatible') ??
    peers.find((p) => p.status === 'revoked') ??
    peers[0]
  )
}

const pad = (n: number) => String(n).padStart(2, '0')
function clock(ms: number) {
  const d = new Date(ms)
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

/** A run of identical lines in a row (same tag, words and tone), kept as
    one: Details shows "×12, last at 19:31" (Mission 04 §1.6) */
export interface LogRun {
  first: CoreLogLine
  latest: CoreLogLine
  count: number
}

const same = (a: CoreLogLine, b: CoreLogLine) => a.tag === b.tag && a.text === b.text && a.tone === b.tone

/** Adds a line: onto the last run when it's the same, else as a new run (the oldest go past `max`) */
export function appendLog(runs: LogRun[], line: CoreLogLine, max: number): LogRun[] {
  const last = runs.at(-1)
  if (last && same(last.first, line)) return [...runs.slice(0, -1), { ...last, latest: line, count: last.count + 1 }]
  return [...runs, { first: line, latest: line, count: 1 }].slice(-max)
}

export function toLogLines(runs: LogRun[]): LogLine[] {
  return runs.map((r, i) => ({
    id: r.first.atMs * 1000 + (i % 1000),
    time: clock(r.first.atMs),
    tag: r.first.tag,
    text: r.first.text,
    // The newest technical part (the size of the last copy, say)
    detail: r.latest.detail ?? undefined,
    tone: r.first.tone ?? undefined,
    count: r.count,
    last: r.count > 1 ? clock(r.latest.atMs).slice(0, 5) : undefined,
  }))
}

/**
 * @param now when this snapshot arrived (epoch ms), so the search clock stays stable between renders
 */
export function buildWorld(snap: CoreSnapshot | null, logs: LogRun[], ui: CoreUi, now: number): World {
  const me = snap?.me
  const peers = (snap?.peers ?? []).map((p) => peerOf(p, me?.proto ?? 0)).sort(order)
  const pairing = snap?.pairing ?? null
  const paired = peers.filter((p) => p.paired)
  const saved = ui.target ?? me?.sendTo ?? []
  const target = saved.filter((id) => paired.some((p) => p.id === id))
  return {
    ready: !!snap,
    me: me ? { ...device(me.id, me.name, me.os), picture: me.picture ?? null } : placeholderMe,
    peers,
    target: target.length ? target : paired.map((p) => p.id),
    focus: focusOf(peers, pairing?.peerId),
    update: ui.update ? updateInfo(ui.update, me?.autoUpdate ?? true, me?.testBuilds ?? false) : undefined,
    clipboard: snap
      ? {
          sync: snap.me.clipboardSync,
          paused: snap.me.clipboardPaused ?? undefined,
          shortcut: ui.shortcut?.shortcut ?? snap.me.clipboardShortcut ?? '',
          defaultShortcut: ui.shortcut?.default ?? '',
          shortcutError: ui.shortcut?.error ?? undefined,
          items: snap.clips.map((c) => ({
            id: c.id,
            from: c.from,
            fromId: c.fromId ?? undefined,
            at: c.atMs,
            kind: c.kind,
            summary: c.summary,
            preview: c.preview,
            copyable: c.copyable,
            pasted: c.pasted,
            private: c.private ?? false,
            thumb: c.thumb ?? undefined,
          })),
          last: ui.clipResult,
          notice: snap.clipNotice
            ? { id: snap.clipNotice.id, kind: snap.clipNotice.kind === 'too_big' ? 'tooBig' : snap.clipNotice.kind, count: snap.clipNotice.count, at: snap.clipNotice.atMs }
            : undefined,
        }
      : undefined,
    shelf: (snap?.shelf ?? []).map((i) => ({
      id: i.id,
      name: i.name,
      preview: i.preview ?? undefined,
      private: i.private,
      from: i.fromId,
      fromName: i.fromName,
      mine: i.mine,
      at: i.atMs,
    })),
    revealed: ui.revealed,
    shelfUndoUntil: ui.shelfUndoUntil,
    next: (snap?.next ?? []).map((n) => ({
      id: n.id,
      peer: n.peerId,
      names: n.names,
      folders: n.folders,
      held: n.held,
      afterPaused: n.waitingForPaused,
    })),
    searchStartedAt: now - (snap?.searchMs ?? 0),
    panel: ui.panel,
    helpTopic: ui.helpTopic,
    pairing: pairing && {
      peer: pairing.peerId,
      peerAddr: pairing.peerAddr,
      mine: pairing.mine,
      step: pairing.step,
      code: pairing.code ? `${pairing.code.slice(0, 3)} ${pairing.code.slice(3)}` : '',
      triesLeft: pairing.triesLeft,
      failed: pairing.failed ?? undefined,
      expiresAt: now + (pairing.leftMs ?? 120_000),
    },
    settings: {
      deviceName: me?.name ?? '',
      receiveDir: me?.receiveDir ?? '',
      skipJunk: me?.skipJunk ?? true,
      folderPerDevice: me?.folderPerDevice ?? false,
      receiveMode: me?.receiveMode ?? 'loose',
      keepDates: me?.keepDates ?? false,
      receiveNote: me?.receiveNote ?? undefined,
      pairingLocked: me?.pairingLocked ?? false,
      pairingLockedBy: me?.pairingLockedBy ?? undefined,
      pairingLockedAt: me?.pairingLockedAtMs ? clock(me.pairingLockedAtMs).slice(0, 5) : undefined,
      appVersion: me?.appVersion ?? '',
      osVersion: me?.osVersion ?? '',
      openAtLogin: ui.openAtLogin,
      addresses: me?.addresses ?? [],
    },
    doctor: snap?.doctor.length
      ? snap.doctor.map((c) => ({ id: c.id, status: c.status, title: c.title, fix: c.fix ?? undefined, action: c.action ?? undefined }))
      : genericDoctor,
    connect: ui.connect,
    fix: ui.fix,
    dragOver: ui.dragOver,
    currentFile: ui.currentFile,
    queue: ui.queue,
    ignored: ui.ignored ?? [],
    mode: ui.dev.projectTab ? ui.mode : 'files',
    sessions: (snap?.transfers ?? []).map(sessionOf),
    sendErrors: ui.sendErrors,
    logs: toLogLines(logs),
    dev: ui.dev,
    appearance: snap?.me.appearance ?? 'system',
  }
}

const phases: Record<CorePhase, Phase> = {
  preparing: 'offer',
  offered: 'offer',
  running: 'transfer',
  paused: 'paused',
  done: 'done',
  failed: 'failed',
  cancelled: 'failed',
  declined: 'declined',
  blocked: 'blocked',
}

/** A core transfer in the shape the stage views render */
export function sessionOf(t: CoreTransfer): Session {
  return {
    id: t.id,
    peer: t.peerId,
    direction: t.direction,
    payload: {
      items: t.items.map((i) => ({ name: i.name, kind: i.folder ? 'folder' : 'file', bytes: i.bytes, count: i.folder ? i.files : undefined })),
      totalBytes: t.totalBytes,
    },
    phase: phases[t.phase],
    block: t.block === 'not_accepted' ? 'not-accepted' : undefined,
    autoAccepted: t.autoAccepted,
    linkNote: t.stalled ? 'stalled' : null,
    startProgress: t.totalBytes ? t.bytes / t.totalBytes : 0,
    elapsed: t.elapsedMs === null ? undefined : t.elapsedMs / 1000,
    linkAtEnd: t.link ?? undefined,
    result: t.result
      ? {
          savedTo: t.result.savedTo ?? undefined,
          revealPath: t.result.reveal ?? undefined,
          openPath: t.result.open ?? undefined,
          files: t.result.files,
          renamed: t.result.renamed,
          skipped: t.result.skipped,
          longPaths: t.result.longPaths,
          keptBytes: t.result.keptBytes ?? 0,
          skippedItems: t.result.skippedItems ?? [],
          renamedItems: t.result.renamedItems ?? [],
          notes: t.result.notes,
          firstArrival: t.result.firstArrival,
        }
      : undefined,
    failure: t.failure ?? undefined,
    pausedSince: t.pausedSinceMs ?? undefined,
    resuming: t.resuming,
    stoppedHere: t.stoppedHere || undefined,
    waitingThere: t.waitingThere || undefined,
    percentKnown: t.percentKnown,
    cancelled: t.phase === 'cancelled',
    canSendAgain: t.canSendAgain,
    saveTo: t.saveTo ?? undefined,
  }
}

/** The shell's update state in the shape the window renders */
export function updateInfo(u: CoreUpdate, auto: boolean, testBuilds = false): UpdateInfo {
  return {
    state: u.state,
    current: u.current,
    version: u.version ?? undefined,
    notes: u.notes ?? undefined,
    percent: u.state === 'downloading' ? u.percent : undefined,
    peer: u.state === 'waiting' ? u.peer : undefined,
    message: u.state === 'failed' ? u.message : undefined,
    dismissed: u.dismissed,
    requested: u.requested,
    auto,
    testBuilds,
    checkedAt: u.checkedMs ?? undefined,
  }
}
