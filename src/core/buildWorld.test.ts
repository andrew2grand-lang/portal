import { listTitle, nearbyRows, sendListTitle, shownSessions, targets, viewFor } from '../engine/views'
import { describe, expect, it } from 'vitest'
import { appendLog, buildWorld, defaultUi, focusOf, toLogLines } from './buildWorld'
import type { CoreLogLine, CorePairing, CorePeer, CoreSnapshot, CoreTransfer } from './bridge'

const me = {
  id: 'a'.repeat(32),
  name: 'Studio Mac',
  os: 'macos' as const,
  port: 47920,
  proto: 3,
  receiveDir: '/Users/sam/Downloads/SeamDrop',
  skipJunk: true,
  autoUpdate: true,
  testBuilds: false,
  clipboardSync: false,
  clipboardPaused: null as number | null,
  sendTo: [] as string[],
  clipboardShortcut: null,
  appearance: 'system' as const,
  folderPerDevice: false, receiveMode: 'loose' as const, keepDates: false, receiveNote: null,
  pairingLocked: false,
  appVersion: '0.4.0',
  osVersion: 'macOS 15.1',
  addresses: [{ label: 'Wi-Fi', address: '192.168.1.24' }],
}

function peer(over: Partial<CorePeer> = {}): CorePeer {
  return {
    id: 'b'.repeat(32),
    name: 'Desk PC',
    os: 'windows',
    paired: false,
    online: false,
    link: 'wifi',
    addr: '192.168.1.31:47920',
    via: 'mdns',
    incoming: 'ask',
    accepts: null,
    incompatible: null,
    revoked: false,
    disconnected: false,
    reach: null,
    clip: null,
    ...over,
  }
}

const reach = (state: 'connecting' | 'unreachable') => ({
  state,
  sinceMs: 999_000,
  attempts: state === 'connecting' ? 1 : 6,
  lastFailure: state === 'connecting' ? null : 'No answer from 192.168.1.31',
  nextTryMs: null,
  step: 'trying_known' as const,
})

const snap = (peers: CorePeer[], searchMs = 0): CoreSnapshot => ({ me, peers, searchMs, pairing: null, transfers: [], doctor: [], clips: [], next: [], shelf: [], clipNotice: null })
const world = (s: CoreSnapshot) => buildWorld(s, [], defaultUi(), 1_000_000)
const status = (s: CoreSnapshot) => world(s).focus?.status

describe('one device: the screens as before', () => {
  it('searches while nothing is known', () => {
    expect(viewFor(world(snap([])))).toBe('searching')
  })
  it('offers to pair with an unpaired device it found', () => {
    expect(viewFor(world(snap([peer()])))).toBe('nearby')
  })
  it('is idle with a connected paired device', () => {
    expect(viewFor(world(snap([peer({ paired: true, online: true })])))).toBe('idle')
  })
  it('is disconnected, not offline, once Disconnect was pressed', () => {
    expect(viewFor(world(snap([peer({ paired: true, disconnected: true })])))).toBe('disconnected')
  })
  it('says connecting, never "isn’t reachable", while the core is still connecting', () => {
    expect(status(snap([peer({ paired: true, reach: reach('connecting') })]))).toBe('connecting')
    // No reach state from the core (yet): still not a guess of "isn't reachable"
    expect(status(snap([peer({ paired: true })]))).toBe('connecting')
    expect(viewFor(world(snap([peer({ paired: true })])))).toBe('connecting')
  })
  it('is offline only once the core says the paired device isn’t reachable', () => {
    expect(status(snap([peer({ paired: true, reach: reach('unreachable') })]))).toBe('unreachable')
    expect(viewFor(world(snap([peer({ paired: true, reach: reach('unreachable') })])))).toBe('offline')
  })
  it('names the computer that needs the update when versions can’t talk', () => {
    const older = world(snap([peer({ incompatible: 1 })]))
    expect(viewFor(older)).toBe('incompatible')
    expect(older.focus?.outdated).toBe('peer')
    expect(world(snap([peer({ incompatible: 4 })])).focus?.outdated).toBe('self')
  })
  it('says when the other side removed this device', () => {
    const w = world(snap([peer({ revoked: true })]))
    expect(viewFor(w)).toBe('nearby')
    expect(w.focus?.status).toBe('revoked')
  })
})

describe('devices are keyed by id; this computer is one of them', () => {
  it('two Macs can both be there', () => {
    const w = world(snap([peer({ os: 'macos', name: 'Air', paired: true, online: true }), peer({ id: 'c'.repeat(32), os: 'macos', name: 'iMac', paired: true })]))
    expect(w.me.name).toBe('Studio Mac')
    expect(w.peers.map((p) => [p.name, p.short])).toEqual([
      ['Air', 'Mac'],
      ['iMac', 'Mac'],
    ])
  })
  it('lists paired first (connected first), then nearby', () => {
    const w = world(
      snap([
        peer({ id: '1'.repeat(32), name: 'Stranger' }),
        peer({ id: '2'.repeat(32), name: 'Zed', paired: true }),
        peer({ id: '3'.repeat(32), name: 'Alpha', paired: true, online: true }),
      ]),
    )
    expect(w.peers.map((p) => p.name)).toEqual(['Alpha', 'Zed', 'Stranger'])
  })
  it('with several paired devices, the window is the drop zone with each device’s status', () => {
    const w = world(snap([peer({ paired: true, online: true }), peer({ id: 'c'.repeat(32), name: 'Laptop', paired: true, reach: reach('unreachable') })]))
    expect(viewFor(w)).toBe('idle')
    expect(w.peers.map((p) => p.status)).toEqual(['connected', 'unreachable'])
  })
  it('remembers where things go, and falls back to every paired device', () => {
    const two = [peer({ paired: true, online: true }), peer({ id: 'c'.repeat(32), name: 'Laptop', paired: true, online: true })]
    expect(world(snap(two)).target).toEqual(['b'.repeat(32), 'c'.repeat(32)])
    expect(world({ ...snap(two), me: { ...me, sendTo: ['c'.repeat(32)] } }).target).toEqual(['c'.repeat(32)])
    // A device that's gone from the list isn't a target any more
    expect(world({ ...snap(two), me: { ...me, sendTo: ['f'.repeat(32)] } }).target).toEqual(['b'.repeat(32), 'c'.repeat(32)])
  })
  it('focuses on the device being paired, else the only paired one', () => {
    const peers = world(snap([peer({ id: '1'.repeat(32), name: 'Nearby' }), peer({ name: 'Mine', paired: true })])).peers
    expect(focusOf(peers)?.name).toBe('Mine')
    expect(focusOf(peers, '1'.repeat(32))?.name).toBe('Nearby')
  })
  it('turns core log lines into Details lines with local times', () => {
    const line: CoreLogLine = { atMs: Date.UTC(2026, 0, 1, 12, 0, 5), tag: 'found', text: 'Found Desk PC on this network', detail: null, tone: null }
    const w = buildWorld(snap([]), appendLog([], line, 300), defaultUi(), 0)
    expect(w.logs).toHaveLength(1)
    expect(w.logs[0].time).toMatch(/^\d\d:\d\d:\d\d$/)
  })
  it('dates the search start from the snapshot', () => {
    expect(world(snap([], 4_000)).searchStartedAt).toBe(996_000)
  })
})

describe('pairing by typed code', () => {
  const pairing: CorePairing = { peerId: 'c'.repeat(32), peerName: 'Other', peerAddr: '192.168.1.20', mine: false, step: 'code', code: '048213', triesLeft: 3, failed: null, leftMs: 100_000 }
  it('shows the device being paired, where it asks from, and the code in two groups', () => {
    const w = world({ ...snap([peer(), peer({ id: 'c'.repeat(32), name: 'Other' })]), pairing })
    expect(viewFor(w)).toBe('pairing')
    expect(w.focus?.name).toBe('Other')
    // Review 04 (privacy): when the code stops working, as a clock time to count down to
    expect(w.pairing).toEqual({ peer: 'c'.repeat(32), peerAddr: '192.168.1.20', mine: false, step: 'code', code: '048 213', triesLeft: 3, failed: undefined, expiresAt: 1_100_000 })
  })
  it('has no digits where the code is typed', () => {
    const w = world({ ...snap([peer()]), pairing: { ...pairing, peerId: 'b'.repeat(32), mine: true, code: null } })
    expect(w.pairing?.code).toBe('')
  })
  it('says when pairing is locked', () => {
    expect(world({ ...snap([]), me: { ...me, pairingLocked: true } }).settings.pairingLocked).toBe(true)
  })
})

const transfer = (over: Partial<CoreTransfer> = {}): CoreTransfer => ({
  stoppedHere: false,
  waitingThere: false,
  percentKnown: true,
  id: '00000000000000ff',
  direction: 'out',
  peerId: 'b'.repeat(32),
  peerName: 'Desk PC',
  phase: 'running',
  block: null,
  items: [
    { name: 'Photos', folder: true, bytes: 900, files: 12 },
    { name: 'a.pdf', folder: false, bytes: 100, files: 1 },
  ],
  totalBytes: 1000,
  fileCount: 13,
  autoAccepted: false,
  bytes: 250,
  elapsedMs: null,
  link: 'wifi',
  stalled: false,
  result: null,
  failure: null,
  saveTo: null,
  pausedSinceMs: null,
  resuming: false,
  canSendAgain: false,
  ...over,
})

describe('transfers', () => {
  const online = [peer({ paired: true, online: true })]
  const w = (...ts: CoreTransfer[]) => world({ ...snap(online), transfers: ts })

  it('one transfer has its own screen; several are one row each', () => {
    expect(viewFor(w(transfer()))).toBe('session')
    expect(viewFor(w(transfer(), transfer({ id: '00000000000000fe', peerId: 'c'.repeat(32) })))).toBe('sessions')
  })
  // Review 04 (four computers, blocks use): more files while transfers run
  describe('sending more while transfers run', () => {
    const two = [peer({ paired: true, online: true }), peer({ id: 'c'.repeat(32), name: 'Laptop', paired: true, online: true })]
    const queued = { ...defaultUi(), queue: [{ path: '/u/plan.pdf', name: 'plan.pdf', kind: 'file' as const, bytes: 10 }] }
    const busy = (ui = queued, ...ts: CoreTransfer[]) => buildWorld({ ...snap(two), transfers: ts }, [], ui, 1_000_000)
    it('shows the send list instead of hiding it behind the transfers', () => {
      expect(viewFor(busy(queued, transfer()))).toBe('idle')
      expect(viewFor(busy(queued, transfer(), transfer({ id: '00000000000000fe', peerId: 'c'.repeat(32) })))).toBe('idle')
    })
    it('but an offer waiting for an answer comes first', () => {
      expect(viewFor(busy(queued, transfer({ direction: 'in', phase: 'offered' })))).toBe('session')
    })
    // Mission 05 §1.5: "Ready to send" only when something can send now
    it('never says Ready to send when every ticked computer is busy', () => {
      const one = (ui = queued, ...ts: CoreTransfer[]) => buildWorld({ ...snap([two[0]]), transfers: ts }, [], ui, 1_000_000)
      expect(sendListTitle(one(queued, transfer()))).toBe('Busy sending to Desk PC.')
      expect(sendListTitle(one(queued, transfer({ direction: 'in' })))).toBe('Busy receiving from Desk PC.')
      expect(sendListTitle(one(queued, transfer({ phase: 'paused' })))).toBe('Busy: a paused transfer with Desk PC.')
      expect(sendListTitle(one(queued))).toBe('Ready to send')
      // One free, one busy: it can send
      expect(sendListTitle(busy(queued, transfer()))).toBe('Ready to send')
    })
    it('a device busy with a transfer isn’t where the next files go', () => {
      const t = targets(busy(queued, transfer({ phase: 'paused' })))
      expect(t.to.map((p) => p.name)).toEqual(['Laptop'])
      expect(t.busy.map((p) => p.name)).toEqual(['Desk PC'])
      // Once it's done, it is again
      expect(targets(busy(queued, transfer({ phase: 'done' }))).to.map((p) => p.name)).toEqual(['Desk PC', 'Laptop'])
    })
  })
  // Review 04 (developer, four computers): the heading counts what's on screen
  it('the list’s heading says what each transfer is doing', () => {
    const t = (over: Partial<CoreTransfer>, id: string) => transfer({ id, peerId: id.repeat(2).padEnd(32, '0'), ...over })
    const list = (...ts: CoreTransfer[]) => listTitle(shownSessions(w(...ts)))
    expect(list(t({}, 'a1'), t({ direction: 'in' }, 'a2'), t({ phase: 'paused' }, 'a3'), t({ phase: 'done' }, 'a4'))).toBe('2 moving · 1 paused · 1 done')
    expect(list(t({ direction: 'in', phase: 'offered' }, 'b1'), t({ direction: 'in', phase: 'offered' }, 'b2'))).toBe('2 waiting for you')
    expect(list(t({ phase: 'offered' }, 'c1'), t({ phase: 'failed' }, 'c2'))).toBe('1 waiting to be accepted · 1 stopped')
  })
  it('a refused or declined transfer is only news to the sender', () => {
    expect(shownSessions(w(transfer({ direction: 'in', phase: 'declined' })))).toEqual([])
    expect(shownSessions(w(transfer({ phase: 'declined' })))).toHaveLength(1)
  })
  it('maps a transfer with its device, direction and items', () => {
    const s = w(transfer()).sessions[0]
    expect(s.peer).toBe('b'.repeat(32))
    expect(s.direction).toBe('out')
    expect(s.phase).toBe('transfer')
    expect(s.payload.items).toEqual([
      { name: 'Photos', kind: 'folder', bytes: 900, count: 12 },
      { name: 'a.pdf', kind: 'file', bytes: 100, count: undefined },
    ])
  })
  it('shows a cancelled send as stopped by this computer, ready to send again', () => {
    const s = w(transfer({ phase: 'cancelled', failure: 'You cancelled it.', canSendAgain: true })).sessions[0]
    expect(s.phase).toBe('failed')
    expect(s.cancelled).toBe(true)
    expect(s.canSendAgain).toBe(true)
    expect(w(transfer({ phase: 'failed' })).sessions[0].cancelled).toBe(false)
  })
  it('shows a paused transfer with what is kept', () => {
    const s = w(transfer({ phase: 'paused', bytes: 400, pausedSinceMs: 5, resuming: true })).sessions[0]
    expect(s.phase).toBe('paused')
    expect(s.pausedSince).toBe(5)
    expect(s.resuming).toBe(true)
    expect(s.startProgress).toBeCloseTo(0.4)
  })
  it('maps a refused send to the blocked state, and carries results and failures', () => {
    expect(w(transfer({ phase: 'blocked', block: 'not_accepted' })).sessions[0].block).toBe('not-accepted')
    const skippedItems = [{ path: 'Docs/link.txt', why: 'a link (shortcut): links aren’t sent' }]
    const renamedItems = [{ path: 'Docs/a:b.txt', why: 'saved as “a_b.txt”: ‘:’ isn’t allowed in Windows names' }]
    const result = { savedTo: '/d', reveal: '/d/Photos', files: 13, bytes: 1000, renamed: 1, skipped: 1, longPaths: 0, keptBytes: 400, skippedItems, renamedItems, notes: ['1 name changed'], firstArrival: false }
    const done = w(transfer({ phase: 'done', elapsedMs: 4000, result })).sessions[0]
    expect(done.elapsed).toBe(4)
    expect(done.result?.revealPath).toBe('/d/Photos')
    // Review 04: the done screen can name which items, and leaves kept bytes out of the speed
    expect(done.result?.skippedItems).toEqual(skippedItems)
    expect(done.result?.renamedItems).toEqual(renamedItems)
    expect(done.result?.keptBytes).toBe(400)
    expect(w(transfer({ phase: 'failed', failure: 'Lost the connection' })).sessions[0].failure).toBe('Lost the connection')
  })
  it('says the connection dropped out while nothing moves', () => {
    expect(w(transfer()).sessions[0].linkNote).toBeNull()
    expect(w(transfer({ stalled: true })).sessions[0].linkNote).toBe('stalled')
  })
})

describe('doctor', () => {
  it('falls back to general suggestions before the core has any', () => {
    expect(world(snap([])).doctor.length).toBeGreaterThan(0)
  })
})

describe('clipboard sync, per device', () => {
  const clip = (over: Partial<NonNullable<CorePeer['clip']>>) => ({ mine: false, theirs: false, asks: false, to: 'clipboard' as const, ...over })
  it('carries each device’s side and the pause', () => {
    const w = world({
      ...snap([peer({ paired: true, online: true, clip: clip({ mine: true, theirs: true }) })]),
      me: { ...me, clipboardSync: true, clipboardPaused: 0 },
    })
    expect(w.peers[0].clip).toEqual(clip({ mine: true, theirs: true }))
    expect(w.clipboard).toMatchObject({ sync: true, paused: 0 })
  })
})

describe('Details: a calm story (Mission 04 §1.6)', () => {
  const at = (h: number, m: number, s = 0) => new Date(2026, 10, 15, h, m, s).getTime()
  const sent = (ms: number, size: string): CoreLogLine => ({ atMs: ms, tag: 'clipboard', text: 'Clipboard sent to Desk PC', detail: `text, ${size}`, tone: null })
  const found: CoreLogLine = { atMs: at(19, 0), tag: 'found', text: 'Found Desk PC on this network', detail: '192.168.1.20:47920 via mDNS', tone: null }

  it('collapses identical lines in a row: ×12, last at 19:31', () => {
    let runs = appendLog([], found, 300)
    for (let i = 0; i < 12; i++) runs = appendLog(runs, sent(at(19, 20 + i), `${i + 1}.0 KB`), 300)
    const lines = toLogLines(runs)
    expect(lines).toHaveLength(2)
    expect(lines[1]).toMatchObject({ text: 'Clipboard sent to Desk PC', count: 12, time: '19:20:00', last: '19:31', detail: 'text, 12.0 KB' })
    expect(lines[0]).toMatchObject({ count: 1, last: undefined })
  })

  it('keeps a line that comes back after another one apart', () => {
    let runs = appendLog([], sent(at(19, 1), '1 KB'), 300)
    runs = appendLog(runs, found, 300)
    runs = appendLog(runs, sent(at(19, 2), '1 KB'), 300)
    expect(toLogLines(runs).map((l) => l.count)).toEqual([1, 1, 1])
  })

  it('a different tone is a different line', () => {
    const warn: CoreLogLine = { ...sent(at(19, 3), '1 KB'), tone: 'warn' }
    expect(toLogLines(appendLog(appendLog([], sent(at(19, 2), '1 KB'), 300), warn, 300))).toHaveLength(2)
  })

  it('repeats take one place, so a burst never pushes the story out', () => {
    let runs = appendLog([], found, 3)
    for (let i = 0; i < 500; i++) runs = appendLog(runs, sent(at(19, 5) + i, '1 KB'), 3)
    expect(toLogLines(runs).map((l) => l.text)).toEqual(['Found Desk PC on this network', 'Clipboard sent to Desk PC'])
    runs = appendLog(runs, { ...found, atMs: at(19, 9) }, 3)
    runs = appendLog(runs, sent(at(19, 10), '1 KB'), 3)
    expect(toLogLines(runs)).toHaveLength(3)
  })
})


describe('appearance (Mission 04 §1.7)', () => {
  it('is System until the core says otherwise, then what was chosen', () => {
    expect(buildWorld(null, [], defaultUi(), 0).appearance).toBe('system')
    expect(world(snap([])).appearance).toBe('system')
    expect(world({ ...snap([]), me: { ...me, appearance: 'dark' } }).appearance).toBe('dark')
  })
})

// Mission 05 §1.8: Sam's Air saw only the PC ("Den_PC" sorts first)
describe('the nearby list', () => {
  const three = [
    peer({ id: 'a'.repeat(32), name: 'Den_PC', paired: false, online: false }),
    peer({ id: 'b'.repeat(32), name: 'macbook one', os: 'macos', paired: false, online: false }),
    peer({ id: 'c'.repeat(32), name: 'Kitchen Mac mini', os: 'macos', paired: false, online: false }),
  ]
  it('lists every nearby computer when nothing is paired, and picks none', () => {
    const w = world(snap(three))
    expect(viewFor(w)).toBe('nearby')
    expect(nearbyRows(w).map((r) => r.peer.name).sort()).toEqual(['Den_PC', 'Kitchen Mac mini', 'macbook one'])
    expect(nearbyRows(w).every((r) => !r.paired)).toBe(true)
  })
  it('marks one that was just paired, so the next can follow', () => {
    const w = world(snap([{ ...three[0], paired: true, online: true }, three[1], three[2]]))
    const rows = nearbyRows(w)
    expect(rows.find((r) => r.peer.name === 'Den_PC')?.paired).toBe(true)
    expect(rows.filter((r) => !r.paired).length).toBe(2)
  })
})
