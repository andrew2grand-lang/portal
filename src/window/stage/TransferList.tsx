import { useState } from 'react'
import { motion, useTransform } from 'motion/react'
import { ArrowClockwiseIcon, FolderOpenIcon, PauseIcon, XIcon } from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { formatBytes } from '../../engine/format'
import { useLive, usePeer, usePortal } from '../../engine/context'
import type { Session } from '../../engine/types'
import { payloadTitle } from '../../engine/words'
import { DeviceIcon } from '../PortalWindow'
import { useSide } from '../WindowContext'
import { listTitle } from '../../engine/views'
import { revealLabel, shortFolder } from './where'
import s from './Stage.module.css'
import { Head, NextList, Pane, SendMore } from './parts'

/** Several transfers at once (Mission 04 §3): one row each, with its own
    progress, pause and keys. Across devices they run side by side. */
export function TransferList({ sessions, more }: { sessions: Session[]; more?: boolean }) {
  return (
    <Pane>
      <Head title={listTitle(sessions)} />
      <ul className={s.rows} aria-label="Transfers">
        {sessions.map((x) => (
          <TransferRow key={x.id} session={x} />
        ))}
      </ul>
      <NextList />
      {more && <SendMore />}
    </Pane>
  )
}

function titleOf(x: Session, name: string): string {
  const out = x.direction === 'out'
  switch (x.phase) {
    case 'offer':
      return out ? `Waiting for ${name} to accept` : `${name} wants to send`
    case 'transfer':
      return out ? `Sending to ${name}` : `Receiving from ${name}`
    case 'paused':
      if (x.stoppedHere) return out ? `Stopped partway: sending to ${name}` : `Stopped partway: receiving from ${name}`
      if (x.waitingThere) return `Paused: waiting for ${name} to continue`
      return x.resuming ? `${name} is back, checking` : `Paused: waiting for ${name}`
    case 'done':
      return out ? `Sent to ${name}` : `Received from ${name}`
    case 'declined':
      return `${name} declined`
    case 'blocked':
      return `${name} doesn’t accept files from this computer`
    case 'failed':
      return x.cancelled ? `Cancelled: ${name}` : `Stopped: ${name}`
  }
}

function TransferRow({ session: x }: { session: Session }) {
  const side = useSide()
  const { world, dispatch } = usePortal()
  const peer = usePeer(x.peer)
  const live = x.phase === 'transfer' || x.phase === 'paused'
  // "Stop?" belongs to a running transfer: once it has stopped, the row says so
  const [asked, setAsking] = useState(false)
  const asking = asked && live
  const dismiss = (
    <Key size="sm" icon={<XIcon />} aria-label="Clear" title="Clear from the list" onClick={() => dispatch({ type: 'dismiss', id: x.id })} />
  )
  let keys
  if (asking) {
    keys = (
      <>
        <Key size="sm" onClick={() => dispatch({ type: 'cancel', id: x.id })}>
          Stop
        </Key>
        <Key size="sm" variant="ink" onClick={() => setAsking(false)}>
          Keep
        </Key>
      </>
    )
  } else if (x.phase === 'offer' && x.direction === 'in') {
    keys = (
      <>
        <Key size="sm" onClick={() => dispatch({ type: 'decline', id: x.id })}>
          Decline
        </Key>
        <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'accept', id: x.id })}>
          Accept
        </Key>
      </>
    )
  } else if (x.phase === 'paused' && x.stoppedHere) {
    keys = (
      <>
        <Key size="sm" onClick={() => setAsking(true)}>
          Cancel
        </Key>
        <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'continue', id: x.id })}>
          Continue
        </Key>
      </>
    )
  } else if (x.phase === 'transfer' || x.phase === 'paused') {
    keys = (
      <Key size="sm" onClick={() => setAsking(true)}>
        Cancel
      </Key>
    )
  } else if (x.phase === 'offer' || x.phase === 'blocked') {
    keys = (
      <Key size="sm" onClick={() => dispatch({ type: 'cancel', id: x.id })}>
        Cancel
      </Key>
    )
  } else if (x.phase === 'done' && x.direction === 'in') {
    keys = (
      <>
        <Key size="sm" icon={<FolderOpenIcon />} onClick={() => dispatch({ type: 'reveal', path: x.result?.revealPath ?? x.result?.savedTo })}>
          {revealLabel[side]}
        </Key>
        {dismiss}
      </>
    )
  } else if (x.phase === 'failed' && x.canSendAgain) {
    keys = (
      <>
        <Key size="sm" icon={<ArrowClockwiseIcon />} onClick={() => dispatch({ type: 'sendAgain', id: x.id })}>
          Send again
        </Key>
        {dismiss}
      </>
    )
  } else {
    keys = dismiss
  }
  const what = `${payloadTitle(x.payload)} · ${formatBytes(x.payload.totalBytes)}`
  // An offer says where it would go; a finished one that it was checked
  const meta =
    x.phase === 'offer' && x.direction === 'in'
      ? `${what} · saved in ${shortFolder(x.saveTo ?? world.settings.receiveDir, side)} if you accept`
      : x.phase === 'done'
        ? `${what} · every file checked`
        : what
  const question =
    x.direction === 'out'
      ? `Stop sending? The part that arrived on ${peer.name} is removed.`
      : `Stop receiving? What arrived from ${peer.name} is removed.`
  return (
    <li className={s.row} data-phase={x.phase}>
      <span className={s.itemTile} aria-hidden>
        <DeviceIcon device={peer} />
      </span>
      <div className={s.rowText}>
        <p className={s.rowTitle}>{titleOf(x, peer.name)}</p>
        <p className={s.rowMeta}>
          {asking ? question : x.phase === 'failed' && x.failure ? x.failure : meta}
        </p>
        {live && <RowProgress session={x} name={peer.name} />}
      </div>
      <div className={s.rowKeys}>{keys}</div>
    </li>
  )
}

/** A thin bar, the percentage and time left (or why it's still) */
function RowProgress({ session: x, name }: { session: Session; name: string }) {
  const { progress, eta } = useLive(x.id)
  const width = useTransform(progress, (p) => `inset(0 ${((1 - p) * 100).toFixed(2)}% 0 0 round 999px)`)
  const percent = useTransform(progress, (p) => `${Math.floor(p * 100)}%`)
  const still = x.phase === 'paused' || x.linkNote === 'stalled'
  return (
    <div className={s.rowProgress}>
      <div className={s.rowBar} role="progressbar" aria-label={`${titleOf(x, '')} progress`}>
        <motion.span className={s.fill} style={{ clipPath: width }} />
      </div>
      <span className={s.rowNumbers}>
        {still && <PauseIcon weight="fill" aria-hidden />}
        <motion.span>{percent}</motion.span>
        <span aria-hidden>·</span>
        {x.phase === 'paused' ? <span>{x.resuming ? 'Checking…' : 'Paused'}</span> : x.linkNote === 'stalled' ? <span>{`Waiting for ${name}…`}</span> : <motion.span>{eta}</motion.span>}
      </span>
    </div>
  )
}
