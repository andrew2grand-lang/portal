import { AnimatePresence, motion } from 'motion/react'
import { ArrowCircleUpIcon } from '@phosphor-icons/react'
import { Key } from '../components/Key/Key'
import { usePortal, useReducedMotionPref } from '../engine/context'
import { noticeFor, restartLabel } from '../engine/updates'
import { easeOut, fadeOnly, materialize } from '../styles/motion'
import s from './UpdateNotice.module.css'

/** A calm line under the header when an update is ready, never a popup */
export function UpdateNotice() {
  const { world, dispatch } = usePortal()
  const reduced = useReducedMotionPref()
  // Paused transfers survive the restart: say so before and during it
  const paused = world.sessions.filter((x) => x.phase === 'paused').map((x) => world.peers.find((p) => p.id === x.peer)?.name ?? 'the other computer')
  const notice = noticeFor(world.update, paused)
  const t = reduced ? fadeOnly : materialize
  return (
    <AnimatePresence initial={false}>
      {notice && (
        <motion.div
          key="update"
          className={s.notice}
          role="status"
          initial={t.initial}
          animate={t.animate}
          exit={t.exit}
          transition={{ duration: reduced ? 0.2 : 0.4, ease: easeOut }}
        >
          <ArrowCircleUpIcon className={s.icon} weight="light" aria-hidden />
          <div className={s.text}>
            <p className={s.title}>{notice.title}</p>
            {notice.sub && <p className={s.sub}>{notice.sub}</p>}
          </div>
          {notice.keys && world.update && (
            <div className={s.keys}>
              {notice.keys === 'offer' && (
                <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'restartToUpdate' })}>
                  {restartLabel(world.update)}
                </Key>
              )}
              {/* Failed: download it again (or check again), or leave it until the next check */}
              {notice.keys === 'failed' && (
                <Key
                  size="sm"
                  variant="ink"
                  onClick={() => dispatch({ type: world.update?.version ? 'restartToUpdate' : 'checkUpdates' })}
                >
                  Try again
                </Key>
              )}
              <Key size="sm" onClick={() => dispatch({ type: 'updateLater' })}>
                {notice.keys === 'failed' ? 'OK' : world.update.state === 'waiting' ? 'Not now' : 'Later'}
              </Key>
            </div>
          )}
        </motion.div>
      )}
    </AnimatePresence>
  )
}
