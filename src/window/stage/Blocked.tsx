import { ProhibitIcon } from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { usePeer, usePortal } from '../../engine/context'
import type { Session } from '../../engine/types'
import { Actions, Head, ItemList, Pane, StateTile, Waiting } from './parts'

/** Sender: the other computer's "Files from <this one>" is set to Never.
    Names the computer and the exact setting to change; nothing was sent,
    and it sends by itself once that changes. */
export function BlockedPeerView({ session }: { session: Session }) {
  const { world, dispatch } = usePortal()
  const peer = usePeer(session.peer).name
  const me = world.me
  return (
    <Pane>
      <StateTile>
        <ProhibitIcon weight="light" />
      </StateTile>
      <Head
        title={`${peer} doesn’t accept files from ${me.self}`}
        sub={`Nothing was sent. On ${peer}, allow files from ${me.name} in Settings.`}
      />
      <ItemList items={session.payload.items} dim />
      <Waiting>Waiting for {peer}…</Waiting>
      <Actions>
        <Key onClick={() => dispatch({ type: 'cancel', id: session.id })}>Cancel</Key>
      </Actions>
    </Pane>
  )
}
