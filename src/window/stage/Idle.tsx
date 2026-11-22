import { formatBytes, plural } from '../../engine/format'
import { ArrowUpRightIcon, CodeBlockIcon, FilePlusIcon, FilesIcon, FolderPlusIcon, XIcon } from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { Spinner } from '../../components/Spinner/Spinner'
import { usePortal } from '../../engine/context'
import type { Peer, QueuedItem } from '../../engine/types'
import { names } from '../../engine/words'
import { Actions, Head, ItemTile, Pane, StateTile } from './parts'
import { SendErrors } from './Clipboard'
import { targets } from './where'
import { sendListTitle } from '../../engine/views'
import s from './Stage.module.css'

/** Idle: the whole stage is the drop zone; files collect, then Send. One
    job (Mission 05 §3, §6): files and folders. The icon, "Drop files here",
    Add files · Add folder; where they go is the ticked strip above. */
export function IdleView() {
  const { world, dispatch } = usePortal()
  const project = world.mode === 'project'
  if (project) return <ComingSoonView />
  if (world.queue.length > 0) return <QueueView />
  const { to, away } = targets(world)
  // Only when it prevents a mistake: nothing ticked can take them
  const why = !to.length ? (away.length ? `${names(away.map((p) => p.name))} ${away.length > 1 ? 'aren’t' : 'isn’t'} connected.` : 'Tick where they go above.') : null

  return (
    <div className={s.idle}>
      <button
        type="button"
        className={s.drop}
        data-over={world.dragOver ? '' : undefined}
        onClick={() => dispatch({ type: 'pickFiles' })}
      >
        <span className={s.dropMark} aria-hidden>
          <span className={s.rings}>
            <span />
            <span />
            <span />
          </span>
          <span className={s.dropTile}>
            <FilesIcon weight="light" />
          </span>
        </span>
        <span className={s.dropTitle}>Drop files here</span>
        {why && <span className={s.dropSub}>{why}</span>}
      </button>
      <div className={s.idleFoot}>
        <SendErrors />
        <div className={s.idleKeys}>
          <Key size="sm" icon={<FilePlusIcon />} onClick={() => dispatch({ type: 'pickFiles' })}>
            Add files
          </Key>
          <Key size="sm" icon={<FolderPlusIcon />} onClick={() => dispatch({ type: 'pickFiles', folders: true })}>
            Add folder
          </Key>
        </div>
      </div>
    </div>
  )
}

/** Who doesn't get the files on the list, and what to do about it:
    "Desk PC is busy with another transfer…", "Old Laptop is asleep, off or
    out of reach…", "Kitchen Mac mini is still connecting…" */
function LeftOut({ busy, away }: { busy: Peer[]; away: Peer[] }) {
  const connecting = away.filter((p) => p.status === 'connecting')
  const off = away.filter((p) => p.status === 'disconnected' || p.status === 'incompatible')
  const gone = away.filter((p) => !connecting.includes(p) && !off.includes(p))
  const are = (ps: Peer[]) => (ps.length > 1 ? 'are' : 'is')
  const lines = [
    busy.length > 0 && `${names(busy.map((p) => p.name))} ${are(busy)} busy: these go next.`,
    gone.length > 0 && `${names(gone.map((p) => p.name))} ${are(gone)} asleep or off, so ${gone.length > 1 ? 'they won’t' : 'it won’t'} get these.`,
    connecting.length > 0 && `${names(connecting.map((p) => p.name))} ${are(connecting)} still connecting.`,
    off.length > 0 && `${names(off.map((p) => p.name))} ${are(off)} not connected, so ${off.length > 1 ? 'they won’t' : 'it won’t'} get these.`,
  ].filter(Boolean)
  if (!lines.length) return null
  return (
    <>
      {lines.map((l) => (
        <p key={l as string} className={s.aside}>
          {l}
        </p>
      ))}
    </>
  )
}

/** "212 files · 611 MB", "18.7 MB", "Counting…", "Can’t be sent" (why: under the list) */
function queuedMeta(item: QueuedItem) {
  if (item.unusable) return 'Can’t be sent'
  if (item.kind === null) return 'Counting…'
  const size = formatBytes(item.bytes ?? 0)
  return item.kind === 'folder' ? `${plural(item.count ?? 0, 'file')} · ${size}` : size
}

/** The send list: what's been collected, until Send sends it all (one
    transfer per ticked device) */
function QueueView() {
  const { world, dispatch } = usePortal()
  const items = world.queue
  const ready = items.filter((i) => !i.unusable)
  const unusable = items.filter((i) => i.unusable)
  const counting = items.some((i) => i.kind === null)
  const files = ready.reduce((n, i) => n + (i.kind === 'folder' ? (i.count ?? 0) : 1), 0)
  const bytes = ready.reduce((n, i) => n + (i.bytes ?? 0), 0)
  const { to, busy, away } = targets(world)
  const dest = to.length
    ? `to ${names(to.map((p) => p.name))}`
    : busy.length
      ? `next for ${names(busy.map((p) => p.name))}`
      : away.length
        ? 'no ticked computer can take them right now'
        : 'tick a computer to send to'
  const summary = counting ? `Counting what’s in it · ${dest}` : `${plural(files, 'file')} · ${formatBytes(bytes)} · ${dest}`
  const live = world.sessions.filter((x) => x.phase === 'transfer' || x.phase === 'paused' || x.phase === 'offer').length
  return (
    <Pane>
      <Head title={sendListTitle(world)} sub={summary} />
      <ul className={s.items} aria-label="Send list">
        {items.map((item) => (
          <li key={item.path} className={s.item} data-state={item.unusable ? 'queued' : undefined}>
            <ItemTile item={{ name: item.name, kind: item.kind ?? 'file', bytes: item.bytes ?? 0, count: item.count }} />
            <span className={s.itemName} title={item.path}>
              {item.name}
            </span>
            <span className={s.itemMeta} data-busy={item.kind === null ? '' : undefined}>
              {item.kind === null && <Spinner size={11} />}
              {queuedMeta(item)}
            </span>
            <Key size="sm" className={s.unqueue} icon={<XIcon />} aria-label={`Remove ${item.name}`} onClick={() => dispatch({ type: 'unqueue', path: item.path })} />
          </li>
        ))}
      </ul>
      <div className={s.addMore} data-over={world.dragOver ? '' : undefined}>
        <span>Drop more here, or</span>
        <Key size="sm" icon={<FilePlusIcon />} onClick={() => dispatch({ type: 'pickFiles' })}>
          Add files
        </Key>
        <Key size="sm" icon={<FolderPlusIcon />} onClick={() => dispatch({ type: 'pickFiles', folders: true })}>
          Add folder
        </Key>
      </div>
      {unusable.length > 0 && (
        <p className={s.aside}>
          {names(unusable.map((i) => i.name))} can’t be sent (a link, or unreadable).
        </p>
      )}
      <LeftOut busy={busy} away={away} />
      {live > 0 && <p className={s.aside}>Transfers under way carry on meanwhile.</p>}
      <SendErrors />
      <Actions>
        <Key variant="ink" icon={<ArrowUpRightIcon />} disabled={!ready.length || counting || (!to.length && !busy.length)} onClick={() => dispatch({ type: 'sendQueue' })}>
          {to.length || !busy.length ? 'Send' : 'Send next'}
        </Key>
        <Key onClick={() => dispatch({ type: 'clearQueue' })}>Clear</Key>
      </Actions>
    </Pane>
  )
}

/** Project Send isn't part of this release: say so calmly, never fake it
    (behind a developer setting, Mission 04 §5) */
function ComingSoonView() {
  return (
    <Pane align="center">
      <StateTile>
        <CodeBlockIcon weight="light" />
      </StateTile>
      <Head
        title="Project Send is coming soon"
        sub="Files work today."
      />
    </Pane>
  )
}
