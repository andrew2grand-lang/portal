import { motion } from 'motion/react'
import { ArrowsLeftRightIcon, ArrowUpRightIcon, FilesIcon, XIcon } from '@phosphor-icons/react'
import { Key } from '../components/Key/Key'
import { useLive, usePortal } from '../engine/context'
import { names } from '../engine/words'
import { DeviceIcon } from './PortalWindow'
import s from './MiniWindow.module.css'

/** The whole window in mini mode: the drop icon (a ring shows a transfer's
    progress around it), where files go, and Send clipboard. Dropping still
    works; anything that needs room asks for a bigger window. */
export function MiniStage() {
  const { world, dispatch } = usePortal()
  const paired = world.peers.filter((p) => p.paired)
  const to = paired.filter((p) => world.target.includes(p.id))
  const target = to[0] ?? paired[0]
  const moving = world.sessions.find((x) => x.phase === 'transfer' || x.phase === 'paused')
  const offer = world.sessions.find((x) => x.phase === 'offer' && x.direction === 'in')

  if (world.panel) {
    return (
      <Note text="Make the window bigger to see this.">
        <Key size="sm" onClick={() => dispatch({ type: 'openPanel', panel: null })}>
          Close
        </Key>
      </Note>
    )
  }
  if (!paired.length || world.pairing) return <Note text="Make the window bigger to pair." />
  if (offer) {
    const from = world.peers.find((p) => p.id === offer.peer)?.name ?? 'A computer'
    return (
      <Note text={`${from} wants to send ${offer.payload.items.length === 1 ? '1 item' : `${offer.payload.items.length} items`}.`}>
        <Key size="sm" onClick={() => dispatch({ type: 'decline', id: offer.id })}>
          Decline
        </Key>
        <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'accept', id: offer.id })}>
          Accept
        </Key>
      </Note>
    )
  }
  if (world.queue.length) {
    const ready = world.queue.filter((q) => !q.unusable).length
    return (
      <Note text={`${ready === 1 ? '1 item' : `${ready} items`} to ${to.length ? names(to.map((p) => p.name)) : 'nowhere yet'}.`}>
        <Key size="sm" icon={<XIcon />} aria-label="Clear" title="Clear" onClick={() => dispatch({ type: 'clearQueue' })} />
        <Key size="sm" variant="ink" icon={<ArrowUpRightIcon />} disabled={!ready || !to.length} onClick={() => dispatch({ type: 'sendQueue' })}>
          Send
        </Key>
      </Note>
    )
  }
  return (
    <div className={s.mini}>
      <button
        type="button"
        className={s.drop}
        data-over={world.dragOver ? '' : undefined}
        data-paused={moving?.phase === 'paused' ? '' : undefined}
        aria-label="Add files"
        title="Drop files here, or click to add them"
        onClick={() => dispatch({ type: 'pickFiles' })}
      >
        {moving && <Ring id={moving.id} />}
        <FilesIcon weight="light" aria-hidden />
      </button>
      <div className={s.row}>
        {target && (
          <span className={s.target} title={`Files go to ${names((to.length ? to : [target]).map((p) => p.name))}`}>
            <DeviceIcon device={target} />
          </span>
        )}
        <Key size="sm" icon={<ArrowsLeftRightIcon />} aria-label="Send clipboard" title="Send clipboard" onClick={() => dispatch({ type: 'sendClipboard' })} />
      </div>
    </div>
  )
}

/** A transfer's progress, drawn around the drop icon */
function Ring({ id }: { id: string }) {
  const { progress } = useLive(id)
  return (
    <svg className={s.ring} viewBox="0 0 100 100" aria-hidden>
      <circle className={s.track} cx="50" cy="50" r="46" />
      <motion.circle className={s.arc} cx="50" cy="50" r="46" style={{ pathLength: progress }} />
    </svg>
  )
}

function Note({ text, children }: { text: string; children?: React.ReactNode }) {
  return (
    <div className={s.note}>
      <p>{text}</p>
      {children && <div className={s.keys}>{children}</div>}
    </div>
  )
}
