import { ArrowsLeftRightIcon, DownloadSimpleIcon, FilesIcon, ImageIcon, LockSimpleIcon, QuestionIcon, TextAlignLeftIcon, WarningIcon, XIcon } from '@phosphor-icons/react'
import { Checkbox } from '../../components/Checkbox/Checkbox'
import { Key } from '../../components/Key/Key'
import { usePortal } from '../../engine/context'
import { noticeWords, syncStatus, syncWord } from '../../engine/clipboard'
import { shortcutLabel } from '../../engine/shortcut'
import { names } from '../../engine/words'
import { useSide } from '../WindowContext'
import { clock, targets } from './where'
import s from './Stage.module.css'
import { Head, Pane, Section } from './parts'


/** What the last Send clipboard said (the provider clears it after a few seconds) */
export function ClipResult() {
  const { world } = usePortal()
  const last = world.clipboard?.last
  if (!last) return null
  return (
    <p className={s.clipResult} data-tone={last.ok ? 'ok' : 'warn'} role="status">
      {last.text}
    </p>
  )
}

/** Sends that didn't start, per device, until dismissed */
export function SendErrors() {
  const { world, dispatch } = usePortal()
  if (!world.sendErrors.length) return null
  return (
    <div className={s.sendErrors} role="status">
      <div>
        {world.sendErrors.map((e) => (
          <p key={e}>{e}</p>
        ))}
      </div>
      <Key size="sm" icon={<XIcon />} aria-label="OK" title="OK" onClick={() => dispatch({ type: 'dismissErrors' })} />
    </div>
  )
}

/** A copy that wasn't synced, said once, with what to do (Mission 05 §3.1) */
export function ClipNoticeCard() {
  const { world, dispatch } = usePortal()
  const n = world.clipboard?.notice
  if (!n) return null
  const w = noticeWords(n.kind)
  return (
    <div className={s.clipNotice} role="status">
      <WarningIcon weight="fill" className={s.clipNoticeIcon} aria-hidden />
      <p className={s.clipNoticeText}>{w.text}</p>
      {n.kind === 'private' && (
        <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'shelfPutMarked', count: n.count })}>
          Put on Shelf
        </Key>
      )}
      {n.kind === 'tooBig' && (
        <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'sendClipboard', ids: targets(world).to.map((p) => p.id) })}>
          Send as file
        </Key>
      )}
      <Key size="sm" icon={<XIcon />} aria-label="Dismiss" title="Dismiss" onClick={() => dispatch({ type: 'dismissClipNotice' })} />
    </div>
  )
}

/** Clipboard: copy here, paste there. Send clipboard, automatic sync (the
    only place it's switched), and what arrived lately (Mission 05 §3.1) */
export function ClipboardView() {
  const side = useSide()
  const { world, dispatch } = usePortal()
  const c = world.clipboard
  if (!c) return null
  const key = shortcutLabel(c.shortcut, side)
  const paired = world.peers.filter((p) => p.paired)
  const { to } = targets(world)
  const syncing = paired.some((p) => p.clip?.mine)
  const paused = c.paused !== undefined
  const files = c.last?.files
  return (
    <Pane onClose={() => dispatch({ type: 'openPanel', panel: null })}>
      <Head title="Clipboard" sub={syncStatus(paired, paused, c.paused)} />
      <ClipNoticeCard />
      <div className={s.sendClip}>
        <Key size="lg" variant="ink" icon={<ArrowsLeftRightIcon />} disabled={!to.length} onClick={() => dispatch({ type: 'sendClipboard', ids: to.map((p) => p.id) })}>
          Send clipboard
        </Key>
        {key && <span className={s.sendClipKey}>{key}</span>}
      </div>
      {/* Where Send clipboard goes, before it goes (review 05): the ticked computers, not only the synced one */}
      {to.length > 0 && <p className={s.aside}>To {to.length > 2 ? `${to.length} computers: ` : ''}{names(to.map((p) => p.name))}.</p>}
      {/* Files copied in Finder or Explorer: a real button (Mission 05 §3.1) */}
      {files && files.length > 0 && to.length > 0 && (
        <Key icon={<FilesIcon />} onClick={() => dispatch({ type: 'sendPaths', paths: files })}>
          Send {files.length === 1 ? '1 file' : `${files.length} files`} to {to.length > 2 ? `${to.length} computers` : names(to.map((p) => p.name))}
        </Key>
      )}
      {!to.length && (
        <p className={s.aside}>
          {paired.length > 1
            ? 'Tick a connected computer to send to.'
            : paired[0]
              ? `${paired[0].name} isn’t connected.`
              : 'Pair a computer to send to.'}
        </p>
      )}
      {c.shortcutError && (
        <p className={s.clipResult} data-tone="warn">
          The shortcut isn’t working. Change it in Settings.
        </p>
      )}
      {paired.length > 0 && (
        <Section title="Automatic sync">
          {paired.map((p) => (
            <div key={p.id} className={s.syncRow}>
              <Checkbox checked={!!p.clip?.mine} onChange={(on) => dispatch({ type: 'setClipSync', id: p.id, on })}>
                Sync with {p.name}
              </Checkbox>
              <span className={s.syncWord}>{syncWord(p.clip, p.name, paused)}</span>
            </div>
          ))}
          {syncing && (
            <div className={s.deviceKeys}>
              {paused ? (
                <Key size="sm" onClick={() => dispatch({ type: 'resumeClipSync' })}>
                  Resume
                </Key>
              ) : (
                <>
                  <Key size="sm" onClick={() => dispatch({ type: 'pauseClipSync', minutes: 60 })}>
                    Pause 1 hour
                  </Key>
                  <Key size="sm" onClick={() => dispatch({ type: 'pauseClipSync' })}>
                    Pause until restart
                  </Key>
                </>
              )}
              <Key size="sm" onClick={() => dispatch({ type: 'clipSyncOff' })}>
                Turn off
              </Key>
            </div>
          )}
        </Section>
      )}
      <Received />
      <button type="button" className={s.helpLink} onClick={() => dispatch({ type: 'openPanel', panel: 'help', topic: 'Clipboard and sync' })}>
        <QuestionIcon aria-hidden /> How the clipboard works
      </button>
    </Pane>
  )
}

/** Recently received: the last 5, memory only (Mission 05 §3.1) */
function Received() {
  const { world, dispatch } = usePortal()
  const items = world.clipboard?.items ?? []
  if (!items.length) return null
  return (
    <Section title="Recently received">
      <ul className={s.clips} aria-label="Recently received">
        {items.map((item) => (
          <li key={item.id} className={s.clip}>
            <span className={s.clipIcon} aria-hidden>
              {item.thumb ? (
                <img src={item.thumb} alt="" className={s.clipThumb} />
              ) : item.private ? (
                <LockSimpleIcon weight="light" />
              ) : item.kind === 'image' ? (
                <ImageIcon weight="light" />
              ) : (
                <TextAlignLeftIcon weight="light" />
              )}
            </span>
            <div className={s.clipText}>
              <p className={s.clipPreview}>{item.private ? 'Private text' : item.kind === 'image' ? 'An image' : item.preview}</p>
              <p className={s.clipMeta}>
                From {item.from} · {clock(item.at)}
                {!item.copyable && ' · too big to keep'}
              </p>
            </div>
            <div className={s.clipKeys}>
              {item.copyable && (
                <Key size="sm" onClick={() => dispatch({ type: 'copyClip', id: item.id })}>
                  Copy
                </Key>
              )}
              {item.copyable && item.kind === 'image' && (
                <Key size="sm" icon={<DownloadSimpleIcon />} onClick={() => dispatch({ type: 'saveClipImage', id: item.id })}>
                  Save as file
                </Key>
              )}
              {item.copyable && item.kind !== 'image' && (
                <Key size="sm" onClick={() => dispatch({ type: 'shelfPutReceived', id: item.id })}>
                  Put on Shelf
                </Key>
              )}
            </div>
          </li>
        ))}
      </ul>
    </Section>
  )
}
