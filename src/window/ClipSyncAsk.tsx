import { AnimatePresence, motion } from 'motion/react'
import { ArchiveIcon, ArrowsLeftRightIcon, FolderOpenIcon } from '@phosphor-icons/react'
import { Key } from '../components/Key/Key'
import { usePortal, useReducedMotionPref } from '../engine/context'
import { easeOut, fadeOnly, materialize } from '../styles/motion'
import s from './UpdateNotice.module.css'

/** A paired device turned on automatic clipboard sync and asks this one.
    Nothing syncs until this computer says yes; Not now turns it off for
    that pair on both. One question at a time (the oldest first). */
export function ClipSyncAsk() {
  const { world, dispatch } = usePortal()
  const reduced = useReducedMotionPref()
  const asking = world.peers.find((p) => p.paired && p.clip?.asks)
  const name = asking?.name
  const t = reduced ? fadeOnly : materialize
  const answer = (on: boolean) => asking && dispatch({ type: 'setClipSync', id: asking.id, on })
  return (
    <AnimatePresence initial={false}>
      {name && (
        <motion.div
          key={`clip-ask-${asking?.id}`}
          className={s.notice}
          role="status"
          initial={t.initial}
          animate={t.animate}
          exit={t.exit}
          transition={{ duration: reduced ? 0.2 : 0.4, ease: easeOut }}
        >
          <ArrowsLeftRightIcon className={s.icon} weight="light" aria-hidden />
          <div className={s.text}>
            <p className={s.title}>{name} asks to sync the clipboard</p>
            <p className={s.sub}>Nothing syncs unless you turn it on.</p>
          </div>
          <div className={s.keys}>
            <Key size="sm" variant="ink" onClick={() => answer(true)}>
              Turn on
            </Key>
            <Key size="sm" onClick={() => answer(false)}>
              Not now
            </Key>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

/** "Share the Shelf with Desk PC?": asked once per paired device, at pairing
    or after the update for older pairings (Mission 05 §3.2). Off until both
    say yes. */
export function ShelfShareAsk() {
  const { world, dispatch } = usePortal()
  const reduced = useReducedMotionPref()
  // The clipboard question first: one question at a time
  const clipAsking = world.peers.some((p) => p.paired && p.clip?.asks)
  const asking = clipAsking || world.panel === 'shelf' ? undefined : world.peers.find((p) => p.paired && p.shelf && !p.shelf.answered)
  const t = reduced ? fadeOnly : materialize
  const answer = (on: boolean) => asking && dispatch({ type: 'setShelfShare', id: asking.id, on })
  return (
    <AnimatePresence initial={false}>
      {asking && (
        <motion.div
          key={`shelf-ask-${asking.id}`}
          className={s.notice}
          role="status"
          initial={t.initial}
          animate={t.animate}
          exit={t.exit}
          transition={{ duration: reduced ? 0.2 : 0.4, ease: easeOut }}
        >
          <ArchiveIcon className={s.icon} weight="light" aria-hidden />
          <div className={s.text}>
            <p className={s.title}>Share the Shelf with {asking.name}?</p>
            <p className={s.sub}>{asking.shelf?.theirs ? `${asking.name} said yes.` : 'Shared only when both say yes.'}</p>
          </div>
          <div className={s.keys}>
            <Key size="sm" variant="ink" onClick={() => answer(true)}>
              Share
            </Key>
            <Key size="sm" onClick={() => answer(false)}>
              Not now
            </Key>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

/** What changed about received files with the update, said once (Mission 05 §4) */
export function ReceiveNote() {
  const { world, dispatch } = usePortal()
  const reduced = useReducedMotionPref()
  const note = world.settings.receiveNote
  const t = reduced ? fadeOnly : materialize
  return (
    <AnimatePresence initial={false}>
      {note && (
        <motion.div key="receive-note" className={s.notice} role="status" initial={t.initial} animate={t.animate} exit={t.exit} transition={{ duration: reduced ? 0.2 : 0.4, ease: easeOut }}>
          <FolderOpenIcon className={s.icon} weight="light" aria-hidden />
          <div className={s.text}>
            <p className={s.title}>Received files go to Downloads now</p>
            <p className={s.sub}>Settings → Received files has the other choices.</p>
          </div>
          <div className={s.keys}>
            <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'saveSettings', patch: { clearReceiveNote: true } })}>
              OK
            </Key>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
