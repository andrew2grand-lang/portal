import { useState } from 'react'
import { Key } from '../../components/Key/Key'
import { APP_NAME } from '../../platform'
import { formatBytes, plural } from '../../engine/format'
import { payloadTitle } from '../../engine/words'
import { useLive, usePeer, usePortal } from '../../engine/context'
import type { Session } from '../../engine/types'
import s from './Stage.module.css'
import { Actions, Head, ItemList, LiveItemList, NextList, Pane, TransferProgress } from './parts'

/**
 * Cancel asks first, saying what stays and what goes: pressed by reflex on a
 * 12 GB transfer it would otherwise throw away everything that arrived.
 * Returns the <Actions> element itself (not a wrapper component) so Pane
 * docks it below the scroll area.
 */
function useCancelActions(session: Session, keepLabel: string) {
  const { dispatch } = usePortal()
  const other = usePeer(session.peer).name
  const [asking, setAsking] = useState(false)
  const out = session.direction === 'out'
  if (!asking) {
    return (
      <Actions>
        <Key onClick={() => setAsking(true)}>Cancel</Key>
      </Actions>
    )
  }
  const question = out
    ? `Stop sending? Your files here stay; the part on ${other} is removed.`
    : `Stop receiving? What arrived is removed; it stays on ${other}.`
  return (
    <Actions leading={<p className={s.ask}>{question}</p>}>
      <Key variant="ink" onClick={() => setAsking(false)}>
        {keepLabel}
      </Key>
      <Key onClick={() => dispatch({ type: 'cancel', id: session.id })}>{out ? 'Stop sending' : 'Stop receiving'}</Key>
    </Actions>
  )
}

/** One transfer, moving; the edge glow does the rest */
export function TransferView({ session }: { session: Session }) {
  const { world } = usePortal()
  const peer = usePeer(session.peer)
  const out = session.direction === 'out'
  const title = out ? `Sending to ${peer.name}` : `Receiving from ${peer.name}`
  const p = session.payload
  const cancel = useCancelActions(session, out ? 'Keep sending' : 'Keep receiving')
  const file = world.currentFile[session.id]

  // Too quick for a progress bar: the edge light says it moved
  if (session.brief) {
    return (
      <Pane>
        <Head title={title} sub={payloadTitle(p)} />
        <ItemList items={p.items} />
      </Pane>
    )
  }

  return (
    <Pane>
      <Head title={title} sub={payloadTitle(p)} />
      <TransferProgress
        id={session.id}
        total={p.totalBytes}
        link={session.linkAtEnd ?? peer.link ?? 'wifi'}
        stalled={session.linkNote === 'stalled'}
        peerName={peer.name}
        orb={out ? 'working' : 'weaving'}
      />
      {file && (
        <p className={s.current} title={file}>
          {file}
        </p>
      )}
      {/* three 7: nothing moving, but still connected: say what happens */}
      {session.linkNote === 'stalled' && (
        <p className={s.aside}>Nothing is moving right now. It carries on by itself, or pauses if {peer.name} goes away.</p>
      )}
      <NextList peer={session.peer} />
      <LiveItemList id={session.id} items={p.items} moving={session.direction === 'in' ? 'Receiving' : 'Sending'} />
      {world.queue.length > 0 && (
        <p className={s.aside}>{plural(world.queue.length, 'more item')} waiting in the send list.</p>
      )}
      {!out && session.autoAccepted && (
        <p className={s.aside}>Accepted automatically (Always accept).</p>
      )}
      {/* Already delivered: nothing left to cancel */}
      {!session.held && cancel}
    </Pane>
  )
}

/** The connection dropped: both sides keep what's done and wait for each
    other (Mission 03 §2). It continues by itself; Cancel removes what's kept. */
export function PausedView({ session }: { session: Session }) {
  if (session.stoppedHere) return <StoppedHere session={session} />
  return <PausedWaiting session={session} />
}

/** The app was quit (or crashed) partway: Continue or Cancel (Mission 05 §1.4) */
function StoppedHere({ session }: { session: Session }) {
  const { dispatch } = usePortal()
  const other = usePeer(session.peer)
  const out = session.direction === 'out'
  const p = session.payload
  const what = payloadTitle(p)
  const named = p.items.length === 1 ? `‘${what}’` : what
  const [asking, setAsking] = useState(false)
  return (
    <Pane>
      <Head
        title={out ? `Sending ${named} to ${other.name} stopped partway` : `Receiving ${named} from ${other.name} stopped partway`}
        sub={`${APP_NAME} closed before it finished. What’s done is kept for 24 hours.`}
      />
      {session.percentKnown !== false && (
        <TransferProgress id={session.id} total={p.totalBytes} link={session.linkAtEnd ?? 'wifi'} paused orb={out ? 'working' : 'weaving'} />
      )}
      {asking ? (
        <Actions leading={<p className={s.ask}>{out ? `Your files here stay; the part on ${other.name} is removed.` : `What arrived is removed; it stays on ${other.name}.`}</p>}>
          <Key variant="ink" onClick={() => setAsking(false)}>
            Keep
          </Key>
          <Key onClick={() => dispatch({ type: 'cancel', id: session.id })}>Cancel transfer</Key>
        </Actions>
      ) : (
        <Actions>
          <Key variant="ink" onClick={() => dispatch({ type: 'continue', id: session.id })}>
            Continue
          </Key>
          <Key onClick={() => setAsking(true)}>Cancel</Key>
        </Actions>
      )}
    </Pane>
  )
}

function PausedWaiting({ session }: { session: Session }) {
  const { progress } = useLive(session.id)
  const other = usePeer(session.peer)
  const out = session.direction === 'out'
  const p = session.payload
  const cancel = useCancelActions(session, 'Keep waiting')
  const arrived = formatBytes(progress.get() * p.totalBytes)
  // Connected again but not yet moving: say so, never "lost connection"
  const back = session.resuming || other.status === 'connected'
  const way = out ? `Sending to ${other.name}` : `Receiving from ${other.name}`
  const title = session.waitingThere ? `${way}: paused` : back ? `${other.name} is back` : `${way}: paused`
  return (
    <Pane>
      <Head
        title={title}
        sub={
          session.waitingThere
            ? `${APP_NAME} closed on ${other.name} before it finished. Press Continue there.`
            : session.resuming
              ? `Checking the ${arrived} already ${out ? 'there' : 'here'}, then ${out ? 'sending' : 'receiving'} carries on.`
              : back
                ? `${out ? 'Sending' : 'Receiving'} picks up where it stopped in a moment.`
                : `Lost connection. Is ${other.kind === 'mac' ? 'the Mac' : other.kind === 'pc' ? 'the PC' : 'it'} asleep or off the network? It continues by itself.`
        }
      />
      <TransferProgress
        id={session.id}
        total={p.totalBytes}
        link={session.linkAtEnd ?? 'wifi'}
        paused
        checking={session.resuming}
        orb={out ? 'working' : 'weaving'}
      />
      <NextList peer={session.peer} />
      <ItemList items={p.items} />
      {cancel}
    </Pane>
  )
}
