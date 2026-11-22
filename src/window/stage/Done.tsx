import { ArrowClockwiseIcon, CheckIcon, FolderOpenIcon, InfoIcon, XIcon } from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { doneTiming, overLink, plural } from '../../engine/format'
import { usePeer, usePortal } from '../../engine/context'
import type { Session, TransferResult } from '../../engine/types'
import { useSide } from '../WindowContext'
import s from './Stage.module.css'
import { Actions, Head, ItemList, Pane, StateTile } from './parts'
import { revealLabel, shortFolder } from './where'


/** Plain notes from the core (renames, skipped links, long paths), then
    which items they mean, one click away */
function Notes({ result }: { result?: TransferResult }) {
  if (!result) return null
  const lists = [
    { key: 'skipped', label: `Show the ${plural(result.skipped, 'item')} left out`, items: result.skippedItems, total: result.skipped },
    { key: 'renamed', label: `Show the ${plural(result.renamed, 'changed name')}`, items: result.renamedItems, total: result.renamed },
  ].filter((l) => l.items.length > 0)
  if (!result.notes.length && !lists.length) return null
  return (
    <>
      {result.notes.length > 0 && (
        <ul className={s.notes}>
          {result.notes.map((n) => (
            <li key={n}>
              <InfoIcon aria-hidden />
              {n}
            </li>
          ))}
        </ul>
      )}
      {lists.map((l) => (
        <details key={l.key} className={s.which}>
          <summary>{l.label}</summary>
          <ul>
            {l.items.map((i) => (
              <li key={i.path}>
                <span className={s.whichPath}>{i.path}</span>
                <span className={s.whichWhy}>{i.why}</span>
              </li>
            ))}
            {l.total > l.items.length && <li className={s.whichWhy}>…and {l.total - l.items.length} more</li>}
          </ul>
        </details>
      ))}
    </>
  )
}

/** "big10g.bin", "Site photos", "3 files", "3 items": what was delivered */
function deliveredWhat(p: Session['payload']): string {
  if (p.items.length === 1) return p.items[0].name
  const folders = p.items.some((i) => i.kind === 'folder')
  return folders ? `${p.items.length} items` : `${p.items.length} files`
}

/** Receiver: the delivery landed. The same finished moment as the sender's
    (Mission 05 §1.6): the tick, what came from whom, then where it is. */
export function DoneInView({ session }: { session: Session }) {
  const side = useSide()
  const { dispatch } = usePortal()
  const from = usePeer(session.peer)
  const p = session.payload
  const r = session.result
  const where = shortFolder(r?.savedTo, side)
  const what = deliveredWhat(p)
  return (
    <Pane>
      <StateTile tone="land">
        <CheckIcon weight="bold" />
      </StateTile>
      <Head
        title={`Received ${p.items.length === 1 ? `‘${what}’` : what} from ${from.name}`}
        sub={`${doneTiming(p.totalBytes, r?.keptBytes ?? 0, r ? session.elapsed : undefined, 'arrived')} · saved in ${where}${r ? ' · every file checked' : ''}`}
      />
      <ItemList items={p.items} dim />
      <Notes result={r} />
      {/* The first files this computer ever received: where they go, and Change */}
      {r?.firstArrival && <p className={s.aside}>Received files go to {where} from now on.</p>}
      <Actions>
        <Key variant="ink" icon={<FolderOpenIcon />} onClick={() => dispatch({ type: 'reveal', path: r?.revealPath ?? r?.savedTo })}>
          {revealLabel[side]}
        </Key>
        {r?.openPath && <Key onClick={() => dispatch({ type: 'openFile', path: r.openPath! })}>Open</Key>}
        {r?.firstArrival && <Key onClick={() => dispatch({ type: 'chooseReceiveDir' })}>Change…</Key>}
        <Key onClick={() => dispatch({ type: 'dismiss', id: session.id })}>Done</Key>
      </Actions>
    </Pane>
  )
}

/** Sender: delivered */
export function DoneOutView({ session }: { session: Session }) {
  const { dispatch } = usePortal()
  const peer = usePeer(session.peer)
  const p = session.payload
  const how = overLink[session.linkAtEnd ?? 'wifi']
  const r = session.result
  // The receiver's folder, in its own style: "Downloads\SeamDrop"
  const savedTo = r?.savedTo
  const theirs = savedTo?.includes('\\') ? '\\' : '/'
  const where = savedTo ? ` · saved in ${savedTo.split(/[\\/]/).filter(Boolean).slice(-2).join(theirs)} on ${peer.name} · every file checked` : ''
  const what = deliveredWhat(p)
  return (
    <Pane>
      <StateTile tone="send">
        <CheckIcon weight="bold" />
      </StateTile>
      <Head title={`Sent ${p.items.length === 1 ? `‘${what}’` : what} to ${peer.name}`} sub={`${doneTiming(p.totalBytes, r?.keptBytes ?? 0, session.elapsed, 'sent')} ${how}${where}`} />
      <ItemList items={p.items} dim />
      <Notes result={r} />
      <Actions>
        <Key variant="ink" onClick={() => dispatch({ type: 'dismiss', id: session.id })}>
          Done
        </Key>
      </Actions>
    </Pane>
  )
}

/** Sender: the other side said no */
export function DeclinedView({ session }: { session: Session }) {
  const { world, dispatch } = usePortal()
  const peer = usePeer(session.peer)
  return (
    <Pane align="center">
      <StateTile>
        <XIcon />
      </StateTile>
      <Head title={`${peer.name} declined`} sub={`Nothing was sent. Everything is still on ${world.me.self}.`} />
      <Actions>
        <Key variant="ink" onClick={() => dispatch({ type: 'dismiss', id: session.id })}>
          OK
        </Key>
      </Actions>
    </Pane>
  )
}

/** Either side: the transfer stopped (cancelled, disk full, damaged). The
    sender can send the same items again without picking them again. */
export function FailedView({ session }: { session: Session }) {
  const { dispatch } = usePortal()
  const again = session.canSendAgain && session.direction === 'out'
  return (
    <Pane align="center">
      <StateTile tone={session.cancelled ? 'neutral' : 'danger'}>
        <XIcon />
      </StateTile>
      <Head title={session.cancelled ? 'Cancelled' : 'Transfer stopped'} sub={session.failure ?? 'Nothing was saved.'} />
      <Actions>
        {again && (
          <Key variant="ink" icon={<ArrowClockwiseIcon />} onClick={() => dispatch({ type: 'sendAgain', id: session.id })}>
            Send again
          </Key>
        )}
        <Key variant={again ? 'plain' : 'ink'} onClick={() => dispatch({ type: 'dismiss', id: session.id })}>
          OK
        </Key>
      </Actions>
    </Pane>
  )
}
