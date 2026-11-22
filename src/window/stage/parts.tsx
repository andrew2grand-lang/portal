import { Children, isValidElement, useContext, useEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { motion, useMotionValueEvent, useTransform } from 'motion/react'
import {
  CheckIcon,
  FilePlusIcon,
  FolderPlusIcon,
  QuestionIcon,
  WarningIcon,
  FileCodeIcon,
  FileIcon,
  FilePdfIcon,
  FolderSimpleIcon,
  HourglassMediumIcon,
  ImageIcon,
  PauseIcon,
  XIcon,
} from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { Orb } from '../../components/Orb/Orb'
import type { OrbState } from '../../components/Orb/Orb'
import { Spinner } from '../../components/Spinner/Spinner'
import { formatBytes, formatSpeed, overLink } from '../../engine/format'
import { useLive, usePortal } from '../../engine/context'
import type { CheckStatus, DoctorCheck, FileItem, Link } from '../../engine/types'
import { useSide } from '../WindowContext'
import s from './Stage.module.css'
import { SendMoreContext } from './sendMore'

/* ---- Layout -------------------------------------------------------------- */

/** "1:42" left until `at` (epoch ms), ticking each second */
function useCountdown(at: number): string {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const t = window.setInterval(() => setNow(Date.now()), 1000)
    return () => window.clearInterval(t)
  }, [])
  const left = Math.max(0, Math.ceil((at - now) / 1000))
  return `${Math.floor(left / 60)}:${String(left % 60).padStart(2, '0')}`
}

/** "The code works for 1:42 more": a quiet line under a pairing step */
export function Countdown({ at, children }: { at: number; children: (left: string) => ReactNode }) {
  return <p className={s.countdown}>{children(useCountdown(at))}</p>
}

/** "Send something else: drop it here, or Add files / Add a folder" (the
    ticked devices that aren't busy get it; the send list says who doesn't) */
export function SendMore() {
  const { world, dispatch } = usePortal()
  return (
    <div className={s.addMore} data-over={world.dragOver ? '' : undefined}>
      <span>Send something else: drop it here, or</span>
      <Key size="sm" icon={<FilePlusIcon />} onClick={() => dispatch({ type: 'pickFiles' })}>
        Add files
      </Key>
      <Key size="sm" icon={<FolderPlusIcon />} onClick={() => dispatch({ type: 'pickFiles', folders: true })}>
        Add folder
      </Key>
    </div>
  )
}

interface PaneProps {
  children: ReactNode
  onClose?: () => void
  /** center: a short state sits in the middle of the stage, like the drop zone */
  align?: 'start' | 'center'
}

/**
 * A stage state. Content scrolls inside the fixed window; the action row is
 * docked below it so the primary action is always in view. Centred states
 * keep their actions inside the centred group.
 */
export function Pane({ children, onClose, align = 'start' }: PaneProps) {
  // Esc closes a panel, like the ✕
  useEffect(() => {
    if (!onClose) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !e.defaultPrevented) onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])
  const more = useContext(SendMoreContext)
  const all = Children.toArray(children)
  const docked = align === 'start' ? all.filter((c) => isValidElement(c) && c.type === Actions) : []
  // A panel keeps its heading and ✕ in place: what scrolls goes under them,
  // never beside the ✕ (Review 04)
  const top = onClose && align === 'start' ? all.filter((c) => isValidElement(c) && c.type === Head).slice(0, 1) : []
  const body = all.filter((c) => !docked.includes(c) && !top.includes(c))
  return (
    <div className={s.pane} data-closable={onClose ? '' : undefined} data-align={align} data-top={top.length ? '' : undefined}>
      {top.length > 0 ? (
        <div className={s.paneTop}>
          {top}
          <Key size="sm" icon={<XIcon />} aria-label="Close" title="Close (Esc)" onClick={onClose} />
        </div>
      ) : (
        onClose && <Key className={s.close} size="sm" icon={<XIcon />} aria-label="Close" title="Close (Esc)" onClick={onClose} />
      )}
      <div className={s.paneBody}>
        {body}
        {more && <SendMore />}
      </div>
      {docked.length > 0 && <div className={s.paneFooter}>{docked}</div>}
    </div>
  )
}

export function Head({ title, sub, aside }: { title: ReactNode; sub?: ReactNode; aside?: ReactNode }) {
  return (
    <div className={s.head}>
      <div className={s.headText}>
        <h2 className={s.title}>{title}</h2>
        {sub && <p className={s.sub}>{sub}</p>}
      </div>
      {aside}
    </div>
  )
}

/** Large icon tile that opens a state; tone ties it to the role key that caused it */
export function StateTile({
  tone = 'neutral',
  children,
}: {
  tone?: 'neutral' | 'send' | 'land' | 'danger'
  children: ReactNode
}) {
  return (
    <span className={s.stateTile} data-tone={tone} aria-hidden>
      {children}
    </span>
  )
}

/**
 * Pass actions primary-first. macOS puts the primary on the right,
 * Windows on the left; both right-align the group.
 */
export function Actions({ children, leading }: { children: ReactNode; leading?: ReactNode }) {
  const side = useSide()
  const keys = Children.toArray(children)
  return (
    <div className={s.actions}>
      {leading}
      <div className={s.keys}>{side === 'mac' ? keys.reverse() : keys}</div>
    </div>
  )
}

export function Waiting({ children }: { children: ReactNode }) {
  return (
    <p className={s.waiting}>
      <Spinner size={14} />
      {children}
    </p>
  )
}

/* ---- Items ----------------------------------------------------------------- */

function itemIcon(item: FileItem, project: boolean) {
  if (project) return <FileCodeIcon weight="light" />
  if (item.kind === 'folder') return <FolderSimpleIcon weight="light" />
  const ext = item.name.split('.').pop()?.toLowerCase()
  if (ext === 'pdf') return <FilePdfIcon weight="light" />
  if (ext && ['jpg', 'jpeg', 'png', 'heic', 'gif', 'webp'].includes(ext)) return <ImageIcon weight="light" />
  return <FileIcon weight="light" />
}

/** The small raised tile with a file or folder glyph */
export function ItemTile({ item, project = false }: { item: FileItem; project?: boolean }) {
  return (
    <span className={s.itemTile} aria-hidden>
      {itemIcon(item, project)}
    </span>
  )
}

function itemMeta(item: FileItem, stack?: string) {
  if (stack) return `${stack} · ${formatBytes(item.bytes)}`
  if (item.kind === 'folder' && item.count) return `${item.count} files · ${formatBytes(item.bytes)}`
  return formatBytes(item.bytes)
}

interface ItemListProps {
  items: FileItem[]
  stack?: string
  dim?: boolean
  /** During a transfer: index of the item in flight (-1 = all done) */
  current?: number
  /** What the item in flight is doing, for screen readers ("Receiving" on the receiving side) */
  moving?: 'Sending' | 'Receiving'
}

export function ItemList({ items, stack, dim, current, moving = 'Sending' }: ItemListProps) {
  const live = current !== undefined
  const stateOf = (i: number) => (current === -1 || (live && i < current!) ? 'done' : i === current ? 'active' : 'queued')
  return (
    <ul className={s.items} data-dim={dim ? '' : undefined}>
      {items.map((item, i) => (
        <li key={item.name} className={s.item} data-state={live ? stateOf(i) : undefined}>
          <ItemTile item={item} project={!!stack} />
          <span className={s.itemName}>{item.name}</span>
          <span className={s.itemMeta}>{itemMeta(item, stack)}</span>
          {live && (
            <span className={s.itemStatus}>
              {stateOf(i) === 'done' ? <CheckIcon weight="bold" /> : stateOf(i) === 'active' ? <Spinner size={13} /> : null}
              <span className="sr-only">{stateOf(i) === 'done' ? 'Done' : stateOf(i) === 'active' ? moving : 'Waiting'}</span>
            </span>
          )}
        </li>
      ))}
    </ul>
  )
}

/** Item list that follows the byte progress: done, in flight, queued */
export function LiveItemList({ id, items, stack, moving }: { id: string; items: FileItem[]; stack?: string; moving?: 'Sending' | 'Receiving' }) {
  const { progress } = useLive(id)
  const ends = useMemo(() => {
    const total = items.reduce((n, i) => n + i.bytes, 0) || 1
    const out: number[] = []
    for (const item of items) out.push(((out.at(-1) ?? 0) * total + item.bytes) / total)
    return out
  }, [items])
  const indexAt = (p: number) => ends.findIndex((end) => p < end - 1e-6)
  const [current, setCurrent] = useState(() => indexAt(progress.get()))
  useMotionValueEvent(progress, 'change', (p) => {
    const next = indexAt(p)
    setCurrent((prev) => (prev === next ? prev : next))
  })
  return <ItemList items={items} stack={stack} current={current} moving={moving} />
}

/* ---- Transfer progress (motion values: updates without re-rendering) ------ */

export function TransferProgress({
  id,
  total,
  link,
  stalled = false,
  paused = false,
  checking = false,
  peerName,
  orb,
}: {
  /** The transfer, for its live numbers */
  id: string
  total: number
  link: Link
  stalled?: boolean
  /** Paused (the connection dropped): what's kept, no speed or time left */
  paused?: boolean
  /** Paused, and checking what already arrived before it continues */
  checking?: boolean
  /** Who it's waiting for when nothing moves */
  peerName?: string
  /** Beside the percentage while files move (still when stalled or paused) */
  orb?: OrbState
}) {
  const { progress, rate, eta } = useLive(id)
  const percent = useTransform(progress, (p) => String(Math.floor(p * 100)))
  const delivered = useTransform(progress, (p) => formatBytes(p * total))
  // Speed and time left are already calm: the core decides (Mission 04 §1.2)
  const speed = useTransform(rate, (r) => (r > 0 ? `${formatSpeed(r)} ${overLink[link]}` : overLinkCap[link]))
  // Movement means work is happening: paused or waiting, everything is still
  const still = (paused && !checking) || stalled
  // The gradient is fixed to the track, so the leading edge warms from blue to orange
  const clip = useTransform(progress, (p) => `inset(0 ${((1 - p) * 100).toFixed(2)}% 0 0 round 999px)`)

  // Coarse value for assistive tech; the visuals update every frame
  const [announced, setAnnounced] = useState(() => Math.floor(progress.get() * 10) * 10)
  const last = useRef(announced)
  useMotionValueEvent(progress, 'change', (p) => {
    const step = Math.floor(p * 10) * 10
    if (step !== last.current) {
      last.current = step
      setAnnounced(step)
    }
  })

  return (
    <div className={s.progress}>
      <div className={s.percentRow}>
        <p className={s.percent} aria-hidden>
          <motion.span>{percent}</motion.span>
          <span className={s.percentUnit}>%</span>
        </p>
        {orb && !still && <Orb state={orb} />}
        {orb && still && (
          <span className={s.stillMark} aria-hidden>
            {paused ? <PauseIcon weight="fill" /> : <HourglassMediumIcon weight="regular" />}
          </span>
        )}
      </div>
      <div
        className={s.bar}
        role="progressbar"
        aria-label="Transfer progress"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={announced}
      >
        <motion.span className={s.fill} style={{ clipPath: clip }} />
      </div>
      <div className={s.meta}>
        <span>
          <motion.span>{delivered}</motion.span> of {formatBytes(total)}
        </span>
        {paused ? (
          <span className={s.eta}>{checking ? 'Checking…' : 'Paused'}</span>
        ) : stalled ? (
          // Nothing has moved for 2 s: no speed (never a number draining away)
          <span className={s.eta}>{peerName ? `Waiting for ${peerName}…` : 'Waiting…'}</span>
        ) : (
          <>
            <motion.span>{speed}</motion.span>
            <motion.span className={s.eta}>{eta}</motion.span>
          </>
        )}
      </div>
    </div>
  )
}

/** "Over Wi-Fi", before a speed is known */
const overLinkCap: Record<Link, string> = {
  wifi: 'Over Wi-Fi',
  cable: 'Over Ethernet',
  ethernet: 'Over Ethernet',
  other: 'Over the network',
}


/* ---- Connection doctor ------------------------------------------------------ */

const statusIcon: Record<CheckStatus, ReactNode> = {
  pass: <CheckIcon weight="bold" />,
  fail: <XIcon weight="bold" />,
  likely: <WarningIcon weight="fill" />,
  unknown: <QuestionIcon weight="bold" />,
}

const statusWord: Record<CheckStatus, string> = {
  pass: 'Fine',
  fail: 'Problem',
  likely: 'Likely cause',
  unknown: 'Not sure',
}

/** The app's own fixes, handled here rather than opened as a link */
const FIX_FIREWALL = 'seamdrop:fix-firewall'

/** Each check: passed, failed, likely or unknown, with a plain fix */
export function DoctorList({ checks }: { checks: DoctorCheck[] }) {
  const { world, dispatch } = usePortal()
  const fix = world.fix
  return (
    <>
    <ol className={s.doctor}>
      {checks.map((c) => (
        <li key={c.id} className={s.blocker} data-status={c.status}>
          <span className={s.doctorIcon} aria-hidden>
            {statusIcon[c.status]}
          </span>
          <span className={s.blockerTitle}>
            <span className="sr-only">{statusWord[c.status]}: </span>
            {c.title}
          </span>
          {c.fix && <span className={s.blockerReason}>{c.fix}</span>}
          {c.action && (
            <span className={s.doctorAction}>
              {c.action.url === FIX_FIREWALL ? (
                <Key size="sm" variant="ink" disabled={fix?.busy} onClick={() => dispatch({ type: 'fixFirewall' })}>
                  {fix?.busy ? 'Waiting for Windows…' : c.action.label}
                </Key>
              ) : (
                <Key size="sm" onClick={() => dispatch({ type: 'openUrl', url: c.action!.url })}>
                  {c.action.label}
                </Key>
              )}
            </span>
          )}
        </li>
      ))}
    </ol>
    {fix?.text && (
      <p className={s.clipResult} data-tone={fix.ok ? 'ok' : 'warn'} role="status">
        {fix.text}
      </p>
    )}
    </>
  )
}

/** The six-digit pairing code, grouped 3 + 3 */
export function PairCode({ code, dim }: { code: string; dim?: boolean }) {
  const digits = code.replace(/\D/g, '')
  return (
    <p className={s.code} data-dim={dim ? '' : undefined} aria-label={`Code ${digits.split('').join(' ')}`}>
      <span>{digits.slice(0, 3)}</span>
      <span>{digits.slice(3)}</span>
    </p>
  )
}

/** A titled group inside a panel (settings) */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className={s.section}>
      <h3 className={s.sectionTitle}>{title}</h3>
      {children}
    </section>
  )
}

/* ---- What goes next to a busy device (Mission 05 §1.5) ------------------- */

/** "Next for Desk PC: 2 files · starts when the current transfer finishes",
    each with ✕; kept through a quit, it waits for Continue */
export function NextList({ peer }: { peer?: string }) {
  const { world, dispatch } = usePortal()
  const items = world.next.filter((n) => !peer || n.peer === peer)
  if (!items.length) return null
  const nameOf = (id: string) => world.peers.find((p) => p.id === id)?.name ?? 'that computer'
  return (
    <ul className={s.nextList} aria-label="Next">
      {items.map((n) => {
        const what =
          n.names.length <= 2 ? n.names.join(' and ') : `${n.names[0]} and ${n.names.length - 1} more`
        const when = n.held ? 'waiting for you' : n.afterPaused ? 'after the paused transfer' : 'starts when the current transfer finishes'
        return (
          <li key={n.id} className={s.nextRow}>
            <p className={s.nextText}>
              <strong>Next for {nameOf(n.peer)}:</strong> {what} · {when}
            </p>
            {n.held && (
              <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'continueNext', peer: n.peer })}>
                Continue
              </Key>
            )}
            <Key size="sm" icon={<XIcon />} aria-label={`Don’t send ${what}`} title="Don’t send" onClick={() => dispatch({ type: 'unqueueNext', id: n.id })} />
          </li>
        )
      })}
    </ul>
  )
}
