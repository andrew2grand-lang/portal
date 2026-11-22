import { useState } from 'react'
import { CopyIcon, EyeIcon, LockSimpleIcon, PlusIcon, QuestionIcon, XIcon } from '@phosphor-icons/react'
import { Checkbox } from '../../components/Checkbox/Checkbox'
import { Key } from '../../components/Key/Key'
import { TextField } from '../../components/TextField/TextField'
import { usePortal } from '../../engine/context'
import type { Peer, ShelfItem } from '../../engine/types'
import { names } from '../../engine/words'
import { DeviceIcon } from '../PortalWindow'
import { clock } from './where'
import s from './Stage.module.css'
import { Head, Pane } from './parts'

/** The Shelf: things you keep, deliberately, on the devices you share it
    with (Mission 05 §3.2). Nothing lands here by itself. */
export function ShelfView() {
  const { world, dispatch } = usePortal()
  const items = world.shelf
  const paired = world.peers.filter((p) => p.paired)
  const asking = paired.filter((p) => p.shelf && !p.shelf.answered)
  const shared = paired.filter((p) => p.shelf?.mine && p.shelf?.theirs)
  return (
    <Pane onClose={() => dispatch({ type: 'openPanel', panel: null })}>
      <Head title="Shelf" sub="Text you keep, to copy here or on your other computers." />
      {asking.map((p) => (
        <ShareAsk key={p.id} peer={p} />
      ))}
      {items.length === 0 && <AddForm />}
      {world.shelfUndoUntil && (
        <div className={s.undoBar} role="status">
          <p>Removed.</p>
          <Key size="sm" onClick={() => dispatch({ type: 'shelfUndo' })}>
            Undo
          </Key>
        </div>
      )}
      {items.length > 0 ? (
        <ul className={s.shelfItems} aria-label="Shelf">
          {items.map((item) => (
            <ShelfRow key={item.id} item={item} />
          ))}
        </ul>
      ) : null}
      {items.length > 0 && <AddMore />}
      <ShelfSharing paired={paired.filter((p) => !asking.includes(p))} shared={shared} />
      <div className={s.shelfFoot}>
        {items.length > 0 && <ClearShelf shared={shared} />}
        <button type="button" className={s.helpLink} onClick={() => dispatch({ type: 'openPanel', panel: 'help', topic: 'The Shelf' })}>
          <QuestionIcon aria-hidden /> How the Shelf works
        </button>
      </div>
    </Pane>
  )
}

/** Where the Shelf goes, and sharing it from here (review 05: sharing was
    only in Settings): who has it, who is away (they get it when they're
    back), who hasn't said yes yet, and a Share key for the rest */
function ShelfSharing({ paired, shared }: { paired: Peer[]; shared: Peer[] }) {
  const { dispatch } = usePortal()
  const away = shared.filter((p) => p.status !== 'connected')
  const waiting = paired.filter((p) => p.shelf?.mine && !p.shelf.theirs)
  const offer = paired.filter((p) => !p.shelf?.mine)
  return (
    <div className={s.shelfSharing}>
      <p className={s.shelfShared}>
        {shared.length ? `Shared with ${names(shared.map((p) => p.name))}.` : 'Only on this computer.'}
        {away.length > 0 && ` ${names(away.map((p) => p.name))} ${away.length > 1 ? 'are' : 'is'} away and ${away.length > 1 ? 'get' : 'gets'} these when back.`}
      </p>
      {waiting.map((p) => (
        <p key={p.id} className={s.shelfShared}>
          Waiting for {p.name}: on {p.name}, press Share.
        </p>
      ))}
      {offer.length > 0 && (
        <div className={s.rowKeys}>
          {offer.map((p) => (
            <Key key={p.id} size="sm" onClick={() => dispatch({ type: 'setShelfShare', id: p.id, on: true })}>
              Share with {p.name}
            </Key>
          ))}
        </div>
      )}
    </div>
  )
}

/** "Share the Shelf with Desk PC?" asked once per paired device */
function ShareAsk({ peer }: { peer: Peer }) {
  const { dispatch } = usePortal()
  const theirs = peer.shelf?.theirs
  return (
    <div className={s.shareAsk}>
      <p>
        <strong>Share the Shelf with {peer.name}?</strong>
        {theirs ? ` ${peer.name} said yes.` : ''}
      </p>
      <div className={s.rowKeys}>
        <Key size="sm" onClick={() => dispatch({ type: 'setShelfShare', id: peer.id, on: false })}>
          Not now
        </Key>
        <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'setShelfShare', id: peer.id, on: true })}>
          Share
        </Key>
      </div>
    </div>
  )
}

/** With items there: the form opens from a key, so the items come first */
function AddMore() {
  const [open, setOpen] = useState(false)
  if (open) return <AddForm onDone={() => setOpen(false)} />
  return (
    <Key size="sm" icon={<PlusIcon />} onClick={() => setOpen(true)}>
      Add to Shelf
    </Key>
  )
}

/** Add to Shelf: an optional name, the text, Private */
function AddForm({ onDone }: { onDone?: () => void }) {
  const { dispatch } = usePortal()
  const [name, setName] = useState('')
  const [text, setText] = useState('')
  const [secret, setSecret] = useState(false)
  const add = () => {
    if (!text.trim()) return
    dispatch({ type: 'shelfAdd', name, text, private: secret })
    setName('')
    setText('')
    setSecret(false)
    onDone?.()
  }
  return (
    <div className={s.shelfAdd}>
      <TextField label="Name (optional)" value={name} onChange={setName} placeholder="Wi-Fi password" />
      <textarea
        className={s.sendTextBox}
        aria-label="Text to keep"
        placeholder="Type or paste text to keep"
        rows={2}
        value={text}
        spellCheck={false}
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="off"
        data-secret={secret ? '' : undefined}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            e.preventDefault()
            add()
          }
        }}
      />
      <div className={s.sendTextFoot}>
        <Checkbox checked={secret} onChange={setSecret}>
          Private
        </Checkbox>
        <Key size="sm" variant="ink" icon={<PlusIcon />} disabled={!text.trim()} onClick={add}>
          Add to Shelf
        </Key>
      </div>
    </div>
  )
}

function ShelfRow({ item }: { item: ShelfItem }) {
  const { world, dispatch } = usePortal()
  const [asking, setAsking] = useState(false)
  const from = item.mine ? world.me : world.peers.find((p) => p.id === item.from)
  const revealed = world.revealed?.id === item.id ? world.revealed.text : undefined
  const title = item.name || (item.private ? 'Private text' : (item.preview ?? ''))
  const body = item.private ? (revealed ?? '••••••••••••') : item.name ? item.preview : undefined
  const label = item.name || (item.private ? 'this private item' : 'this item')
  return (
    <li className={s.shelfRow}>
      <div className={s.shelfText}>
        <p className={s.shelfTitle}>
          {item.private && <LockSimpleIcon weight="bold" aria-label="Private" />}
          {title}
        </p>
        {body && (
          <p className={s.shelfBody} data-secret={item.private && !revealed ? '' : undefined}>
            {body}
          </p>
        )}
        <p className={s.clipMeta}>
          from {from && <DeviceIcon device={from} />} {item.mine ? 'this computer' : item.fromName} · {clock(item.at)}
        </p>
        {asking && (
          <div className={s.shelfAsk}>
            <p>Remove {item.name ? `‘${item.name}’` : label} from the Shelf on your devices?</p>
            <div className={s.rowKeys}>
              <Key size="sm" onClick={() => setAsking(false)}>
                Keep
              </Key>
              <Key
                size="sm"
                variant="danger"
                onClick={() => {
                  setAsking(false)
                  dispatch({ type: 'shelfRemove', ids: [item.id] })
                }}
              >
                Remove
              </Key>
            </div>
          </div>
        )}
      </div>
      <div className={s.rowKeys}>
        {item.private && !revealed && (
          <Key size="sm" icon={<EyeIcon />} aria-label={`Reveal ${label}`} title="Reveal for a few seconds" onClick={() => dispatch({ type: 'shelfReveal', id: item.id })} />
        )}
        <Key size="sm" variant="ink" icon={<CopyIcon />} onClick={() => dispatch({ type: 'shelfCopy', id: item.id })}>
          Copy
        </Key>
        <Key size="sm" icon={<XIcon />} aria-label={`Remove ${label}`} title="Remove" onClick={() => setAsking(true)} />
      </div>
    </li>
  )
}

/** Clear Shelf: red, asks first, and names the devices it clears */
function ClearShelf({ shared }: { shared: Peer[] }) {
  const { world, dispatch } = usePortal()
  const [asking, setAsking] = useState(false)
  const where = names([world.me.self, ...shared.map((p) => p.name)])
  if (!asking)
    return (
      <Key size="sm" variant="danger" onClick={() => setAsking(true)}>
        Clear Shelf…
      </Key>
    )
  return (
    <div className={s.shelfAsk}>
      <p>Clear the Shelf on {where}?</p>
      <div className={s.rowKeys}>
        <Key size="sm" onClick={() => setAsking(false)}>
          Keep
        </Key>
        <Key
          size="sm"
          variant="danger"
          onClick={() => {
            setAsking(false)
            dispatch({ type: 'shelfRemove', ids: null })
          }}
        >
          Clear Shelf
        </Key>
      </div>
    </div>
  )
}
