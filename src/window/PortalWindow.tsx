import { AnimatePresence, motion } from 'motion/react'
import {
  ArrowsLeftRightIcon,
  CheckIcon,
  ArchiveIcon,
  ClipboardTextIcon,
  DesktopIcon,
  DevicesIcon,
  GearSixIcon,
  LaptopIcon,
  MagnifyingGlassIcon,
  NetworkIcon,
  PlusIcon,
  WifiHighIcon,
} from '@phosphor-icons/react'
import { EdgeGlow } from '../components/EdgeGlow/EdgeGlow'
import { Key } from '../components/Key/Key'
import { ModeSwitch } from '../components/ModeSwitch/ModeSwitch'
import { usePortal } from '../engine/context'
import { linkLabel } from '../engine/format'
import type { Device, Link, Peer, Side, World } from '../engine/types'
import { arrivalFor, glowFor } from '../engine/views'
import { names } from '../engine/words'
import { APP_NAME } from '../platform'
import { spring } from '../styles/motion'
import { AppWindow } from './AppWindow'
import { Foot } from './Details'
import { Stage } from './stage/Stage'
import { ClipSyncAsk, ReceiveNote, ShelfShareAsk } from './ClipSyncAsk'
import { UpdateNotice } from './UpdateNotice'
import { DevicePicture } from './picture/DevicePicture'
import { MiniStage } from './MiniWindow'
import { useMini } from './useMini'
import { ClipNoticeCard } from './stage/Clipboard'
import { PairingLocked } from './stage/Connection'
import { WindowContext } from './WindowContext'
import s from './PortalWindow.module.css'

/** Everything inside the app window. `side` is the OS it runs on (its chrome). */
export function PortalWindow({ side, native = false }: { side: Side; native?: boolean }) {
  const { world, dispatch } = usePortal()
  const settingsOpen = world.panel === 'settings'
  const clipOpen = world.panel === 'clipboard'
  const shelfOpen = world.panel === 'shelf'
  const paired = world.peers.filter((p) => p.paired)
  const syncing = paired.filter((p) => p.clip?.mine)
  const active = syncing.filter((p) => p.clip?.theirs).map((p) => p.name)
  const mini = useMini()
  // Agreed, but none of them is here: copies made now aren't sent
  const allAway = active.length > 0 && syncing.filter((p) => p.clip?.theirs).every((p) => p.status !== 'connected')

  return (
    <WindowContext value={side}>
      <AppWindow
        side={side}
        native={native}
        accessory={
          <span className={s.titleKeys}>
            {/* One job per place (Mission 05 §3): Clipboard and Shelf from
                every screen, also while files move */}
            {paired.length > 0 && (
              <Key
                size="sm"
                variant={clipOpen ? 'lit' : 'plain'}
                icon={<ClipboardTextIcon />}
                aria-label="Clipboard"
                title="Clipboard: send it, sync it, what arrived"
                aria-pressed={clipOpen}
                onClick={() => dispatch({ type: 'openPanel', panel: clipOpen ? null : 'clipboard' })}
              >
                {/* Words beside the icons where the window has room (review 05) */}
                <span className={s.titleWord}>Clipboard</span>
              </Key>
            )}
            <Key
              size="sm"
              variant={shelfOpen ? 'lit' : 'plain'}
              icon={<ArchiveIcon />}
              aria-label="Shelf"
              title="Shelf: things you keep"
              aria-pressed={shelfOpen}
              onClick={() => dispatch({ type: 'openPanel', panel: shelfOpen ? null : 'shelf' })}
            >
              <span className={s.titleWord}>Shelf</span>
            </Key>
            <Key
              size="sm"
              variant={settingsOpen ? 'lit' : 'plain'}
              icon={<GearSixIcon />}
              aria-label="Settings"
              aria-pressed={settingsOpen}
              onClick={() => dispatch({ type: 'openPanel', panel: settingsOpen ? null : 'settings' })}
            />
          </span>
        }
        glow={
          <EdgeGlow
            mode={glowFor(world)}
            link={world.sessions.find((x) => x.phase === 'transfer')?.linkAtEnd ?? 'wifi'}
            arrival={arrivalFor(world)}
          />
        }
      >
        {/* Mission 05 §5: a really small window is just the drop icon */}
        {mini ? (
          <MiniStage />
        ) : (
        <>
        {paired.length > 1 ? (
          <DevicesHeader peers={paired} />
        ) : (
          // Nothing paired: no computer singled out in the header (Mission 05
          // §1.8), unless one is being paired with or was removed
          <PeerHeader peer={paired.length || world.pairing || world.focus?.status === 'revoked' ? world.focus : undefined} />
        )}
        {(world.dev.projectTab || syncing.length > 0) && (
          <div className={s.controls}>
            {world.dev.projectTab ? (
              <ModeSwitch
                id={side}
                value={world.mode}
                onChange={(mode) => dispatch({ type: 'setMode', mode })}
                locked={world.sessions.some((x) => x.phase === 'transfer' || x.phase === 'offer')}
              />
            ) : (
              <span />
            )}
            {syncing.length > 0 && (
              <button
                type="button"
                className={s.link}
                data-sync=""
                title={
                  active.length
                    ? `Copies sync with ${names(active)}. Open the clipboard to pause or turn it off.`
                    : 'Turned on here. Nothing syncs until the other side turns it on too.'
                }
                onClick={() => dispatch({ type: 'openPanel', panel: 'clipboard' })}
              >
                <span className={s.linkInner}>
                  <ArrowsLeftRightIcon weight="bold" />
                  {world.clipboard?.paused !== undefined
                    ? 'Clipboard sync paused'
                    : allAway
                      ? 'Clipboard sync waits'
                      : active.length
                      ? paired.length > 1
                        ? `Clipboard sync: ${active.length === 1 ? active[0] : `${active.length} devices`}`
                        : 'Clipboard sync on'
                      : `Sync: waiting for ${names(syncing.map((p) => p.name))}`}
                </span>
              </button>
            )}
          </div>
        )}
        {/* Between the header and the stage: anything that asks for a
            decision, then where things go. When the window is short this
            part scrolls, so the stage keeps room to work in; a decision
            comes first so its keys stay in view (Mission 04 §1.5). */}
        <div className={s.top}>
          {/* Someone may be guessing codes: said where it's seen (Settings and Pair another show it themselves) */}
          {world.panel !== 'settings' && world.panel !== 'add' && <PairingLocked />}
          <UpdateNotice />
          <ClipSyncAsk />
          <ShelfShareAsk />
          <ReceiveNote />
          {/* A copy that wasn't synced, said in the window, not only in Clipboard (review 05) */}
          {world.panel !== 'clipboard' && <ClipNoticeCard />}
          {/* Where things go: next to the drop zone and transfers, not over a panel */}
          {paired.length > 1 && (!world.panel || world.panel === 'clipboard') && !world.pairing && <DeviceStrip peers={paired} />}
        </div>
        <Stage />
        <Foot />
        </>
        )}
      </AppWindow>
    </WindowContext>
  )
}

const statusWords: Record<Peer['status'], string> = {
  connected: 'Connected',
  connecting: 'Connecting…',
  unreachable: 'Not reachable',
  disconnected: 'Disconnected',
  nearby: 'Not paired yet',
  incompatible: 'Older version',
  revoked: 'Removed this computer',
}

/** A device as it's shown everywhere: its picture (Mission 05 §2.3), or
    without an id yet, its system's icon */
export function DeviceIcon({ device, ...rest }: { device?: Pick<Device, 'kind'> & Partial<Pick<Device, 'id' | 'picture'>>; weight?: 'light' | 'regular' }) {
  if (device?.id) return <DevicePicture device={{ id: device.id, kind: device.kind }} picture={device.picture} />
  return device?.kind === 'mac' ? <LaptopIcon weight="light" {...rest} /> : <DesktopIcon weight="light" {...rest} />
}

/** One device: name, OS, connection and link type (with one device, as before) */
export function PeerHeader({ peer }: { peer?: Peer }) {
  const { world } = usePortal()
  const searching = !peer
  const pairing = world.pairing && peer && world.pairing.peer === peer.id
  // Mid-transfer and nothing moving: not "Connected"
  const reconnecting = peer?.status === 'connected' && world.sessions.some((x) => x.peer === peer.id && x.phase === 'transfer' && x.linkNote === 'stalled')
  const nearby = world.peers.filter((p) => !p.paired).length
  const meta = !peer
    ? nearby
      ? `${nearby} nearby`
      : 'Searching this network'
    : `${peer.os} · ${
        pairing
          ? world.pairing?.step === 'paired'
            ? 'Paired'
            : world.pairing?.failed
              ? 'Not paired'
              : 'Pairing'
          : reconnecting
            ? 'Reconnecting…'
            : peer.status === 'incompatible' && peer.outdated === 'self'
              ? 'Newer version'
              : statusWords[peer.status]
      }`
  return (
    <div className={s.peer} data-presence={peer?.status ?? 'searching'}>
      <span className={s.peerTile} aria-hidden>
        {searching ? <MagnifyingGlassIcon weight="light" /> : <DeviceIcon device={peer} />}
      </span>
      <div className={s.peerText}>
        <p className={s.peerName}>{searching ? (nearby ? 'Nothing paired yet' : 'No computer yet') : peer.name}</p>
        <p className={s.peerMeta}>{meta}</p>
      </div>
      {peer?.status === 'connected' && !reconnecting && peer.link && <LinkPill link={peer.link} />}
    </div>
  )
}

/** Several devices: how many, and how many are connected */
function DevicesHeader({ peers }: { peers: Peer[] }) {
  const connected = peers.filter((p) => p.status === 'connected').length
  const connecting = peers.filter((p) => p.status === 'connecting').length
  const parts = [
    `${connected} connected`,
    connecting ? `${connecting} connecting` : '',
    peers.length - connected - connecting ? `${peers.length - connected - connecting} not reachable` : '',
  ].filter(Boolean)
  return (
    <div className={s.peer} data-presence={connected ? 'connected' : 'connecting'}>
      <span className={s.peerTile} aria-hidden>
        <DevicesIcon weight="light" />
      </span>
      <div className={s.peerText}>
        <p className={s.peerName}>{peers.length} devices</p>
        <p className={s.peerMeta}>{parts.join(' · ')}</p>
      </div>
    </div>
  )
}

/** Every paired device, with its status; ticked ones are where files and the
    clipboard go (remembered; the shortcut and the menu bar use it too) */
/** A tile's second line while it's busy, always what that computer is
    doing (review 05: "Desk PC · Sending" read backwards): "Getting your
    files", "Sending you files", "Wants to send", "Asked to accept", "Paused" */
function activity(w: World, id: string): string | undefined {
  const x = w.sessions.find((s) => s.peer === id && (s.phase === 'transfer' || s.phase === 'paused' || s.phase === 'offer'))
  if (!x) return undefined
  if (x.phase === 'paused') return 'Paused'
  if (x.phase === 'offer') return x.direction === 'in' ? 'Wants to send' : 'Asked to accept'
  return x.direction === 'out' ? 'Getting your files' : 'Sending you files'
}

function DeviceStrip({ peers }: { peers: Peer[] }) {
  const { world, dispatch } = usePortal()
  const away = peers.filter((p) => p.status === 'unreachable')
  const toggle = (id: string) => {
    const on = world.target.includes(id)
    const next = on ? world.target.filter((t) => t !== id) : [...world.target, id]
    if (next.length) dispatch({ type: 'setTarget', ids: next })
  }
  return (
    <div className={s.stripArea}>
    <div className={s.stripRow}>
    <div className={s.strip} role="group" aria-label="Where files and the clipboard go">
      {peers.map((p) => {
        const on = world.target.includes(p.id)
        // What it's doing, when it's busy; else how it's connected
        const how = activity(world, p.id) ?? (p.status === 'connected' && p.link ? linkLabel[p.link] : statusWords[p.status])
        return (
          <button
            key={p.id}
            type="button"
            role="checkbox"
            aria-checked={on}
            className={s.device}
            data-on={on ? '' : undefined}
            data-status={p.status}
            title={`${p.name}: ${how}. ${on ? 'Files and the clipboard go there. Click to leave it out.' : 'Click to send there too.'}`}
            onClick={() => toggle(p.id)}
          >
            <span className={s.deviceTick} aria-hidden>
              {on && <CheckIcon weight="bold" />}
            </span>
            <DeviceIcon device={p} />
            <span className={s.deviceText}>
              <span className={s.deviceName}>{p.name}</span>
              <span className={s.deviceStatus}>
                <span className={s.dot} aria-hidden />
                {how}
              </span>
            </span>
          </button>
        )
      })}
    </div>
      {/* Mission 05 §2.2: add a computer from here, not only in Settings */}
      <span className={s.stripAdd}>
        <Key size="sm" icon={<PlusIcon />} aria-label="Pair another computer" title="Pair another computer" onClick={() => dispatch({ type: 'openPanel', panel: 'add' })} />
      </span>
    </div>
      {away.length > 0 && (
        <p className={s.stripNote}>
          {names(away.map((p) => p.name))} {away.length > 1 ? 'aren’t' : 'isn’t'} reachable. Is it on and awake, with {APP_NAME} open? It connects by itself.
        </p>
      )}
    </div>
  )
}

export function LinkPill({ link }: { link: Link }) {
  return (
    <motion.span layout transition={spring.ui} className={s.link} data-link={link}>
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.span
          key={link}
          className={s.linkInner}
          initial={{ opacity: 0, y: 6, filter: 'blur(3px)' }}
          animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
          exit={{ opacity: 0, y: -6, filter: 'blur(3px)' }}
          transition={spring.ui}
        >
          {link === 'cable' || link === 'ethernet' ? <NetworkIcon /> : <WifiHighIcon />}
          {linkLabel[link]}
        </motion.span>
      </AnimatePresence>
    </motion.span>
  )
}
