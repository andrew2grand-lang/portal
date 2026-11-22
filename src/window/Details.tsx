import { useEffect, useRef, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { CheckCircleIcon, CopyIcon, DownloadSimpleIcon, EnvelopeSimpleIcon, ListBulletsIcon, WarningIcon } from '@phosphor-icons/react'
import { Checkbox } from '../components/Checkbox/Checkbox'
import { LINKS } from '../links'
import { LOG_HOLDS } from './stage/Feedback'
import { Key } from '../components/Key/Key'
import { useReducedMotionPref, usePortal } from '../engine/context'
import { Actions, Head, Pane } from './stage/parts'
import { targets } from './stage/where'
import { names } from '../engine/words'
import st from './stage/Stage.module.css'
import s from './Details.module.css'

/** The bottom of the window: a small Details button, and the one fixed place
    where clipboard messages show and fade (Mission 04 §1.5, §4.1). The row
    keeps its height, so a message never moves anything. */
/** Where Details remembers whether to show the technical part */
const TECH_KEY = 'seamdrop.details.technical'

export function Foot() {
  const { world, dispatch } = usePortal()
  const reduced = useReducedMotionPref()
  const open = world.panel === 'details'
  const last = world.clipboard?.last
  const { to } = targets(world)
  return (
    <div className={s.foot}>
      <button
        type="button"
        className={s.toggle}
        aria-pressed={open}
        onClick={() => dispatch({ type: 'openPanel', panel: open ? null : 'details' })}
      >
        <ListBulletsIcon className={s.toggleIcon} />
        <span className={s.toggleLabel}>Details</span>
      </button>
      <div className={s.message} role="status" aria-live="polite">
        <AnimatePresence initial={false}>
          {last && (
            <motion.div
              key={last.at}
              className={s.messageText}
              data-tone={last.ok ? 'ok' : 'warn'}
              initial={{ opacity: 0, y: reduced ? 0 : 3 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.25 }}
            >
              {/* Done or not done, told apart at a glance (Review 04) */}
              {last.ok ? <CheckCircleIcon weight="fill" className={s.messageIcon} aria-hidden /> : <WarningIcon weight="fill" className={s.messageIcon} aria-hidden />}
              {last.text}
              {/* Files copied in Finder or Explorer: one press sends them (Mission 04 §4.2) */}
              {last.files && to.length > 0 && (
                <>
                  {' · '}
                  <button type="button" className={s.messageAction} onClick={() => dispatch({ type: 'sendPaths', paths: last.files ?? [] })}>
                    Send {last.files.length === 1 ? 'it' : 'them'} to {to.length > 2 ? `the ${to.length} ticked devices` : names(to.map((p) => p.name))}
                  </button>
                </>
              )}
              {/* "…paste it into the Shelf's text box": the Shelf is an icon up top, so one press opens it */}
              {!last.files && /\bShelf\b/.test(last.text) && world.panel !== 'shelf' && (
                <>
                  {' · '}
                  <button type="button" className={s.messageAction} onClick={() => dispatch({ type: 'openPanel', panel: 'shelf' })}>
                    Open the Shelf
                  </button>
                </>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  )
}

/** Details, open: what happened, in plain words, newest last, like a calm
    progress story; the technical part follows each line, quieter. It takes
    the stage like the other panels, so opening it never squashes the window
    (Mission 04 §1.6). */
export function DetailsView() {
  const { world, dispatch } = usePortal()
  const list = useRef<HTMLOListElement>(null)
  // Follows new lines while you're at the end; stays put while you read back
  const atEnd = useRef(true)
  const lines = world.logs
  // The technical part under each line: for whoever wants it (remembered here)
  const [tech, setTech] = useState(() => {
    try {
      return localStorage.getItem(TECH_KEY) === '1'
    } catch {
      return false
    }
  })
  const showTech = (on: boolean) => {
    setTech(on)
    try {
      localStorage.setItem(TECH_KEY, on ? '1' : '0')
    } catch {
      // Not remembered: it still works for now
    }
  }

  useEffect(() => {
    const box = list.current?.closest<HTMLElement>(`.${st.paneBody}`)
    if (!box) return
    const onScroll = () => {
      atEnd.current = box.scrollHeight - box.scrollTop - box.clientHeight < 24
    }
    box.addEventListener('scroll', onScroll, { passive: true })
    return () => box.removeEventListener('scroll', onScroll)
  }, [])
  useEffect(() => {
    const box = list.current?.closest<HTMLElement>(`.${st.paneBody}`)
    if (box && atEnd.current) box.scrollTop = box.scrollHeight
  }, [lines.length])

  return (
    <Pane onClose={() => dispatch({ type: 'openPanel', panel: null })}>
      <Head
        title="Details"
        sub="What happened, newest last."
        aside={
          <Checkbox checked={tech} onChange={showTech}>
            Technical details
          </Checkbox>
        }
      />
      {lines.length === 0 ? (
        <p className={st.aside}>Nothing yet.</p>
      ) : (
        <ol ref={list} className={s.story} role="log" aria-label="What happened">
          {lines.map((l) => (
            <li key={l.id} className={s.line} data-tone={l.tone}>
              <span className={s.time}>{l.time}</span>
              <span className={s.said}>
                <span className={s.text}>
                  {l.text}
                  {l.count > 1 && <span className={s.repeat}> ×{l.count}, last at {l.last}</span>}
                </span>
                {tech && l.detail && <span className={s.detail}>{l.detail}</span>}
              </span>
            </li>
          ))}
        </ol>
      )}
      <Actions>
        <Key icon={<CopyIcon />} title="The full log, with every technical detail, for support" onClick={() => dispatch({ type: 'copyTechnicalLog' })}>
          Copy technical log
        </Key>
        {LINKS.confirmed ? (
          <Key icon={<EnvelopeSimpleIcon />} onClick={() => dispatch({ type: 'sendFeedback', withLog: true })}>
            Send feedback
          </Key>
        ) : (
          <Key icon={<DownloadSimpleIcon />} title={LOG_HOLDS} onClick={() => dispatch({ type: 'saveTechnicalLog' })}>
            Save technical log
          </Key>
        )}
      </Actions>
    </Pane>
  )
}
