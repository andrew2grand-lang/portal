/* A fake Rust core for working on the real app's screens in a browser
   (`npm run dev`, then /?mock=<scenario>&os=mac|win[&devices=6]). Dev builds
   only. Speaks the same snapshot and commands as src-tauri/src/bridge.rs. */
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import type { CoreClip, CoreEta, CoreEvent, CoreLogLine, CorePairAction, CorePairing, CorePeer, CoreResult, CoreSnapshot, CoreTransfer, CoreUpdate, SettingsPatch } from '../core/bridge'
import { scenarioList } from './scenarioList'
import type { Scenario } from './scenarioList'

export { scenarioList as scenarios }

type Os = 'mac' | 'win'

/** The device every single-device scenario is about */
function mainPeer(over: Partial<CorePeer>, os: Os): CorePeer {
  const peerIsMac = os === 'win'
  return {
    id: 'b'.repeat(32),
    name: peerIsMac ? 'MacBook Pro' : 'Desk PC',
    os: peerIsMac ? 'macos' : 'windows',
    paired: false,
    online: false,
    link: 'wifi',
    addr: peerIsMac ? '192.168.1.24:47920' : '192.168.1.31:47920',
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

const pairedClip = { mine: false, theirs: false, asks: false, to: 'clipboard' as const }

/** More paired devices for `&devices=N`: a household of computers */
function extraPeers(n: number): CorePeer[] {
  const list: [string, CorePeer['os'], CorePeer['link'], boolean, boolean][] = [
    ['Studio iMac', 'macos', 'ethernet', true, false],
    ['MacBook Air', 'macos', 'wifi', true, false],
    ['Old Laptop', 'windows', null, false, true],
    ['Office PC', 'windows', 'ethernet', true, false],
    ['Kitchen Mac mini', 'macos', null, false, false],
  ]
  return list.slice(0, Math.max(0, n)).map(([name, os, link, online, unreachable], i) => ({
    ...mainPeer({}, 'mac'),
    id: `${i + 1}`.repeat(32).slice(0, 32),
    name,
    os,
    link,
    online,
    paired: true,
    addr: `192.168.1.${40 + i}:47920`,
    reach: online
      ? null
      : unreachable
        ? { state: 'unreachable', sinceMs: Date.now() - 60_000, attempts: 30, lastFailure: `No answer from 192.168.1.${40 + i}`, nextTryMs: null, step: 'trying_known' }
        : { state: 'connecting', sinceMs: Date.now() - 3000, attempts: 3, lastFailure: null, nextTryMs: null, step: 'looking' },
    clip: pairedClip,
  }))
}

const ITEMS = [
  { name: 'Site photos', folder: true, bytes: 611_400_000, files: 212 },
  { name: 'Floor plan.dwg', folder: false, bytes: 18_700_000, files: 1 },
  { name: 'Résumé_ Q3_.pdf', folder: false, bytes: 2_400_000, files: 1 },
]

function baseTransfer(os: Os, over: Partial<CoreTransfer> = {}): CoreTransfer {
  return {
    id: '0000000000a1b2c3',
    direction: 'out',
    peerId: 'b'.repeat(32),
    peerName: os === 'win' ? 'MacBook Pro' : 'Desk PC',
    phase: 'running',
    block: null,
    items: ITEMS,
    totalBytes: 632_500_000,
    fileCount: 214,
    autoAccepted: false,
    bytes: 0,
    elapsedMs: null,
    link: 'wifi',
    stalled: false,
    result: null,
    failure: null,
    saveTo: null,
    pausedSinceMs: null,
    resuming: false,
    stoppedHere: false,
    waitingThere: false,
    percentKnown: true,
    canSendAgain: false,
    ...over,
  }
}

/** "Desk PC and MacBook Air" (not engine/words: that loads platform.ts,
    which must only load once the mock has set up the Tauri globals) */
const names = (list: string[]) => (list.length > 1 ? `${list.slice(0, -1).join(', ')} and ${list.at(-1)}` : (list[0] ?? ''))

/** How long a pairing step lasts in the core */
const PAIR_TIMEOUT = 120_000

/** The file moving at `bytes`: the item there, or a photo inside a folder */
function fileAt(t: CoreTransfer, bytes: number): string {
  let start = 0
  for (const item of t.items) {
    if (bytes < start + item.bytes || item === t.items.at(-1)) {
      if (!item.folder) return item.name
      const each = item.bytes / Math.max(1, item.files)
      const n = Math.min(item.files - 1, Math.floor((bytes - start) / each))
      return `${item.name}/IMG_${String(400 + n).padStart(4, '0')}.HEIC`
    }
    start += item.bytes
  }
  return ''
}

function transfersFor(s: Scenario, os: Os): CoreTransfer[] {
  const peerName = os === 'win' ? 'MacBook Pro' : 'Desk PC'
  const base = baseTransfer(os)
  // Names change only for a Windows receiver (macOS takes what Windows allows)
  const renamedItems = [{ path: 'Résumé: Q3?.pdf', why: 'saved as “Résumé_ Q3_.pdf”: ‘:’ and ‘?’ aren’t allowed in Windows names' }]
  const skippedItems = [
    { path: 'Site photos/latest.jpg', why: 'a link (shortcut): links aren’t sent' },
    { path: 'Site photos/.DS_Store', why: 'a system file the other computer makes for itself' },
  ]
  const doneResult = {
    savedTo: os === 'mac' ? '/Users/sam/Downloads/SeamDrop' : 'C:\\Users\\Sam\\Downloads\\SeamDrop',
    reveal: os === 'mac' ? '/Users/sam/Downloads/SeamDrop/Site photos' : 'C:\\Users\\Sam\\Downloads\\SeamDrop\\Site photos',
    files: 214,
    bytes: 632_500_000,
    renamed: os === 'win' ? 1 : 0,
    skipped: 2,
    longPaths: 0,
    keptBytes: 0,
    skippedItems,
    renamedItems: os === 'win' ? renamedItems : [],
    notes: [
      ...(os === 'win' ? ['1 name changed so it works on Windows.'] : []),
      `2 items left out on ${peerName}: 1 link and 1 system file.`,
    ],
    // `&first=1`: the first files this computer ever received
    firstArrival: new URLSearchParams(window.location.search).get('first') === '1',
  }
  const me = os === 'mac' ? 'this Mac' : 'this PC'
  switch (s) {
    case 'offer-in':
    case 'instant':
      return [{ ...base, direction: 'in', phase: 'offered' }]
    case 'offer-out':
      return [{ ...base, phase: 'offered' }]
    case 'sending':
      return [{ ...base, bytes: 280_000_000 }]
    case 'receiving':
      return [{ ...base, direction: 'in', bytes: 280_000_000 }]
    case 'stalled':
      return [{ ...base, bytes: 280_000_000, stalled: true }]
    case 'paused-out':
      return [{ ...base, phase: 'paused', bytes: 280_000_000, pausedSinceMs: Date.now() - 40_000 }]
    // Mission 05 §1.4: the app was quit partway here, so it asks
    case 'stopped-out':
      return [{ ...base, phase: 'paused', bytes: 280_000_000, pausedSinceMs: Date.now() - 40_000, stoppedHere: true }]
    case 'stopped-in':
      return [{ ...base, direction: 'in', phase: 'paused', bytes: 280_000_000, pausedSinceMs: Date.now() - 40_000, stoppedHere: true }]
    // …and the other computer waits for it
    case 'waiting-there':
      return [{ ...base, direction: 'in', phase: 'paused', bytes: 280_000_000, pausedSinceMs: Date.now() - 40_000, waitingThere: true }]
    case 'paused-in':
      return [{ ...base, direction: 'in', phase: 'paused', bytes: 280_000_000, pausedSinceMs: Date.now() - 40_000 }]
    case 'resuming':
      return [{ ...base, direction: 'in', phase: 'paused', bytes: 280_000_000, pausedSinceMs: Date.now() - 40_000, resuming: true }]
    case 'done-in':
      return [{ ...base, direction: 'in', phase: 'done', bytes: base.totalBytes, elapsedMs: 7_400, result: doneResult }]
    case 'done-out':
      return [
        {
          ...base,
          phase: 'done',
          bytes: base.totalBytes,
          elapsedMs: 7_400,
          result: {
            ...doneResult,
            savedTo: os === 'mac' ? 'C:\\Users\\Sam\\Downloads\\SeamDrop' : '/Users/sam/Downloads/SeamDrop',
            reveal: null,
            keptBytes: 280_000_000,
            renamed: os === 'mac' ? 1 : 0,
            renamedItems: os === 'mac' ? renamedItems : [],
            notes: [
              `Continued after the connection dropped: the 280 MB already on ${peerName} wasn’t sent again.`,
              ...(os === 'mac' ? ['1 name changed so it works on Windows.'] : []),
              '2 items left out: 1 link and 1 system file.',
            ],
          },
        },
      ]
    case 'failed':
      return [
        {
          ...base,
          phase: 'failed',
          bytes: 120_000_000,
          canSendAgain: true,
          failure: `${peerName} ran out of space. Your files are still on ${me}; nothing was saved on ${peerName}.`,
        },
      ]
    case 'cancelled':
      return [{ ...base, phase: 'cancelled', bytes: 280_000_000, canSendAgain: true, failure: `You cancelled it. Your files are still on ${me}; nothing was saved on ${peerName}.` }]
    case 'cancelled-in':
      return [{ ...base, direction: 'in', phase: 'cancelled', bytes: 280_000_000, failure: `You cancelled it. What had arrived from ${peerName} was removed.` }]
    case 'blocked-peer':
      return [{ ...base, phase: 'blocked', block: 'not_accepted' }]
    case 'declined':
      return [{ ...base, phase: 'declined' }]
    // Several devices: side by side, one each
    case 'many-sending':
      return [
        { ...base, bytes: 280_000_000 },
        { ...base, id: '0000000000a1b2c4', peerId: '1'.repeat(32), peerName: 'Studio iMac', bytes: 420_000_000, link: 'ethernet' },
      ]
    case 'many-mixed':
      return [
        { ...base, bytes: 280_000_000 },
        {
          ...base,
          id: '0000000000a1b2c5',
          direction: 'in',
          peerId: '2'.repeat(32),
          peerName: 'MacBook Air',
          items: [{ name: 'Holiday video.mov', folder: false, bytes: 4_200_000_000, files: 1 }],
          totalBytes: 4_200_000_000,
          fileCount: 1,
          bytes: 1_300_000_000,
        },
        { ...base, id: '0000000000a1b2c6', peerId: '3'.repeat(32), peerName: 'Old Laptop', phase: 'paused', bytes: 120_000_000, pausedSinceMs: Date.now() - 300_000 },
        {
          ...base,
          id: '0000000000a1b2c7',
          peerId: '4'.repeat(32),
          peerName: 'Office PC',
          phase: 'done',
          bytes: base.totalBytes,
          elapsedMs: 9_100,
          result: { ...doneResult, savedTo: 'C:\\Users\\Sam\\Downloads\\SeamDrop', reveal: null, notes: [] },
        },
      ]
    case 'many-offers':
      return [
        { ...base, direction: 'in', phase: 'offered' },
        { ...base, id: '0000000000a1b2c8', direction: 'in', phase: 'offered', peerId: '2'.repeat(32), peerName: 'MacBook Air', items: [ITEMS[1]], totalBytes: ITEMS[1].bytes, fileCount: 1 },
      ]
    default:
      return []
  }
}

/** What the real core writes to Details for each transfer state */
function transferLogs(s: Scenario, os: Os): CoreLogLine[] {
  const peerName = os === 'win' ? 'MacBook Pro' : 'Desk PC'
  const at = (ago: number) => Date.now() - ago
  const line = (ago: number, tag: string, text: string, detail: string | null = null, tone: CoreLogLine['tone'] = null): CoreLogLine => ({
    atMs: at(ago),
    tag,
    text,
    detail,
    tone,
  })
  const out = !['receiving', 'offer-in', 'instant', 'paused-in', 'stopped-in', 'waiting-there', 'resuming', 'done-in', 'cancelled-in'].includes(s)
  const offer = out ? line(9000, 'offer', `Offered 3 items to ${peerName}`, '632.5 MB') : line(9000, 'offer', `${peerName} offers 3 items`, '632.5 MB')
  const accepted = out ? line(8000, 'accept', `${peerName} accepted; sending`, '1 MiB chunks') : line(8000, 'accept', 'Accepted here')
  const pause = line(6000, 'pause', `Lost the connection to ${peerName} at 44 %; it continues when ${peerName} is back`, '280 MB of 632.5 MB', 'warn')
  switch (s) {
    case 'sending':
    case 'receiving':
    case 'many-sending':
      return [offer, accepted]
    case 'stalled':
      return [offer, accepted, line(3000, 'stall', `No data from ${peerName} for a few seconds; waiting for the connection`, null, 'warn')]
    case 'paused-out':
    case 'paused-in':
      return [offer, accepted, pause]
    case 'many-mixed':
      return [
        offer,
        accepted,
        line(7000, 'offer', 'MacBook Air offers Holiday video.mov', '4.2 GB'),
        line(6900, 'accept', 'Accepted here'),
        line(6000, 'pause', 'Lost the connection to Old Laptop at 18 %; it continues when Old Laptop is back', '120 MB of 632.5 MB', 'warn'),
        line(3000, 'sent', 'Sent 214 files (632.5 MB) to Office PC', 'in 9.1 s, 70 MB/s'),
      ]
    case 'resuming':
      return [offer, accepted, pause, line(1000, 'resume', `${peerName} is back; asking what already arrived`)]
    case 'done-in':
      return [
        offer,
        accepted,
        ...(os === 'win' ? [line(1500, 'rename', 'Renamed Résumé: Q3?.pdf to Résumé_ Q3_.pdf', '‘:’ and ‘?’ aren’t allowed in Windows names')] : []),
        line(1400, 'skip', `${peerName} left out Site photos/latest.jpg`, 'link (shortcut), skipped', 'dim'),
        line(1300, 'skip', `${peerName} left out Site photos/.DS_Store`, 'system file, skipped', 'dim'),
        line(1000, 'verify', 'Every file checked ✓', 'BLAKE3, 214 files'),
        line(900, 'saved', 'Saved 3 items in Downloads/SeamDrop', `632.5 MB, ${os === 'mac' ? '/Users/sam/Downloads/SeamDrop' : 'C:\\Users\\Sam\\Downloads\\SeamDrop'}`),
      ]
    case 'done-out':
      return [
        offer,
        accepted,
        pause,
        line(4000, 'resume', 'Resumed at 280 MB (nothing sent twice)', `${peerName} has 280 MB of 632.5 MB, continuing from 44 % (file 3 of 214)`),
        ...(os === 'mac' ? [line(1100, 'rename', `Renamed on ${peerName}: Résumé: Q3?.pdf to Résumé_ Q3_.pdf`, '‘:’ and ‘?’ aren’t allowed in Windows names')] : []),
        line(1000, 'verify', `Every file checked on ${peerName} ✓`, 'BLAKE3'),
        line(900, 'sent', `Sent 214 files (632.5 MB) to ${peerName}`, '352.5 MB in 5.5 s, 64.1 MB/s; 280 MB was already there'),
      ]
    case 'failed':
      return [offer, accepted, line(1000, 'stop', `${peerName} ran out of space, so the transfer stopped at 61 %. Free up space there and send it again.`, 'disk_full', 'err')]
    case 'cancelled':
      return [offer, accepted, line(1000, 'cancel', `Cancelled; the partial files on ${peerName} were removed`)]
    case 'cancelled-in':
      return [offer, accepted, line(1000, 'cancel', 'Cancelled; the partial files were removed')]
    default:
      return []
  }
}

const connected = (over: Partial<CorePeer> = {}): Partial<CorePeer> => ({ paired: true, online: true, clip: pairedClip, ...over })

/** `&devices=N` on the nearby scenario: N computers nearby */
const nearbyCount = () => Number(new URLSearchParams(window.location.search).get('devices') ?? '1') || 1

function peersFor(s: Scenario, os: Os): CorePeer[] {
  const p = (over: Partial<CorePeer>) => mainPeer(over, os)
  switch (s) {
    case 'searching':
    case 'searching-long':
      return []
    // Every computer nearby, none paired (Mission 05 §1.8): `&devices=` adds more
    case 'nearby':
      return [p({}), ...extraPeers(nearbyCount() - 1).map((x) => ({ ...x, paired: false, online: false, link: null, clip: null, reach: null }))]
    case 'revoked':
      return [p({ revoked: true, online: true })]
    case 'pairing-incoming':
    case 'pairing-show':
    case 'pairing-wrong-there':
    case 'pairing-type':
    case 'pairing-wrong-here':
    case 'pairing-paired':
      return [p({ online: true })]
    case 'pairing-locked':
      return [p({})]
    case 'online-ethernet':
      // The PC on Ethernet through the router, the Mac on its Wi-Fi
      return [p(connected({ link: os === 'win' ? 'ethernet' : 'wifi' }))]
    case 'connecting':
      return [p({ paired: true, link: null, via: null, clip: pairedClip, reach: { state: 'connecting', sinceMs: Date.now() - 4000, attempts: 4, lastFailure: null, nextTryMs: null, step: 'trying_known' } })]
    case 'offline':
      return [
        p({
          paired: true,
          link: null,
          via: null,
          clip: pairedClip,
          reach: { state: 'unreachable', sinceMs: Date.now() - 45_000, attempts: 23, lastFailure: `No answer from ${os === 'win' ? '192.168.1.24' : '192.168.1.31'}`, nextTryMs: null, step: 'trying_known' },
        }),
      ]
    case 'disconnected':
      return [p({ paired: true, link: null, via: null, disconnected: true, clip: pairedClip })]
    // Usually a paired computer that hasn't been updated yet
    case 'incompatible':
      return [p({ incompatible: 1, paired: true })]
    case 'incompatible-self':
      return [p({ incompatible: 4, paired: true })]
    case 'blocked-peer':
      return [p(connected({ accepts: 'never' }))]
    case 'add-device':
      return [p({}), ...extraPeers(2)]
    // A transfer pauses because the other computer went away: as the core
    // shows it, that computer isn't reachable while it's paused
    case 'paused-out':
    case 'paused-in':
      return [
        p({
          paired: true,
          link: null,
          via: null,
          clip: pairedClip,
          reach: { state: 'unreachable', sinceMs: Date.now() - 40_000, attempts: 9, lastFailure: `No answer from ${os === 'win' ? '192.168.1.24' : '192.168.1.31'}`, nextTryMs: null, step: 'trying_known' },
        }),
      ]
    default:
      return [p(connected())]
  }
}

function initial(s: Scenario, os: Os, devices: number): CoreSnapshot {
  const me = {
    id: 'a'.repeat(32),
    name: os === 'mac' ? 'MacBook Pro' : 'Desk PC',
    os: os === 'mac' ? ('macos' as const) : ('windows' as const),
    port: 47920,
    proto: 3,
    receiveDir: os === 'mac' ? '/Users/sam/Downloads/SeamDrop' : 'C:\\Users\\Sam\\Downloads\\SeamDrop',
    skipJunk: true,
    autoUpdate: true,
    testBuilds: false,
    clipboardSync: false,
    clipboardPaused: null as number | null,
    sendTo: [] as string[],
    clipboardShortcut: null,
    appearance: (new URLSearchParams(window.location.search).get('appearance') ?? 'system') as CoreSnapshot['me']['appearance'],
    folderPerDevice: false, receiveMode: 'loose' as const, keepDates: false, receiveNote: null,
    pairingLocked: s === 'pairing-locked',
    pairingLockedBy: s === 'pairing-locked' ? `Stranger’s laptop (192.168.1.57)` : null,
    pairingLockedAtMs: s === 'pairing-locked' ? Date.now() - 6 * 60_000 : null,
    appVersion: '0.4.0',
    osVersion: os === 'mac' ? 'macOS 15.1 (24B83)' : 'Windows 11 24H2 (build 26100)',
    addresses: [{ label: 'Wi-Fi', address: os === 'mac' ? '192.168.1.24' : '192.168.1.31' }],
  }
  let peers = peersFor(s, os)
  // `&devices=N` (or a many-* scenario): N paired devices in all
  const wanted = s.startsWith('many-') ? Math.max(devices, 6) : devices
  const pairedNow = peers.filter((p) => p.paired).length
  if (wanted > 1 && pairedNow > 0) peers = [...peers, ...extraPeers(wanted - pairedNow)]
  const pairStates: Partial<Record<Scenario, Pick<CorePairing, 'mine' | 'step' | 'code' | 'triesLeft'>>> = {
    'pairing-incoming': { mine: false, step: 'asked', code: null, triesLeft: 3 },
    'pairing-show': { mine: false, step: 'code', code: '048213', triesLeft: 3 },
    'pairing-wrong-there': { mine: false, step: 'wrong', code: '048213', triesLeft: 2 },
    'pairing-type': { mine: true, step: 'code', code: null, triesLeft: 3 },
    'pairing-wrong-here': { mine: true, step: 'wrong', code: null, triesLeft: 1 },
    'pairing-paired': { mine: true, step: 'paired', code: null, triesLeft: 3 },
  }
  const ps = pairStates[s]
  const pairing: CorePairing | null = ps
    ? { peerId: 'b'.repeat(32), peerName: peers[0].name, peerAddr: os === 'mac' ? '192.168.1.31' : '192.168.1.24', failed: null, leftMs: PAIR_TIMEOUT, ...ps }
    : null
  const live = peers.filter((p) => p.paired && p.online)
  const online = live.map((p) => p.name)
  const doctor: CoreSnapshot['doctor'] =
    s === 'searching-long' || s === 'offline'
      ? os === 'mac'
        ? [
            ...(s === 'offline'
              ? [{ id: 'local-network', status: 'pass' as const, title: 'Local network access is on', fix: null, action: null }]
              : [
                  { id: 'local-network', status: 'likely' as const, title: 'SeamDrop may need Local Network access on this Mac', fix: 'Turn SeamDrop on in System Settings → Privacy & Security → Local Network. Until then, on the PC, use Connect by IP and type this Mac’s address: 192.168.1.24.', action: { label: 'Open Settings', url: 'x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork' } },
                  { id: 'isolation', status: 'likely' as const, title: 'Your network may be keeping devices apart', fix: 'Guest, school and office Wi-Fi often do. Use your home network, or Connect by IP.', action: null },
                ]),
            { id: 'peer-open', status: 'unknown', title: 'Is SeamDrop open on the other computer?', fix: 'Open SeamDrop there too. It keeps running from the menu bar or system tray.', action: null },
            { id: 'network', status: 'pass', title: 'On Wi-Fi', fix: null, action: null },
            { id: 'vpn', status: 'pass', title: 'No VPN in the way', fix: null, action: null },
          ]
        : [
            { id: 'firewall-block', status: 'fail', title: 'A Windows Firewall rule blocks SeamDrop', fix: 'Usually left by pressing Cancel on a Windows Firewall question about SeamDrop. Fix it removes the block (SeamDrop) and allows SeamDrop again; Windows asks for an administrator first.', action: { label: 'Fix it', url: 'seamdrop:fix-firewall' } },
            { id: 'profile', status: 'pass', title: '“Home Wi-Fi” is a Public network, and SeamDrop is allowed there', fix: 'If the Mac still can’t reach this PC, the PC connects out to the Mac instead.', action: null },
            { id: 'peer-open', status: 'unknown', title: 'Is SeamDrop open on the other computer?', fix: 'Open SeamDrop there too. It keeps running from the menu bar or system tray.', action: null },
            { id: 'network', status: 'pass', title: 'On Wi-Fi', fix: null, action: null },
          ]
      : // What the core says while paired devices are connected: that first
        [
          ...(online.length ? [{ id: 'connected', status: 'pass' as const, title: `Connected to ${names(online)} right now`, fix: 'So this network and this computer’s settings work for SeamDrop.', action: null }] : []),
          { id: 'network', status: 'pass', title: live.some((p) => p.link === 'ethernet') ? 'On Ethernet' : 'On Wi-Fi', fix: null, action: null },
          { id: 'vpn', status: 'pass', title: 'No VPN in the way', fix: null, action: null },
        ]
  // Mission 05 §1.5: files waiting for a busy device
  const next: CoreSnapshot['next'] =
    s === 'next'
      ? [{ id: 1, peerId: peers[0]?.id ?? '', peerName: peers[0]?.name ?? '', names: ['notes.txt', 'plan.pdf'], folders: 0, held: false, waitingForPaused: false }]
      : []
  return { doctor, me, peers, pairing, transfers: transfersFor(s === 'next' ? 'sending' : s, os), searchMs: s === 'searching-long' ? 12_000 : 0, clips: [], next, shelf: [], clipNotice: null }
}

/** `&update=…`: where updates stand (Settings → About and the notice) */
function updateFor(kind: string | null): CoreUpdate {
  const base = { current: '0.5.0-beta.1', version: null, notes: null, dismissed: false, requested: false, checkedMs: Date.now() - 600_000 }
  const next = {
    version: '0.5.0-beta.2',
    notes: 'Pair by typing a code, send to several computers at once, and keep text on the Shelf, shared with the computers you choose.',
  }
  switch (kind) {
    case 'ready':
      return { ...base, ...next, state: 'ready' }
    case 'available':
      return { ...base, ...next, state: 'available' }
    case 'waiting':
      return { ...base, ...next, state: 'waiting', peer: 'Desk PC', requested: true }
    case 'downloading':
      return { ...base, ...next, state: 'downloading', percent: 42, requested: true }
    case 'failed':
      return {
        ...base,
        ...next,
        requested: true,
        state: 'failed',
        message: 'The update didn’t download correctly, so it was thrown away. Nothing on this computer changed; SeamDrop 0.3.1 keeps running and tries again later.',
      }
    case 'offline':
      return { ...base, state: 'failed', message: 'Couldn’t reach the update server. Check the internet connection; SeamDrop tries again later.' }
    default:
      return { ...base, state: 'upToDate' }
  }
}

/** The core's words for time left (`estimate.rs` `Eta::of`) */
function etaOf(left: number): CoreEta {
  if (left >= 285) return { kind: 'minutes', n: Math.max(5, Math.round(left / 60)) }
  if (left >= 105) {
    const halves = Math.min(9, Math.max(4, Math.round(left / 30 - 0.15)))
    return halves % 2 === 0 ? { kind: 'minutes', n: halves / 2 } : { kind: 'minutes_and_a_half', n: Math.floor(halves / 2) }
  }
  if (left >= 85 && left < 95) return { kind: 'minute_and_a_half' }
  if (left >= 55 && left < 65) return { kind: 'about_a_minute' }
  if (left >= 12) return { kind: 'seconds', n: Math.min(100, Math.max(10, Math.round(left / 10) * 10)) }
  return { kind: 'few_seconds' }
}

/** A received image's small preview, for Recently received */
const THUMB = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAGCAYAAAD+Bd/7AAAAfElEQVR4nBXKoQ3DUAxF0UyXGTpDJjAoNI48gBUVFFhFUWQFBUUF3er1/oPPNNtPD7u12KWnnVpt18s+Ouytr22aZic4wQlOcIITnOAjBCEIQQhCEIIQhBghCUlIQhKSkIQk5AhFKEIRilCEIhShRmhCE5rQhCY0oQm96Q/ULIZxKzaNOAAAAABJRU5ErkJggg=='

const ACTIVE: CoreTransfer['phase'][] = ['preparing', 'offered', 'running', 'paused']

export function installMockCore(scenario: string, os: Os) {
  const s = (scenarioList as readonly string[]).includes(scenario) ? (scenario as Scenario) : 'searching'
  const params = new URLSearchParams(window.location.search)
  let snap = initial(s, os, Number(params.get('devices') ?? '1'))
  let removedShelf: CoreSnapshot['shelf'] = []
  let update = updateFor(params.get('update'))
  // `&clip=history|sync|asking|request|private|files|taken|paused`: the clipboard's states
  const clipKind = params.get('clip')
  const main = snap.peers.find((p) => p.paired) ?? snap.peers[0]
  const setClip = (id: string | undefined, clip: Partial<NonNullable<CorePeer['clip']>>) =>
    (snap = { ...snap, peers: snap.peers.map((p) => (p.id === id ? { ...p, clip: { ...pairedClip, ...p.clip, ...clip } } : p)) })
  if (clipKind === 'asking') setClip(main?.id, { mine: true })
  if (clipKind === 'request') setClip(main?.id, { theirs: true, asks: true })
  if (clipKind === 'sync' || clipKind === 'paused') {
    setClip(main?.id, { mine: true, theirs: true })
    snap = { ...snap, me: { ...snap.me, clipboardSync: clipKind === 'sync', clipboardPaused: clipKind === 'paused' ? Date.now() + 50 * 60_000 : null } }
  }
  let shortcut = {
    shortcut: os === 'mac' ? 'Control+Alt+Super+C' : 'Control+Alt+Shift+C',
    default: os === 'mac' ? 'Control+Alt+Super+C' : 'Control+Alt+Shift+C',
    error: clipKind === 'taken' ? 'Another app uses that shortcut, or it can’t be used. Try another.' : null,
  }
  if (clipKind === 'history' || clipKind === 'sync' || clipKind === 'paused') {
    const from = main?.name ?? 'Desk PC'
    const at = Date.now()
    const item = (over: Partial<CoreClip> & Pick<CoreClip, 'id'>): CoreClip => ({
      from,
      fromId: main?.id ?? null,
      atMs: at,
      kind: 'text',
      summary: 'text, 1.2 KB',
      preview: '“Meeting notes, Thursday: budg…”',
      copyable: true,
      pasted: true,
      ...over,
    })
    snap = {
      ...snap,
      clips: [
        item({ id: 5, atMs: at - 30_000, summary: 'private text, 23 bytes', preview: 'Private text', private: true }),
        item({ id: 4, atMs: at - 60_000 }),
        item({ id: 3, atMs: at - 600_000, kind: 'image', summary: 'image, 23.9 MB', preview: 'An image, 6000 × 4000', thumb: THUMB }),
        item({ id: 2, atMs: at - 3_600_000, kind: 'image', summary: 'image, 24.6 MB', preview: 'An image, 5472 × 3648' }),
        // Over the 64 MB the Shelf holds in all: the oldest big one is let go
        item({ id: 1, atMs: at - 7_200_000, kind: 'image', summary: 'image, 22.4 MB', preview: 'An image, 6000 × 4000', copyable: false }),
      ],
    }
  }
  // `&clip=notice|toobig|unreadable`: a copy that wasn't synced (Mission 05 §3.1)
  if (clipKind === 'notice' || clipKind === 'toobig' || clipKind === 'unreadable') {
    setClip(main?.id, { mine: true, theirs: true })
    snap = {
      ...snap,
      me: { ...snap.me, clipboardSync: true },
      clipNotice: { id: 1, kind: clipKind === 'notice' ? 'private' : clipKind === 'toobig' ? 'too_big' : 'unreadable', count: 41, atMs: Date.now() - 2000 },
    }
  }
  // `&shelf=1`: things kept on the Shelf, shared with the first device (Mission 05 §3.2)
  if (params.get('shelf')) {
    const other = snap.peers.find((p) => p.paired)
    const at = Date.now()
    snap = {
      ...snap,
      peers: snap.peers.map((p, i) => (p.paired ? { ...p, shelf: { mine: i === 0, theirs: i === 0, answered: i === 0 } } : p)),
      shelf: [
        { id: 'a'.repeat(32), name: 'Vercel key', preview: null, private: true, fromId: snap.me.id, fromName: snap.me.name, mine: true, atMs: at - 60_000 },
        { id: 'b'.repeat(32), name: 'Office address', preview: '14 Rue de la Paix, 75002 Paris', private: false, fromId: other?.id ?? snap.me.id, fromName: other?.name ?? snap.me.name, mine: !other, atMs: at - 3_600_000 },
        { id: 'c'.repeat(32), name: '', preview: 'ssh -i ~/.ssh/deploy.pem deploy@203.0.113.7', private: false, fromId: snap.me.id, fromName: snap.me.name, mine: true, atMs: at - 86_400_000 },
      ],
    }
  }
  const pushUpdate = (next: CoreUpdate) => {
    update = next
    void emit('portal:update', update)
  }
  // What the core writes, in the order it happened (newest last), and only
  // what this state's screen agrees with
  const first = snap.peers[0]
  const over = first?.link === 'ethernet' ? 'Ethernet' : 'Wi-Fi'
  const ago = (ms: number) => Date.now() - ms
  const linked = first && (first.online || first.reach || first.incompatible)
  const logs: CoreLogLine[] = [
    { atMs: ago(60_000), tag: 'start', text: `SeamDrop started as ${snap.me.name}`, detail: 'port 47920, protocol 3', tone: 'dim' as const },
    ...(first ? [{ atMs: ago(58_000), tag: 'found', text: `Found ${first.name} on this network`, detail: `${first.addr} via mDNS`, tone: null }] : []),
    ...(linked && !first.incompatible
      ? [
          {
            atMs: ago(57_900),
            tag: 'link',
            text: first.paired ? `Connected to ${first.name} over ${over}` : `${first.name} can be reached over ${over}; not paired`,
            detail: `${first.addr}, dialled`,
            tone: null,
          },
        ]
      : []),
    ...(first?.incompatible
      ? [{ atMs: ago(57_900), tag: 'error', text: `${first.name} runs another version of SeamDrop`, detail: `its protocol ${first.incompatible}, this one 3`, tone: 'warn' as const }]
      : []),
    ...(first?.reach
      ? [
          { atMs: ago(50_000), tag: 'link', text: `The connection to ${first.name} closed`, detail: 'connection reset', tone: 'dim' as const },
          ...(first.reach.state === 'unreachable'
            ? [{ atMs: ago(40_000), tag: 'link', text: `${first.name} isn’t reachable; trying again by itself`, detail: first.reach.lastFailure, tone: 'warn' as const }]
            : []),
        ]
      : []),
    ...(s === 'blocked-peer' ? [{ atMs: ago(5000), tag: 'block', text: `${first?.name} doesn’t accept files from this computer`, detail: 'Files from: Never there', tone: 'warn' as const }] : []),
    ...transferLogs(s, os),
    // A run of the same line: Details shows it once, "×3, last at …" (kept in time order below)
    ...(clipKind === 'history'
      ? [3000, 2000, 1000].map((ago, i) => ({
          atMs: Date.now() - ago,
          tag: 'clipboard',
          text: `Clipboard sent to ${snap.peers[0]?.name ?? 'Desk PC'}`,
          detail: `text, ${i + 1}.2 KB`,
          tone: null,
        }))
      : []),
  ].sort((x, y) => x.atMs - y.atMs)
  // Like the core: each pairing step has 2 minutes (a shown code too)
  let pairSince = Date.now()
  let pairStep = snap.pairing?.step
  const push = (next: CoreSnapshot) => {
    if (next.pairing && next.pairing.step !== pairStep && ['asking', 'asked', 'code'].includes(next.pairing.step)) pairSince = Date.now()
    pairStep = next.pairing?.step
    snap = next.pairing ? { ...next, pairing: { ...next.pairing, leftMs: Math.max(0, PAIR_TIMEOUT - (Date.now() - pairSince)) } } : next
    void emit('portal:event', { type: 'snapshot', ...snap } satisfies CoreEvent)
  }
  window.setInterval(() => {
    const pr = snap.pairing
    if (!pr || pr.failed || pr.step === 'paired' || Date.now() - pairSince < PAIR_TIMEOUT) return
    const failed = pr.mine
      ? pr.step === 'asking'
        ? `No answer from ${pr.peerName} within 2 minutes. Nothing was paired.`
        : 'The code expired after 2 minutes. Nothing was paired. Press Pair again for a new one.'
      : `The code expired after 2 minutes. Nothing was paired. To try again, press Pair on ${pr.peerName}.`
    push(pr.step === 'asked' ? { ...snap, pairing: null } : { ...snap, pairing: { ...pr, failed } })
  }, 1000)
  const setTransfers = (f: (ts: CoreTransfer[]) => CoreTransfer[]) => push({ ...snap, transfers: f(snap.transfers) })
  // Files waiting for a busy device start when its transfer ends (the core's queue)
  let nextIds = 100
  const shelfTexts = new Map<string, string>()
  window.setInterval(() => {
    const ready = snap.next.find((n) => !n.held && !snap.transfers.some((t) => t.peerId === n.peerId && ACTIVE.includes(t.phase)))
    if (!ready) return
    const fresh = baseTransfer(os, { id: `0000000000e${String(ready.id).padStart(5, '0')}`, peerId: ready.peerId, peerName: ready.peerName, phase: 'offered' })
    push({ ...snap, next: snap.next.filter((n) => n.id !== ready.id), transfers: [...snap.transfers.filter((t) => t.peerId !== ready.peerId || ACTIVE.includes(t.phase)), fresh] })
  }, 700)
  // Running transfers move, so the progress bar and edge glow can be seen live,
  // and finish. `&still=1` (the layout check) holds them where they start, so
  // a state can't turn into another while it's being measured.
  const still = params.get('still') === '1'
  const finishing = new Set<string>()
  window.setInterval(() => {
    if (still) return
    for (const t of snap.transfers) {
      if (t.phase !== 'running' || t.stalled) continue
      if (t.bytes >= t.totalBytes) {
        // All there: the other side checks the last of it, then it's done
        if (!finishing.has(t.id)) {
          finishing.add(t.id)
          window.setTimeout(() => {
            const done: CoreResult = {
              savedTo: t.direction === 'in' ? (os === 'mac' ? '/Users/sam/Downloads/SeamDrop' : 'C:\\Users\\Sam\\Downloads\\SeamDrop') : null,
              reveal: null,
              files: t.fileCount,
              bytes: t.totalBytes,
              renamed: 0,
              skipped: 0,
              longPaths: 0,
              keptBytes: 0,
              skippedItems: [],
              renamedItems: [],
              notes: [],
              firstArrival: false,
            }
            setTransfers((ts) => ts.map((x) => (x.id === t.id ? { ...x, phase: 'done', elapsedMs: Math.round((t.totalBytes / 64_000_000) * 1000), result: done } : x)))
          }, 1200)
        }
        continue
      }
      const bytes = Math.min(t.totalBytes, t.bytes + 6_400_000)
      snap = { ...snap, transfers: snap.transfers.map((x) => (x.id === t.id ? { ...x, bytes } : x)) }
      // What the core says: a calm speed and time left (Mission 04 §1.2)
      void emit('portal:event', {
        type: 'progress',
        id: t.id,
        bytes,
        total: t.totalBytes,
        rate: 64_000_000,
        speed: 64_000_000,
        eta: etaOf((t.totalBytes - bytes) / 64_000_000),
        file: fileAt(t, bytes),
      } satisfies CoreEvent)
    }
  }, 100)
  mockWindows('main')
  mockIPC(
    (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>
      const tid = String(a.transferId ?? '')
      // `&fail=portal_save_settings`: that command fails, as beta.1's Settings did
      if (params.get('fail')?.split(',').includes(cmd)) throw `invalid args \`patch\` for command \`${cmd}\`: missing field \`clearReceiveNote\``
      switch (cmd) {
        case 'portal_snapshot':
          return snap
        case 'portal_logs':
          return logs
        case 'portal_retry':
          push({ ...snap, searchMs: 0 })
          return null
        case 'portal_connect': {
          // Like the core: a real IPv4 address (an optional port), and the
          // devices already known stay as they are
          const address = String(a.address ?? '').trim()
          const m = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?::(\d{1,5}))?$/.exec(address)
          if (!m || m.slice(1, 5).some((o) => Number(o) > 255) || (m[5] && Number(m[5]) > 65535)) {
            throw 'That doesn’t look like an IP address. Try something like 192.168.1.20.'
          }
          const host = m.slice(1, 5).join('.')
          const known = snap.peers.find((p) => p.addr?.replace(/:\d+$/, '') === host)
          if (known) {
            const found = { ...known, online: true, reach: null }
            push({ ...snap, peers: snap.peers.map((p) => (p.id === known.id ? found : p)) })
            return found
          }
          const found: CorePeer = { ...mainPeer({ via: 'manual', addr: `${host}:47920`, online: true }, os), id: 'e'.repeat(32), name: os === 'mac' ? 'Spare PC' : 'Spare Mac' }
          push({ ...snap, peers: [...snap.peers, found] })
          return found
        }
        case 'portal_pair': {
          // Asks; the other computer's person shows a code after a moment
          const target = snap.peers.find((p) => p.id === a.peerId) ?? snap.peers[0]
          const addr = target.addr?.replace(/:\d+$/, '') ?? '192.168.1.31'
          push({ ...snap, pairing: { peerId: target.id, peerName: target.name, peerAddr: addr, mine: true, step: 'asking', code: null, triesLeft: 3, failed: null, leftMs: PAIR_TIMEOUT } })
          window.setTimeout(() => snap.pairing?.step === 'asking' && push({ ...snap, pairing: { ...snap.pairing, step: 'code' } }), 1500)
          return null
        }
        case 'portal_pairing': {
          const act = a.action as CorePairAction
          const pr = snap.pairing
          if (act.type === 'allowAgain') {
            push({ ...snap, me: { ...snap.me, pairingLocked: false, pairingLockedBy: null, pairingLockedAtMs: null } })
            return null
          }
          if (!pr) return null
          if (act.type === 'showCode') push({ ...snap, pairing: { ...pr, step: 'code', code: '048213' } })
          else if (act.type === 'code') {
            // The mock's code is 048 213, exactly (the core compares nothing
            // else: a wrong code fails the key exchange)
            push({ ...snap, pairing: { ...pr, step: 'checking' } })
            window.setTimeout(() => {
              const right = act.code === '048213'
              if (right) {
                push({
                  ...snap,
                  pairing: { ...pr, step: 'paired' },
                  peers: snap.peers.map((p) => (p.id === pr.peerId ? { ...p, paired: true, online: true, revoked: false, clip: pairedClip } : p)),
                })
              } else if (pr.triesLeft > 1) push({ ...snap, pairing: { ...pr, step: 'wrong', triesLeft: pr.triesLeft - 1 } })
              else push({ ...snap, pairing: { ...pr, failed: `That wasn’t the code shown on ${pr.peerName}, 3 times. Nothing was paired. Pair again for a new code.` } })
            }, 700)
          } else push({ ...snap, pairing: null })
          return null
        }
        case 'portal_save_settings': {
          const patch = a.patch as SettingsPatch
          push({
            ...snap,
            me: {
              ...snap.me,
              name: patch.deviceName ?? snap.me.name,
              skipJunk: patch.skipJunk ?? snap.me.skipJunk,
              receiveDir: patch.receiveDir ?? snap.me.receiveDir,
              autoUpdate: patch.autoUpdate ?? snap.me.autoUpdate,
              testBuilds: patch.testBuilds ?? snap.me.testBuilds,
              sendTo: patch.sendTo ?? snap.me.sendTo,
              appearance: patch.appearance ?? snap.me.appearance,
              folderPerDevice: patch.folderPerDevice ?? snap.me.folderPerDevice,
              receiveMode: patch.receiveMode ?? snap.me.receiveMode,
              keepDates: patch.keepDates ?? snap.me.keepDates,
            },
          })
          return null
        }
        case 'portal_set_clip_sync': {
          const on = !!a.on
          snap = {
            ...snap,
            peers: snap.peers.map((p) => {
              if (p.id !== a.peerId) return p
              const agreed = (p.clip?.asks || p.clip?.theirs) ?? false
              return { ...p, clip: { ...pairedClip, ...p.clip, mine: on, asks: false, theirs: on ? agreed : false } }
            }),
          }
          push({ ...snap, me: { ...snap.me, clipboardSync: snap.peers.some((p) => p.clip?.mine && p.clip.theirs) } })
          return null
        }
        case 'portal_clip_sync_off':
          push({
            ...snap,
            me: { ...snap.me, clipboardSync: false, clipboardPaused: null },
            peers: snap.peers.map((p) => (p.clip ? { ...p, clip: { ...p.clip, mine: false, theirs: false, asks: false } } : p)),
          })
          return null
        case 'portal_pause_clip_sync':
          push({ ...snap, me: { ...snap.me, clipboardPaused: a.minutes ? Date.now() + Number(a.minutes) * 60_000 : 0 } })
          return null
        case 'portal_resume_clip_sync':
          push({ ...snap, me: { ...snap.me, clipboardPaused: null } })
          return null
        case 'portal_set_clip_to':
          push({ ...snap, peers: snap.peers.map((p) => (p.id === a.peerId ? { ...p, clip: { ...pairedClip, ...p.clip, to: a.to as 'clipboard' | 'shelf' } } : p)) })
          return null
        case 'portal_set_connected':
          push({ ...snap, peers: snap.peers.map((p) => (p.id === a.peerId ? { ...p, disconnected: !a.value, online: !!a.value, reach: null } : p)) })
          return null
        case 'portal_set_incoming':
          push({ ...snap, peers: snap.peers.map((p) => (p.id === a.peerId ? { ...p, incoming: a.value as CorePeer['incoming'] } : p)) })
          return null
        case 'portal_rename_peer':
          push({ ...snap, peers: snap.peers.map((p) => (p.id === a.peerId ? { ...p, name: String(a.name) } : p)) })
          return null
        case 'portal_remove_peer':
          // As the core does: what it sent leaves the Shelf too
          push({
            ...snap,
            peers: snap.peers.map((p) => (p.id === a.peerId ? { ...p, paired: false, revoked: false, clip: null } : p)),
            clips: snap.clips.filter((c) => c.fromId !== a.peerId),
          })
          return null
        case 'portal_send': {
          const ids = (a.peerIds as string[]) ?? []
          return ids.map((id, i) => {
            const p = snap.peers.find((x) => x.id === id)
            if (s === 'refused' || !p?.online) return { peerId: id, error: `${p?.name ?? 'That device'} isn’t connected right now.` }
            // Busy: they wait and go next, as the core does (Mission 05 §1.5)
            const busy = snap.transfers.find((t) => t.peerId === id && ACTIVE.includes(t.phase))
            if (busy) {
              const paths = (a.paths as string[]) ?? []
              const names = paths.map((x) => x.split(/[\\/]/).pop() ?? x)
              const entry = { id: ++nextIds, peerId: id, peerName: p.name, names, folders: names.filter((n) => !n.includes('.')).length, held: false, waitingForPaused: busy.phase === 'paused' }
              push({ ...snap, next: [...snap.next, entry] })
              return { peerId: id, error: null }
            }
            const fresh = baseTransfer(os, { id: `0000000000f${i}${String(Date.now()).slice(-4)}`, peerId: id, peerName: p.name, phase: 'offered' })
            setTransfers((ts) => [...ts.filter((t) => t.peerId !== id), fresh])
            return { peerId: id, error: null }
          })
        }
        case 'portal_describe': {
          // A moment of "Counting…", then what each path holds
          const paths = (a.paths as string[]) ?? []
          return new Promise((done) =>
            window.setTimeout(
              () =>
                done(
                  paths.map((p) => {
                    const name = p.split(/[\\/]/).pop() ?? p
                    if (name.startsWith('Shortcut')) return null
                    const folder = !name.includes('.')
                    return folder ? { name, folder, bytes: 611_400_000, files: 212 } : { name, folder, bytes: name.endsWith('.pdf') ? 2_400_000 : 18_700_000, files: 1 }
                  }),
                ),
              1500,
            ),
          )
        }
        case 'portal_accept':
          setTransfers((ts) => ts.map((t) => (t.id === tid ? { ...t, phase: 'running' as const } : t)))
          if (s === 'instant') {
            window.setTimeout(() => setTransfers((ts) => ts.map((t) => (t.id === tid ? { ...transfersFor('done-in', os)[0], elapsedMs: 40 } : t))), 40)
          }
          return null
        case 'plugin:window|close':
          document.body.dataset.closed = ''
          return null
        case 'portal_cancel':
          setTransfers((ts) =>
            ts.flatMap((t) => {
              if (t.id !== tid) return [t]
              if (t.phase !== 'running' && t.phase !== 'paused') return []
              const out = t.direction === 'out'
              const failure = out
                ? `You cancelled it. Your files are still on this computer; nothing was saved on ${t.peerName}.`
                : `You cancelled it. What had arrived from ${t.peerName} was removed.`
              return [{ ...t, phase: 'cancelled' as const, canSendAgain: out, failure }]
            }),
          )
          return null
        case 'portal_fix_firewall':
          // A moment for the administrator question, then the check passes
          return new Promise((done) =>
            window.setTimeout(() => {
              push({ ...snap, doctor: snap.doctor.filter((c) => c.id !== 'firewall-block') })
              done('Fixed. Windows Firewall allows SeamDrop again.')
            }, 1500),
          )
        case 'portal_send_again':
          setTransfers((ts) => ts.map((t) => (t.id === tid ? { ...t, phase: 'offered' as const, failure: null, canSendAgain: false, bytes: 0 } : t)))
          return null
        case 'portal_decline':
        case 'portal_dismiss':
          setTransfers((ts) => ts.filter((t) => t.id !== tid))
          return null
        case 'portal_shelf_add': {
          const id = Math.random().toString(16).slice(2).padEnd(32, '0').slice(0, 32)
          const text = String(a.text)
          const item = { id, name: String(a.name ?? ''), preview: a.private ? null : text.split('\n')[0].slice(0, 80), private: !!a.private, fromId: snap.me.id, fromName: snap.me.name, mine: true, atMs: Date.now() }
          shelfTexts.set(id, text)
          push({ ...snap, shelf: [item, ...snap.shelf] })
          return null
        }
        case 'portal_shelf_edit':
          push({ ...snap, shelf: snap.shelf.map((i) => (i.id === a.id && a.name !== null ? { ...i, name: String(a.name) } : i)) })
          return null
        case 'portal_shelf_remove': {
          const ids = a.ids as string[] | null
          removedShelf = snap.shelf.filter((i) => !ids || ids.includes(i.id))
          push({ ...snap, shelf: ids ? snap.shelf.filter((i) => !ids.includes(i.id)) : [] })
          return null
        }
        case 'portal_shelf_undo':
          push({ ...snap, shelf: [...removedShelf, ...snap.shelf] })
          removedShelf = []
          return null
        case 'portal_shelf_copy':
          return null
        case 'portal_shelf_reveal':
          return shelfTexts.get(String(a.id)) ?? 'sk_live_51H…demo'
        case 'portal_set_shelf_share':
          push({ ...snap, peers: snap.peers.map((p) => (p.id === a.peer ? { ...p, shelf: { mine: !!a.on, theirs: !!p.shelf?.theirs, answered: true } } : p)) })
          return null
        case 'portal_set_picture':
          push({ ...snap, me: { ...snap.me, picture: (a.picture as CoreSnapshot['me']['picture']) ?? null } })
          return null
        case 'portal_shelf_put_marked':
        case 'portal_shelf_put_received': {
          // As the core does: the text goes on the Shelf (private stays private)
          const clip = a.id !== undefined ? snap.clips.find((c) => c.id === Number(a.id)) : undefined
          const priv = a.id === undefined || !!clip?.private
          const text = priv ? 'sk_test_51Hx3…private' : (clip?.preview ?? 'Copied text')
          const item = { id: Math.random().toString(16).slice(2).padEnd(32, '0').slice(0, 32), name: '', preview: priv ? null : text.slice(0, 80), private: priv, fromId: snap.me.id, fromName: snap.me.name, mine: true, atMs: Date.now() }
          shelfTexts.set(item.id, text)
          push({ ...snap, clipNotice: a.id === undefined ? null : snap.clipNotice, shelf: [item, ...snap.shelf] })
          return null
        }
        case 'portal_save_clip_image':
          return os === 'mac' ? '/Users/sam/Downloads/Clipboard image.png' : 'C:\\Users\\Sam\\Downloads\\Clipboard image.png'
        case 'portal_dismiss_clip_notice':
          push({ ...snap, clipNotice: null })
          return null
        case 'portal_unqueue_next':
          push({ ...snap, next: snap.next.filter((n) => n.id !== Number(a.id)) })
          return null
        case 'portal_continue_next':
          push({ ...snap, next: snap.next.map((n) => (n.peerId === a.peer ? { ...n, held: false } : n)) })
          return null
        case 'portal_continue':
          setTransfers((ts) => ts.map((t) => (t.id === tid ? { ...t, phase: 'running' as const, stoppedHere: false, pausedSinceMs: null } : t)))
          return null
        case 'plugin:dialog|open': {
          const dir = (a.options as { directory?: boolean } | undefined)?.directory
          const home = os === 'mac' ? '/Users/sam' : 'C:\\Users\\Sam'
          const sep = os === 'mac' ? '/' : '\\'
          return dir
            ? [`${home}${sep}Pictures${sep}Site photos`]
            : [`${home}${sep}Documents${sep}Floor plan.dwg`, `${home}${sep}Documents${sep}Résumé Q3.pdf`, `${home}${sep}Desktop${sep}Shortcut to Notes.lnk`]
        }
        case 'plugin:autostart|is_enabled':
          return false
        case 'portal_update_status':
          return update
        case 'portal_shortcut':
          return shortcut
        case 'portal_set_shortcut': {
          const want = String(a.shortcut ?? '') || shortcut.default
          if (want.endsWith('+V')) throw 'Another app uses that shortcut, or it can’t be used. Try another.'
          shortcut = { ...shortcut, shortcut: want, error: null }
          return shortcut
        }
        case 'portal_send_clipboard': {
          const ids = (a.peerIds as string[] | null) ?? snap.peers.filter((p) => p.paired).map((p) => p.id)
          const to = snap.peers.filter((p) => ids.includes(p.id) && p.online)
          if (!to.length) throw `${snap.peers[0]?.name ?? 'Your other computer'} isn’t connected right now.`
          if (clipKind === 'private') throw 'The app you copied this from marked it as private, so it wasn’t sent. To send it anyway, paste it into the Shelf’s text box.'
          if (clipKind === 'files') {
            const dir = os === 'mac' ? '/Users/sam/Desktop/' : 'C:\\Users\\Sam\\Desktop\\'
            return { said: '2 files on the clipboard', files: [`${dir}Floor plan.dwg`, `${dir}Site photos`] }
          }
          const list = to.map((p) => p.name)
          const joined = list.length > 1 ? `${list.slice(0, -1).join(', ')} and ${list.at(-1)}` : list[0]
          logs.push({ atMs: Date.now(), tag: 'clipboard', text: `Clipboard sent to ${joined}`, detail: 'text, 1.2 KB', tone: null })
          return {
            said: `Sent to ${joined} (text, 1.2 KB). Paste it there ${to.length > 1 ? 'as usual' : `with ${os === 'mac' ? 'Ctrl+V' : '⌘V'}`}.`,
            files: [],
          }
        }
        case 'portal_send_text':
          if (!String(a.text ?? '').trim()) throw 'Type something to send first.'
          return a.private
            ? `Sent to ${snap.peers[0]?.name ?? 'Desk PC'} (private text, ${String(a.text).length} bytes). Paste it there as usual. It’s marked private there, so clipboard history leaves it.`
            : `Sent to ${snap.peers[0]?.name ?? 'Desk PC'} (text, ${String(a.text).length} bytes). Paste it there as usual.`
        case 'portal_remove_clip':
          push({ ...snap, clips: snap.clips.filter((c) => c.id !== Number(a.id)) })
          return null
        case 'portal_copy_clip':
        case 'portal_copy_technical_log':
        case 'portal_open_received':
          return null
        case 'portal_save_technical_log': {
          // Stamped like the core: now, in UTC
          const stamp = `${new Date().toISOString().slice(0, 16).replace('T', ' ').replace(':', '-')} UTC`
          return os === 'win' ? `C:\\Users\\Sam\\Downloads\\SeamDrop technical log ${stamp}.txt` : `/Users/sam/Downloads/SeamDrop technical log ${stamp}.txt`
        }
        case 'portal_clear_clips':
          push({ ...snap, clips: [] })
          return null
        case 'portal_update_check':
          pushUpdate({ ...update, state: 'checking' })
          window.setTimeout(() => pushUpdate({ ...updateFor(null), checkedMs: Date.now() }), 1200)
          return null
        case 'portal_update_restart': {
          const running = snap.transfers.find((t) => t.phase === 'running')
          pushUpdate(running ? { ...update, state: 'waiting', peer: running.peerName, requested: true } : { ...update, state: 'installing', requested: true })
          return null
        }
        case 'portal_update_later':
          pushUpdate({ ...update, dismissed: true, requested: false, ...(update.state === 'waiting' ? { state: 'ready' } : {}) })
          return null
        default:
          return null
      }
    },
    { shouldMockEvents: true },
  )
}
