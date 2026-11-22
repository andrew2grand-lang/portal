import { Checkbox } from '../../components/Checkbox/Checkbox'
import { Key } from '../../components/Key/Key'
import { formatBytes, plural } from '../../engine/format'
import { payloadTitle } from '../../engine/words'
import { usePeer, usePortal } from '../../engine/context'
import type { Payload, Session } from '../../engine/types'
import { useSide } from '../WindowContext'
import s from './Stage.module.css'
import { Actions, Head, ItemList, Pane, Waiting } from './parts'
import { shortFolder } from './where'

function summary(p: Payload) {
  return `${plural(p.items.length, 'item')}, ${formatBytes(p.totalBytes)}.`
}

/** Receiver: someone wants to send. Says where the files will go. */
export function OfferInView({ session }: { session: Session }) {
  const side = useSide()
  const { world, dispatch } = usePortal()
  const sender = usePeer(session.peer)
  const p = session.payload
  // Its own folder when "A folder for each device" is on
  const folder = shortFolder(session.saveTo ?? world.settings.receiveDir, side)
  return (
    <Pane>
      <Head title={`${sender.name} wants to send`} sub={summary(p)} />
      <ItemList items={p.items} />
      {folder && <p className={s.aside}>Saved in {folder} if you accept.</p>}
      <Checkbox
        checked={sender.incoming === 'always'}
        onChange={(value) => dispatch({ type: 'setPeerIncoming', id: sender.id, value: value ? 'always' : 'ask' })}
      >
        Always accept from {sender.name}
      </Checkbox>
      <Actions>
        <Key variant="ink" onClick={() => dispatch({ type: 'accept', id: session.id })}>
          Accept
        </Key>
        <Key onClick={() => dispatch({ type: 'decline', id: session.id })}>Decline</Key>
      </Actions>
    </Pane>
  )
}

/** Sender: waiting for the other side */
export function OfferOutView({ session }: { session: Session }) {
  const { dispatch } = usePortal()
  const peer = usePeer(session.peer)
  const p = session.payload
  return (
    <Pane>
      <Head title={`Waiting for ${peer.name} to accept`} sub={payloadTitle(p)} />
      <ItemList items={p.items} dim />
      <Waiting>Asked {peer.name} just now</Waiting>
      <Actions>
        <Key onClick={() => dispatch({ type: 'cancel', id: session.id })}>Cancel</Key>
      </Actions>
    </Pane>
  )
}
