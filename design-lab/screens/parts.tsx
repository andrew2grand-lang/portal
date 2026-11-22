import { useEffect, useRef } from 'react'
import type { ReactNode } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import {
  CaretDownIcon,
  CheckIcon,
  HandIcon,
  InfoIcon,
  LightbulbIcon,
  QuestionIcon,
  WarningOctagonIcon,
  WrenchIcon,
  XIcon,
} from '@phosphor-icons/react'
import { Spinner } from '../../src/components/Spinner/Spinner'
import { useReducedMotionPref } from '../../src/engine/context'
import { formatBytes } from '../../src/engine/format'
import { spring } from '../../src/styles/motion'
import type { Finding, FindingGroup, State } from '../report'
import { place, stateName } from '../report'
import type { DiffLine, ResultState } from '../scenarios'
import s from './screens.module.css'
import st from '../../src/window/stage/Stage.module.css'

export type Tone = State | ResultState | 'running' | 'pending'

const toneIcon: Record<Tone, ReactNode> = {
  blocker: <WarningOctagonIcon weight="fill" />,
  willNeedAFix: <WrenchIcon weight="fill" />,
  needsYou: <HandIcon weight="fill" />,
  couldntCheck: <QuestionIcon weight="bold" />,
  info: <InfoIcon weight="bold" />,
  proved: <CheckIcon weight="bold" />,
  failed: <XIcon weight="bold" />,
  running: <Spinner size={13} />,
  pending: null,
}

/** The round status mark every state uses: same size as the app's checklist */
export function Dot({ tone, size }: { tone: Tone; size?: 'sm' }) {
  return (
    <span className={s.dot} data-tone={tone} data-size={size} aria-hidden>
      {toneIcon[tone]}
    </span>
  )
}

/** Items arriving one after another (findings, steps, diff lines) */
export function Arrive({ children, i = 0, as = 'li' }: { children: ReactNode; i?: number; as?: 'li' | 'div' }) {
  const reduced = useReducedMotionPref()
  const Tag = as === 'li' ? motion.li : motion.div
  return (
    <Tag
      initial={reduced ? { opacity: 0 } : { opacity: 0, y: 6, filter: 'blur(3px)' }}
      animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
      transition={{ ...spring.ui, delay: reduced ? 0 : Math.min(i, 12) * 0.045 }}
    >
      {children}
    </Tag>
  )
}

/* ---- Report findings ----------------------------------------------------------- */

/** "2 places", "main.py:27", or "Whole project"; the full path is inside */
function placesLabel(places: Finding[]) {
  if (places.length > 1) return `${places.length} places`
  const at = place(places[0])
  return at ? at.split('/').pop() : 'Whole project'
}

/** One finding (with all its places): title first; why, places and the fix inside */
export function FindingRow({ group, open, onToggle, target, action }: {
  group: FindingGroup
  open: boolean
  onToggle: () => void
  target: string
  /** A next step that belongs to this finding (e.g. checking packages online) */
  action?: ReactNode
}) {
  const reduced = useReducedMotionPref()
  const f = group.first
  const shown = group.places.filter((p) => p.file || p.evidence).slice(0, 6)
  const extra = group.places.length - shown.length
  const actionable = f.state === 'blocker' || f.state === 'willNeedAFix' || f.state === 'needsYou'
  // Opened from outside (a link to this finding): bring it into view
  const row = useRef<HTMLLIElement>(null)
  const openedFirst = useRef(open)
  useEffect(() => {
    if (openedFirst.current) row.current?.scrollIntoView({ block: 'start' })
  }, [])
  return (
    <li className={s.finding} ref={row}>
      <button type="button" className={s.findingHead} aria-expanded={open} onClick={onToggle}>
        <span>{f.title}</span>
        <span className={s.findingMeta}>{placesLabel(group.places)}</span>
        <CaretDownIcon className={s.caret} data-open={open ? '' : undefined} weight="bold" />
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            className={s.findingBody}
            initial={reduced ? { opacity: 0 } : { height: 0, opacity: 0 }}
            animate={reduced ? { opacity: 1 } : { height: 'auto', opacity: 1 }}
            exit={reduced ? { opacity: 0 } : { height: 0, opacity: 0 }}
            transition={spring.ui}
          >
            <p className={s.why}>{f.why}</p>
            {shown.length > 0 && (
              <ul className={s.places} aria-label="Where">
                {shown.map((p) => (
                  <li key={p.id}>
                    {place(p) && <span className={s.where}>{place(p)}</span>}
                    {p.evidence && <code className={s.code}>{p.evidence}</code>}
                  </li>
                ))}
              </ul>
            )}
            {extra > 0 && <p className={s.more}>and {extra} more</p>}
            {actionable && (
              <p className={s.fix}>
                <LightbulbIcon weight="regular" />
                <span>{f.fixPattern}</span>
              </p>
            )}
            {f.autoFixable && actionable && <p className={s.fixable}>A Pro conversion does this for you on {target}.</p>}
            {action && <div>{action}</div>}
          </motion.div>
        )}
      </AnimatePresence>
    </li>
  )
}

const groupNote: Partial<Record<State, string>> = {
  blocker: 'These stop it from running there until something changes.',
  willNeedAFix: 'Known problems with known fixes.',
  needsYou: 'Only you can provide these.',
  couldntCheck: 'Not tested here, so nothing is claimed.',
}

export function FindingSection({ state, groups, target, openKey, setOpen, i = 0, actionFor }: {
  state: State
  groups: FindingGroup[]
  target: string
  openKey: string | null
  setOpen: (key: string | null) => void
  i?: number
  actionFor?: (f: Finding) => ReactNode
}) {
  if (!groups.length) return null
  return (
    <Arrive as="div" i={i}>
      <section className={s.group} aria-label={stateName[state].group}>
        <h3 className={s.groupHead}>
          <Dot tone={state} />
          {stateName[state].group}
          <span className={s.groupCount}>{groups.length}</span>
        </h3>
        {groupNote[state] && <p className={s.groupNote}>{groupNote[state]}</p>}
        <ul className={s.findings}>
          {groups.map((g) => (
            <FindingRow
              key={g.key}
              group={g}
              target={target}
              open={openKey === g.key}
              onToggle={() => setOpen(openKey === g.key ? null : g.key)}
              action={actionFor?.(g.first)}
            />
          ))}
        </ul>
      </section>
    </Arrive>
  )
}

/* ---- Diffs ------------------------------------------------------------------------ */

/** A change as readable lines; `upTo` reveals it line by line while applying */
export function Diff({ lines, upTo }: { lines: DiffLine[]; upTo?: number }) {
  const reduced = useReducedMotionPref()
  const shown = upTo === undefined ? lines : lines.slice(0, upTo)
  return (
    <ol className={s.diff}>
      {shown.map((l, i) => (
        <motion.li
          key={i}
          data-kind={l.kind}
          initial={upTo === undefined || reduced ? false : { opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.18 }}
        >
          {/* The indent stays unstruck, so a removed line reads as code, not a rule */}
          <span>
            {l.text.match(/^\s*/)?.[0]}
            <span className={s.diffText}>{l.text.trimStart() || ' '}</span>
          </span>
        </motion.li>
      ))}
    </ol>
  )
}

/* ---- Usage meter ------------------------------------------------------------------- */

const dollars = (n: number) => `$${n.toFixed(2)}`

export function Meter({ spent, cap, gift = 0, label = 'AI usage this month' }: {
  spent: number
  cap: number
  gift?: number
  label?: string
}) {
  const limit = cap + gift
  const full = spent >= limit
  return (
    <div className={s.meter}>
      <div className={s.meterRow}>
        <span>{label}</span>
        <span>
          <strong>{dollars(spent)}</strong> of {dollars(limit)}
          {gift > 0 && ` (with ${dollars(gift)} gift)`}
        </span>
      </div>
      <div
        className={s.track}
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={limit}
        aria-valuenow={Math.min(spent, limit)}
      >
        {gift > 0 && <span className={s.gift} style={{ width: '100%' }} />}
        <span className={s.fill} data-full={full ? '' : undefined} style={{ width: `${Math.min(100, (spent / limit) * 100)}%` }} />
      </div>
    </div>
  )
}

/* ---- Small things ---------------------------------------------------------------------- */

export function Bytes({ n }: { n: number }) {
  return <>{formatBytes(n)}</>
}

/** The app's blocker list (moved here from the app in Mission 04: only the
    lab's Project Send screens use it) */
export function Blockers({ blockers }: { blockers: { title: string; reason: string }[] }) {
  return (
    <ol className={st.blockers}>
      {blockers.map((b) => (
        <li key={b.title} className={st.blocker}>
          <span className={st.blockerIcon} aria-hidden>
            <XIcon weight="bold" />
          </span>
          <span className={st.blockerTitle}>{b.title}</span>
          <span className={st.blockerReason}>{b.reason}</span>
        </li>
      ))}
    </ol>
  )
}
