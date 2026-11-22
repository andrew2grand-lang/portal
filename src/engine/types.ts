import type { Picture } from '../window/picture/picture'
/** The OS this window runs on: its chrome and wording (never "which device") */
export type Side = 'mac' | 'win'
/** What a computer does with files from another one (Settings → Devices) */
export type Incoming = 'ask' | 'always' | 'never'
export type Mode = 'files' | 'project'
/** How the link is labelled: Wi-Fi, direct cable (offline), routed Ethernet, anything else */
export type Link = 'wifi' | 'cable' | 'ethernet' | 'other'
export type Theme = 'light' | 'dark'
/** Settings → Appearance */
export type Appearance = 'system' | 'light' | 'dark'

/** A computer: this one or another (Mission 04 §3: keyed by device id) */
export interface Device {
  id: string
  name: string
  os: string
  /** Short OS name: "Mac", "PC" */
  short: string
  /** How this machine refers to itself: "this Mac" */
  self: string
  /** The icon */
  kind: 'mac' | 'pc' | 'other'
  /** Its picture (Mission 05 §2.3); none: a colour from its id and its system's icon */
  picture?: Picture | null
}

/** Where this computer stands with another one */
export type DeviceStatus =
  /** Paired and connected */
  | 'connected'
  /** Paired, not connected yet: the core is still trying (and says how) */
  | 'connecting'
  /** Paired, and the core says it isn't reachable (after its grace period) */
  | 'unreachable'
  /** Paired, Disconnect was pressed here */
  | 'disconnected'
  /** Found on the network, not paired */
  | 'nearby'
  /** Runs another protocol version */
  | 'incompatible'
  /** It removed this computer: pair again */
  | 'revoked'

/** How connecting to a device is going (from the core) */
export interface ReachInfo {
  /** What it's doing, in plain words: "Trying the address it was last seen at" */
  doing: string
  /** The last failure, in plain words */
  lastFailure?: string
  attempts: number
}

/** The clipboard with one paired device */
export interface PeerClip {
  /** This computer agreed to sync with it */
  mine: boolean
  /** It agreed to sync with this computer */
  theirs: boolean
  /** It asks this computer to agree */
  asks: boolean
  /** Where its clipboard items go here */
  to: 'clipboard' | 'shelf'
}

export interface Peer extends Device {
  paired: boolean
  status: DeviceStatus
  link?: Link
  reach?: ReachInfo
  /** What this computer does with its files */
  incoming: Incoming
  /** What it does with this computer's files, once it has said */
  accepts?: Incoming
  /** Status incompatible: which computer needs the update */
  outdated?: 'peer' | 'self'
  /** Its address, for "MacBook Pro (192.168.1.20) wants to pair" */
  addr?: string
  clip?: PeerClip
  /** Sharing the Shelf with it (paired only) */
  shelf?: { mine: boolean; theirs: boolean; answered: boolean }
}

/** In-window panels that replace the stage */
export type Panel = 'settings' | 'connect' | 'doctor' | 'clipboard' | 'shelf' | 'add' | 'details' | 'help'

export type CheckStatus = 'pass' | 'fail' | 'likely' | 'unknown'

/** One line of the connection doctor */
export interface DoctorCheck {
  id: string
  status: CheckStatus
  title: string
  /** What to do about it, in plain words */
  fix?: string
  /** A settings page that fixes it */
  action?: { label: string; url: string }
}

/** Pairing by typed code (Mission 04 §2): the asker types, the other shows */
export type PairStep = 'asking' | 'asked' | 'code' | 'checking' | 'wrong' | 'paired'

export interface PairingState {
  /** The device being paired */
  peer: string
  /** Its address, for "MacBook Pro (192.168.1.20) wants to pair" */
  peerAddr: string
  /** This side asked to pair: its person types the code */
  mine: boolean
  step: PairStep
  /** The code to show, "048 213" (only on the computer that was asked) */
  code: string
  /** Wrong codes still allowed */
  triesLeft: number
  /** Set when pairing ended without trust */
  failed?: string
  /** When this step stops working (epoch ms): the code's countdown */
  expiresAt: number
}

export interface SettingsState {
  deviceName: string
  receiveDir: string
  skipJunk: boolean
  /** "A folder for each device": SeamDrop/From MacBook Pro/… */
  folderPerDevice: boolean
  /** Straight into a folder, in a SeamDrop folder, or a numbered folder each delivery */
  receiveMode: 'loose' | 'app_folder' | 'numbered'
  /** Keep the sender's dates (off: dated on arrival) */
  keepDates: boolean
  /** What changed about received files with the update, said once */
  receiveNote?: string
  /** Too many wrong pairing codes: requests are turned down until allowed again */
  pairingLocked: boolean
  /** Who typed the last wrong code, and when ("14:02") */
  pairingLockedBy?: string
  pairingLockedAt?: string
  /** For Send feedback */
  appVersion: string
  osVersion: string
  openAtLogin: boolean
  /** This machine's addresses, for typing into the other side */
  addresses: { label: string; address: string }[]
}

export interface FileItem {
  name: string
  kind: 'folder' | 'file'
  bytes: number
  count?: number
}

export interface Payload {
  items: FileItem[]
  totalBytes: number
}

export type Phase = 'blocked' | 'offer' | 'transfer' | 'paused' | 'done' | 'declined' | 'failed'
/** The receiver's "Files from <sender>" is set to Never */
export type BlockReason = 'not-accepted'
/** A line under the progress bar: the network dropped out */
export type LinkNote = 'stalled' | null

/** Files waiting for a busy device: "Next for Desk PC: 2 files" */
export interface NextItem {
  id: number
  peer: string
  /** What was dropped, by name */
  names: string[]
  folders: number
  /** Kept through a quit: waits for Continue */
  held: boolean
  /** The device's slot is taken by a paused transfer, not a moving one */
  afterPaused: boolean
}

/** One transfer with one device */
export interface Session {
  /** The core's transfer id (hex) */
  id: string
  /** The other device */
  peer: string
  direction: 'out' | 'in'
  payload: Payload
  phase: Phase
  block?: BlockReason
  autoAccepted: boolean
  linkNote: LinkNote
  /** Where the progress bar starts */
  startProgress: number
  /** A near-instant transfer: show the light and the landing, not a progress bar */
  brief?: boolean
  /** The core is already done; the window is finishing the arrival moment */
  held?: boolean
  elapsed?: number
  linkAtEnd?: Link
  /** Where the files landed and what had to change */
  result?: TransferResult
  /** Why it stopped */
  failure?: string
  /** Paused since (epoch ms), continues when the device is back */
  pausedSince?: number
  /** Paused, connected again, checking what arrived */
  resuming?: boolean
  /** Quit or crashed here partway: the window asks Continue or Cancel */
  stoppedHere?: boolean
  /** The other computer was quit or crashed partway: waiting for it */
  waitingThere?: boolean
  /** The percentage is known for sure (false: don't show one) */
  percentKnown?: boolean
  /** Stopped because Cancel was pressed on this computer */
  cancelled?: boolean
  /** Sender, stopped or cancelled: the same items can go again */
  canSendAgain?: boolean
  /** Receiver: the folder its files are (or will be) saved in */
  saveTo?: string
}

export interface TransferResult {
  /** The first files this computer ever received */
  firstArrival?: boolean
  savedTo?: string
  /** First received item, for "Show in Finder" */
  revealPath?: string
  /** One received file that's safe to Open (the core decides: never apps, scripts or archives) */
  openPath?: string
  files: number
  renamed: number
  skipped: number
  longPaths: number
  /** Already there after a pause, not sent again (left out of the speed) */
  keptBytes: number
  /** Which items were left out, and why (the first 50) */
  skippedItems: ListedItem[]
  /** Which names changed, to what, and why (the first 50) */
  renamedItems: ListedItem[]
  notes: string[]
}

/** An item a done screen names: "Docs/link.txt", "a link (shortcut): links aren’t sent" */
export interface ListedItem {
  path: string
  why: string
}

/** One line of the story Details tells (Mission 04 §1.6) */
export interface LogLine {
  id: number
  /** When it happened ("19:31:04"); the first of a run of repeats */
  time: string
  tag: string
  /** Plain words */
  text: string
  /** The technical part, after */
  detail?: string
  /** Neutral unless something is wrong and affects you (warn) or failed and needs you (err) */
  tone?: 'warn' | 'err' | 'dim'
  /** The same line this many times in a row */
  count: number
  /** When the last of the repeats happened ("19:31") */
  last?: string
}

export interface DevState {
  open: boolean
  theme: Theme
  reducedMotion: boolean
  /** Developer setting: show the Project · Paid tab (off) */
  projectTab: boolean
}

/** Something waiting in the send list: collected, then sent in one go */
export interface QueuedItem {
  path: string
  name: string
  /** null while it's being counted */
  kind: 'file' | 'folder' | null
  bytes?: number
  /** Files in a folder */
  count?: number
  /** Unreadable, or a link: it won't be sent */
  unusable?: boolean
}

/** Where the app is with updates (Settings → About and the notice) */
export type UpdateState =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'waiting'
  | 'installing'
  | 'failed'

export interface UpdateInfo {
  state: UpdateState
  /** The running version */
  current: string
  /** The version on offer, and what's new in it */
  version?: string
  notes?: string
  percent?: number
  /** Waiting: the device(s) the running transfer is with */
  peer?: string
  /** Failed: what happened, in plain words */
  message?: string
  /** Later was pressed for this version */
  dismissed: boolean
  /** Restart was pressed: the window shows the download and install */
  requested: boolean
  /** Install updates automatically */
  auto: boolean
  /** Get test builds */
  testBuilds: boolean
  checkedAt?: number
}

/** One clipboard item that arrived (the Shelf) */
export interface ClipItem {
  id: number
  from: string
  /** The device it came from */
  fromId?: string
  at: number
  kind: string
  summary: string
  preview: string
  copyable: boolean
  /** Also put on this computer's clipboard (false: Shelf only) */
  pasted: boolean
  /** Sent as private on purpose: no preview; marked private when copied */
  private: boolean
  /** An image's small preview (data URL) */
  thumb?: string
}

/** One item on the Shelf (Mission 05 §3.2) */
export interface ShelfItem {
  id: string
  /** "Vercel key", or empty */
  name: string
  /** The first line; undefined for a private item (its text stays in the core) */
  preview?: string
  private: boolean
  /** Where it was added */
  from: string
  fromName: string
  mine: boolean
  at: number
}

/** A copy that wasn't synced, said once (Mission 05 §3.1) */
export interface ClipNotice {
  id: number
  kind: 'private' | 'tooBig' | 'unreadable'
  /** The clipboard change it's about (Put on Shelf reads only that one) */
  count: number
  at: number
}

/** The shared clipboard */
export interface ClipboardInfo {
  /** Copies sync with at least one device right now */
  sync: boolean
  /** Paused here until then (epoch ms; 0: until SeamDrop restarts) */
  paused?: number
  /** Accelerator form, e.g. "Control+Alt+Super+C" */
  shortcut: string
  defaultShortcut: string
  shortcutError?: string
  items: ClipItem[]
  /** What the last Send clipboard said. With files: they were on the
      clipboard, offered as a transfer ("2 files on the clipboard · Send them
      to Desk PC"), not sent */
  last?: { ok: boolean; text: string; at: number; files?: string[] }
  /** The latest copy that wasn't synced */
  notice?: ClipNotice
}

export interface World {
  /** The core has answered (until then the stage stays empty: no flash of
      "Looking for your other computer" at start) */
  ready: boolean
  /** This computer */
  me: Device
  /** Every other device: paired first, then nearby */
  peers: Peer[]
  /** Where files and the clipboard go: the remembered choice (device ids) */
  target: string[]
  /** The device single-device screens talk about: the one being paired, the
      only paired one, else the first that matters */
  focus?: Peer
  /** The shared clipboard (absent until the core answers) */
  clipboard?: ClipboardInfo
  /** Updates (absent until the shell answers) */
  update?: UpdateInfo
  /** When the search for other computers (re)started, epoch ms */
  searchStartedAt: number
  panel: Panel | null
  /** Help opened from a "How … works" link: the section to show first */
  helpTopic?: string
  pairing: PairingState | null
  settings: SettingsState
  doctor: DoctorCheck[]
  connect: { busy: boolean; error?: string }
  /** The doctor's firewall "Fix it" (running, or what it said) */
  fix?: { busy: boolean; ok?: boolean; text?: string }
  /** Files are being dragged over the window (native drag events) */
  dragOver: boolean
  /** The file moving right now, per transfer */
  currentFile: Record<string, string>
  /** Files and folders collected for the next send */
  queue: QueuedItem[]
  /** What goes next to a busy device, earliest first (Mission 05 §1.5) */
  next: NextItem[]
  /** The Shelf, newest first (Mission 05 §3.2) */
  shelf: ShelfItem[]
  /** A private Shelf item revealed for a few seconds (memory only) */
  revealed?: { id: string; text: string }
  /** Undo after a removal from the Shelf, until then (epoch ms) */
  shelfUndoUntil?: number
  /** Devices whose pairing request was ignored here (this session) */
  ignored: string[]
  mode: Mode
  /** Every transfer on screen, oldest first */
  sessions: Session[]
  /** What a send that didn't start said, per device (shown until dismissed) */
  sendErrors: string[]
  logs: LogLine[]
  dev: DevState
  appearance: Appearance
}

export type Action =
  | { type: 'accept'; id: string; always?: boolean }
  | { type: 'decline'; id: string }
  | { type: 'cancel'; id: string }
  /** A transfer that stopped partway here (the app was quit) goes on */
  | { type: 'continue'; id: string }
  /** The Shelf (Mission 05 §3.2) */
  | { type: 'shelfAdd'; name: string; text: string; private: boolean }
  | { type: 'shelfRename'; id: string; name: string }
  | { type: 'shelfRemove'; ids: string[] | null }
  | { type: 'shelfUndo' }
  | { type: 'shelfCopy'; id: string }
  | { type: 'shelfReveal'; id: string }
  | { type: 'setShelfShare'; id: string; on: boolean }
  /** Put on Shelf: from a not-synced private copy, or a received text */
  | { type: 'shelfPutMarked'; count: number }
  | { type: 'shelfPutReceived'; id: number }
  | { type: 'saveClipImage'; id: number }
  | { type: 'dismissClipNotice' }
  /** ✕ on files waiting for a busy device */
  | { type: 'unqueueNext'; id: number }
  /** Continue on what waited through a quit */
  | { type: 'continueNext'; peer: string }
  /** The same items again, after a stop or a cancel */
  | { type: 'sendAgain'; id: string }
  | { type: 'dismiss'; id: string }
  | { type: 'dismissErrors' }
  | { type: 'setMode'; mode: Mode }
  /** Where files and the clipboard go (remembered) */
  | { type: 'setTarget'; ids: string[] }
  | { type: 'dev'; patch: Partial<DevState> }
  | { type: 'openPanel'; panel: Panel | null; topic?: string }
  | { type: 'setPicture'; picture: Picture | null }
  | { type: 'pair'; id: string }
  /** Show code, Ignore, Cancel, OK, Allow pairing again */
  | { type: 'pairing'; action: 'showCode' | 'ignore' | 'cancel' | 'dismiss' | 'allowAgain' }
  /** The code as typed (the asker) */
  | { type: 'pairCode'; code: string }
  | { type: 'connect'; address: string }
  | { type: 'retrySearch' }
  | {
      type: 'saveSettings'
      patch: Partial<Pick<SettingsState, 'deviceName' | 'skipJunk' | 'folderPerDevice' | 'receiveMode' | 'keepDates'>> & {
        autoUpdate?: boolean
        testBuilds?: boolean
        clipboardSync?: boolean
        clearReceiveNote?: boolean
      }
    }
  | { type: 'chooseReceiveDir' }
  /** Opens the received-files folder */
  | { type: 'openReceived' }
  | { type: 'setOpenAtLogin'; value: boolean }
  | { type: 'setAppearance'; value: Appearance }
  | { type: 'renamePeer'; id: string; name: string }
  | { type: 'removePeer'; id: string }
  | { type: 'setPeerIncoming'; id: string; value: Incoming }
  /** Disconnect (false) or Connect (true) */
  | { type: 'setConnected'; id: string; value: boolean }
  | { type: 'pickFiles'; folders?: boolean }
  /* The send list: collect, then send in one go */
  | { type: 'queuePaths'; paths: string[] }
  | { type: 'unqueue'; path: string }
  | { type: 'clearQueue' }
  | { type: 'sendQueue' }
  | { type: 'reveal'; path?: string }
  /** Open a received document, image or media file (Mission 05 §1.6) */
  | { type: 'openFile'; path: string }
  | { type: 'openUrl'; url: string }
  /** The doctor's "Fix it" for Windows Firewall (asks for an administrator) */
  | { type: 'fixFirewall' }
  /* Clipboard */
  | { type: 'sendClipboard'; ids?: string[] }
  /** The Shelf's text box */
  | { type: 'sendText'; ids?: string[]; text: string; private?: boolean }
  | { type: 'removeClip'; id: number }
  /** Files from the clipboard, as a normal transfer to the ticked devices */
  | { type: 'sendPaths'; paths: string[] }
  | { type: 'copyClip'; id: number }
  /* Details */
  | { type: 'copyTechnicalLog' }
  | { type: 'saveTechnicalLog' }
  /** Settings → About and Details (Mission 04 §5) */
  | { type: 'sendFeedback'; withLog: boolean }
  | { type: 'clearClips' }
  | { type: 'setShortcut'; shortcut: string }
  | { type: 'setClipSync'; id: string; on: boolean }
  | { type: 'clipSyncOff' }
  | { type: 'pauseClipSync'; minutes?: number }
  | { type: 'resumeClipSync' }
  | { type: 'setClipTo'; id: string; to: 'clipboard' | 'shelf' }
  /* Updates */
  | { type: 'checkUpdates' }
  | { type: 'restartToUpdate' }
  | { type: 'updateLater' }
