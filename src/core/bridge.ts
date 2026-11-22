/* Types and calls for the Rust core (src-tauri/src/bridge.rs). The webview
   only ever sends paths and choices; file bytes never pass through it. */
import type { Picture } from '../window/picture/picture'
import { invoke } from '@tauri-apps/api/core'

/** What a computer does with files from one paired device */
export type CoreIncoming = 'ask' | 'always' | 'never'
export type CoreOs = 'macos' | 'windows' | 'linux' | 'other'
export type CoreLink = 'ethernet' | 'wifi' | 'other'
export type CoreVia = 'mdns' | 'broadcast' | 'known' | 'manual' | 'inbound'
/** Details' tones: amber when wrong and it affects you, red when it failed and needs you */
export type CoreTone = 'warn' | 'err' | 'dim'

export interface CoreAddress {
  label: string
  address: string
}

export interface CoreMe {
  id: string
  name: string
  os: CoreOs
  port: number
  /** Protocol version this computer speaks */
  proto: number
  receiveDir: string
  skipJunk: boolean
  /** Install updates automatically (Settings → About) */
  autoUpdate: boolean
  /** Get test builds (Settings → About): the beta feed is checked too */
  testBuilds: boolean
  /** Copies sync with at least one device right now (both agreed, not paused) */
  clipboardSync: boolean
  /** Sync is paused here until then (epoch ms; 0: until SeamDrop restarts) */
  clipboardPaused: number | null
  /** Where files and the clipboard go: the remembered choice */
  sendTo: string[]
  /** The Send clipboard shortcut, if changed from the default */
  clipboardShortcut: string | null
  /** Settings → Appearance (the tray has it too) */
  appearance: 'system' | 'light' | 'dark'
  /** "A folder for each device" */
  folderPerDevice: boolean
  receiveMode: 'loose' | 'app_folder' | 'numbered'
  keepDates: boolean
  receiveNote: string | null
  /** Too many wrong pairing codes: requests are turned down until allowed again */
  pairingLocked: boolean
  /** Who typed the last wrong code: "Desk PC (192.168.1.31)" */
  pairingLockedBy?: string | null
  /** When (epoch ms) */
  pairingLockedAtMs?: number | null
  /** For Send feedback: "0.4.0", "macOS 15.1 (24B83)" */
  appVersion: string
  osVersion: string
  addresses: CoreAddress[]
  /** This computer's picture (Mission 05 §2.3); null: the default */
  picture?: Picture | null
}

export interface CorePeer {
  id: string
  name: string
  os: CoreOs
  /** Its picture, once checked by the core; null: the default */
  picture?: Picture | null
  paired: boolean
  online: boolean
  link: CoreLink | null
  addr: string | null
  via: CoreVia | null
  /** What this computer does with files from it */
  incoming: CoreIncoming
  /** What it does with files from this computer, once it has said */
  accepts: CoreIncoming | null
  incompatible: number | null
  revoked: boolean
  /** Paired, but Disconnect was pressed here */
  disconnected: boolean
  /** Paired and not connected: how connecting is going (the core decides
      when it "isn't reachable"; the UI never guesses from timers) */
  reach: CoreReach | null
  /** Paired: the clipboard with it */
  clip: CoreClipPeer | null
  shelf?: CoreShelfPeer | null
}

export interface CoreClipPeer {
  /** This computer agreed to sync with it */
  mine: boolean
  /** It agreed to sync with this computer */
  theirs: boolean
  /** It asks this computer to agree */
  asks: boolean
  /** Where its clipboard items go here */
  to: 'clipboard' | 'shelf'
}

export interface CoreReach {
  state: 'connecting' | 'unreachable'
  sinceMs: number
  attempts: number
  lastFailure: string | null
  nextTryMs: number | null
  step: 'looking' | 'trying_known' | 'trying_found'
}

/** Where a pairing by typed code is (Mission 04 §2) */
export type CorePairStep = 'asking' | 'asked' | 'code' | 'checking' | 'wrong' | 'paired'

export interface CorePairing {
  peerId: string
  peerName: string
  /** Its address: "MacBook Pro (192.168.1.20) wants to pair" */
  peerAddr: string
  /** This computer asked: its person types the code */
  mine: boolean
  step: CorePairStep
  /** The code to show (the computer that was asked, after Show code) */
  code: string | null
  /** Wrong codes still allowed in this pairing */
  triesLeft: number
  failed: string | null
  /** How long this step has left (a code works for 2 minutes), at the snapshot */
  leftMs: number
}

/** What the person did about a pairing */
export type CorePairAction =
  | { type: 'showCode' }
  | { type: 'ignore' }
  | { type: 'code'; code: string }
  | { type: 'cancel' }
  | { type: 'dismiss' }
  | { type: 'allowAgain' }

export type CoreDirection = 'out' | 'in'
export type CorePhase =
  | 'preparing'
  | 'offered'
  | 'running'
  | 'paused'
  | 'done'
  | 'failed'
  | 'cancelled'
  | 'declined'
  | 'blocked'

/** "Docs/link.txt", "a link (shortcut): links aren’t sent" */
export interface CoreListedItem {
  path: string
  why: string
}

export interface CoreResult {
  savedTo: string | null
  reveal: string | null
  /** One file that's safe to Open (documents, images, media; never apps or scripts) */
  open?: string | null
  files: number
  bytes: number
  renamed: number
  skipped: number
  longPaths: number
  /** Already there after a pause, not sent again (left out of the speed) */
  keptBytes: number
  /** Which items were left out, and why (the first 50) */
  skippedItems: CoreListedItem[]
  /** Which names changed, to what, and why (the first 50) */
  renamedItems: CoreListedItem[]
  notes: string[]
  /** The first files this computer ever received (the done screen offers Change folder) */
  firstArrival: boolean
}

export interface CoreTransfer {
  id: string
  direction: CoreDirection
  peerId: string
  peerName: string
  phase: CorePhase
  /** The receiver's "Files from <this computer>" is Never */
  block: 'not_accepted' | null
  items: { name: string; folder: boolean; bytes: number; files: number }[]
  totalBytes: number
  fileCount: number
  autoAccepted: boolean
  bytes: number
  elapsedMs: number | null
  link: CoreLink | null
  /** Running, but nothing has moved for a few seconds */
  stalled: boolean
  result: CoreResult | null
  failure: string | null
  /** Receiver: the folder its files are (or will be) saved in */
  saveTo: string | null
  /** Paused since (epoch ms): the connection dropped; it continues by itself */
  pausedSinceMs: number | null
  /** Paused, connected again, checking what arrived before continuing */
  resuming: boolean
  /** Quit or crashed here mid-transfer: asks Continue or Cancel */
  stoppedHere: boolean
  /** The other computer was quit or crashed: waiting for it to continue */
  waitingThere: boolean
  /** False while the percentage is a guess */
  percentKnown: boolean
  /** Sender, stopped or cancelled: Send again sends the same items */
  canSendAgain: boolean
}

/** What one path would send (null: unreadable, or a link) */
export interface CoreItem {
  name: string
  folder: boolean
  bytes: number
  files: number
}

export interface CoreProgress {
  id: string
  /** Bytes that arrived (the sender: what the receiver confirmed) */
  bytes: number
  total: number
  /** Bytes per second over the last 10 s */
  rate: number
  /** The speed to show (changes at most every 2 s, 2 significant figures);
      null for the first seconds of a run */
  speed: number | null
  /** Time left, decided by the core; the UI only words it */
  eta: CoreEta
  file: string
}

export interface CoreNext {
  id: number
  peerId: string
  peerName: string
  names: string[]
  folders: number
  /** Kept through a quit: waits for Continue */
  held: boolean
  /** The device's slot is taken by a paused transfer */
  waitingForPaused: boolean
}

export type CoreEta =
  | { kind: 'working_out' }
  | { kind: 'checking' }
  | { kind: 'hidden' }
  | { kind: 'minutes'; n: number }
  | { kind: 'minutes_and_a_half'; n: number }
  | { kind: 'minute_and_a_half' }
  | { kind: 'about_a_minute' }
  | { kind: 'seconds'; n: number }
  | { kind: 'few_seconds' }

export interface CoreNotice {
  kind: 'offer' | 'received' | 'failed' | 'paused' | 'clipboard' | 'clipboard_sync'
  title: string
  body: string
}

export interface CoreCheck {
  id: string
  status: 'pass' | 'fail' | 'likely' | 'unknown'
  title: string
  fix: string | null
  action: { label: string; url: string } | null
}

export interface SettingsPatch {
  deviceName?: string
  receiveDir?: string
  skipJunk?: boolean
  autoUpdate?: boolean
  testBuilds?: boolean
  /** true: agree with every paired device; false: off with all */
  clipboardSync?: boolean
  /** Where files and the clipboard go (remembered) */
  sendTo?: string[]
  appearance?: 'system' | 'light' | 'dark'
  folderPerDevice?: boolean
  /** Mission 05 §4: how received files are put away, and their dates */
  receiveMode?: 'loose' | 'app_folder' | 'numbered'
  keepDates?: boolean
  clearReceiveNote?: boolean
}

/** One device's answer to a send */
export interface CoreSendResult {
  peerId: string
  /** Why it didn't start, in plain words */
  error: string | null
}

/** One clipboard item received (its contents stay in the core) */
export interface CoreClip {
  id: number
  from: string
  /** The device it came from */
  fromId: string | null
  atMs: number
  kind: string
  /** "text, 1.2 KB" */
  summary: string
  /** “meeting notes…” or "An image, 1200 × 800" */
  preview: string
  copyable: boolean
  /** Also put on the clipboard here (false: Shelf only) */
  pasted: boolean
  /** Sent as private on purpose: no preview; marked private when copied */
  private?: boolean
  /** An image: a small preview (data URL) */
  thumb?: string | null
}

/** Sharing the Shelf with one paired device (Mission 05 §3.2) */
export interface CoreShelfPeer {
  mine: boolean
  theirs: boolean
  answered: boolean
}

/** One Shelf item (a private one's text stays in the core) */
export interface CoreShelfItem {
  id: string
  name: string
  preview: string | null
  private: boolean
  fromId: string
  fromName: string
  mine: boolean
  atMs: number
}

/** A copy that wasn't synced, and why (Mission 05 §3.1) */
export interface CoreClipNotice {
  id: number
  kind: 'private' | 'too_big' | 'unreadable'
  count: number
  atMs: number
}

/** The Send clipboard shortcut (src-tauri/src/clipboard.rs) */
export interface CoreShortcut {
  /** Accelerator form, e.g. "Control+Alt+Super+C" */
  shortcut: string
  default: string
  /** Why it isn't working, if it isn't */
  error: string | null
}

/** Where the app is with updates (src-tauri/src/updates.rs) */
export type CoreUpdateState =
  | { state: 'idle' }
  | { state: 'checking' }
  | { state: 'upToDate' }
  | { state: 'available' }
  | { state: 'downloading'; percent: number }
  | { state: 'ready' }
  | { state: 'waiting'; peer: string }
  | { state: 'installing' }
  | { state: 'failed'; message: string }

export type CoreUpdate = CoreUpdateState & {
  /** The running version */
  current: string
  /** The version on offer, and what's new in it */
  version: string | null
  notes: string | null
  /** Later was pressed for this version */
  dismissed: boolean
  /** Restart was pressed: show the download and install in the window */
  requested: boolean
  checkedMs: number | null
}

export interface CoreSnapshot {
  me: CoreMe
  peers: CorePeer[]
  pairing: CorePairing | null
  /** Every transfer on screen, oldest first (Mission 04 §3) */
  transfers: CoreTransfer[]
  doctor: CoreCheck[]
  searchMs: number
  /** Clipboard items received, newest first */
  clips: CoreClip[]
  /** What goes next to a busy device (Mission 05 §1.5) */
  next: CoreNext[]
  /** The Shelf, newest first (Mission 05 §3.2) */
  shelf: CoreShelfItem[]
  /** The latest copy that wasn't synced */
  clipNotice: CoreClipNotice | null
}

export interface CoreLogLine {
  atMs: number
  tag: string
  /** Plain words */
  text: string
  /** The technical part */
  detail: string | null
  tone: CoreTone | null
}

export type CoreEvent =
  | ({ type: 'snapshot' } & CoreSnapshot)
  | ({ type: 'log' } & CoreLogLine)
  | ({ type: 'progress' } & CoreProgress)
  | ({ type: 'notice' } & CoreNotice)

export const EVENT = 'portal:event'
export const UPDATE_EVENT = 'portal:update'

export const core = {
  snapshot: () => invoke<CoreSnapshot>('portal_snapshot'),
  logs: () => invoke<CoreLogLine[]>('portal_logs'),
  connect: (address: string) => invoke<CorePeer>('portal_connect', { address }),
  retry: () => invoke<void>('portal_retry'),
  pair: (peerId: string) => invoke<void>('portal_pair', { peerId }),
  pairing: (action: CorePairAction) => invoke<void>('portal_pairing', { action }),
  removePeer: (peerId: string) => invoke<void>('portal_remove_peer', { peerId }),
  renamePeer: (peerId: string, name: string) => invoke<void>('portal_rename_peer', { peerId, name }),
  setIncoming: (peerId: string, value: CoreIncoming) => invoke<void>('portal_set_incoming', { peerId, value }),
  setConnected: (peerId: string, value: boolean) => invoke<void>('portal_set_connected', { peerId, value }),
  saveSettings: (patch: SettingsPatch) => invoke<void>('portal_save_settings', { patch }),
  /** One transfer per device; says per device what didn't start */
  send: (peerIds: string[], paths: string[]) => invoke<CoreSendResult[]>('portal_send', { peerIds, paths }),
  describe: (paths: string[]) => invoke<(CoreItem | null)[]>('portal_describe', { paths }),
  accept: (transferId: string, always: boolean) => invoke<void>('portal_accept', { transferId, always }),
  decline: (transferId: string) => invoke<void>('portal_decline', { transferId }),
  cancel: (transferId: string) => invoke<void>('portal_cancel', { transferId }),
  continueTransfer: (transferId: string) => invoke<void>('portal_continue', { transferId }),
  openFile: (path: string) => invoke<void>('portal_open_file', { path }),
  sendAgain: (transferId: string) => invoke<void>('portal_send_again', { transferId }),
  fixFirewall: () => invoke<string>('portal_fix_firewall'),
  dismiss: (transferId: string) => invoke<void>('portal_dismiss', { transferId }),
  updateStatus: () => invoke<CoreUpdate | null>('portal_update_status'),
  checkUpdates: () => invoke<void>('portal_update_check'),
  restartToUpdate: () => invoke<void>('portal_update_restart'),
  updateLater: () => invoke<void>('portal_update_later'),
  /** To these devices, or (none) the remembered choice */
  /** The Shelf's text box: typed text, sent like a copy */
  sendText: (peerIds: string[] | undefined, text: string, isPrivate: boolean) =>
    invoke<string>('portal_send_text', { peerIds: peerIds ?? null, text, private: isPrivate }),
  /** What was said, or the files on the clipboard to offer (not sent) */
  sendClipboard: (peerIds?: string[]) => invoke<{ said: string; files: string[] }>('portal_send_clipboard', { peerIds: peerIds ?? null }),
  setClipSync: (peerId: string, on: boolean) => invoke<void>('portal_set_clip_sync', { peerId, on }),
  clipSyncOff: () => invoke<void>('portal_clip_sync_off'),
  pauseClipSync: (minutes?: number) => invoke<void>('portal_pause_clip_sync', { minutes: minutes ?? null }),
  resumeClipSync: () => invoke<void>('portal_resume_clip_sync'),
  setClipTo: (peerId: string, to: 'clipboard' | 'shelf') => invoke<void>('portal_set_clip_to', { peerId, to }),
  copyClip: (id: number) => invoke<void>('portal_copy_clip', { id }),
  /** Details → Copy technical log (not synced to other computers) */
  copyTechnicalLog: () => invoke<void>('portal_copy_technical_log'),
  /** Opens the received-files folder */
  openReceived: () => invoke<void>('portal_open_received'),
  /** Saves the technical log in Downloads; its path */
  saveTechnicalLog: () => invoke<string>('portal_save_technical_log'),
  clearClips: () => invoke<void>('portal_clear_clips'),
  removeClip: (id: number) => invoke<void>('portal_remove_clip', { id }),
  shelfAdd: (name: string, text: string, isPrivate: boolean) => invoke<void>('portal_shelf_add', { name, text, private: isPrivate }),
  shelfEdit: (id: string, name: string | null, text: string | null) => invoke<void>('portal_shelf_edit', { id, name, text }),
  shelfRemove: (ids: string[] | null) => invoke<void>('portal_shelf_remove', { ids }),
  shelfUndo: () => invoke<void>('portal_shelf_undo'),
  shelfCopy: (id: string) => invoke<void>('portal_shelf_copy', { id }),
  shelfReveal: (id: string) => invoke<string | null>('portal_shelf_reveal', { id }),
  setShelfShare: (peer: string, on: boolean) => invoke<void>('portal_set_shelf_share', { peer, on }),
  setPicture: (picture: Picture | null) => invoke<void>('portal_set_picture', { picture }),
  shelfPutMarked: (count: number) => invoke<void>('portal_shelf_put_marked', { count }),
  shelfPutReceived: (id: number) => invoke<void>('portal_shelf_put_received', { id }),
  saveClipImage: (id: number, name: string) => invoke<string>('portal_save_clip_image', { id, name }),
  dismissClipNotice: () => invoke<void>('portal_dismiss_clip_notice'),
  unqueueNext: (id: number) => invoke<void>('portal_unqueue_next', { id }),
  continueNext: (peer: string) => invoke<void>('portal_continue_next', { peer }),
  shortcut: () => invoke<CoreShortcut>('portal_shortcut'),
  setShortcut: (shortcut: string) => invoke<CoreShortcut>('portal_set_shortcut', { shortcut }),
}
