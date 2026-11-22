import { AnimatePresence, motion } from 'motion/react'
import { useReducedMotionPref, usePortal } from '../../engine/context'
import type { Session } from '../../engine/types'
import { shownSessions, viewFor } from '../../engine/views'
import type { ViewKind } from '../../engine/views'
import { easeOut, fadeOnly, materialize } from '../../styles/motion'
import { BlockedPeerView } from './Blocked'
import {
  AddView,
  ConnectView,
  ConnectingView,
  DisconnectedView,
  DoctorView,
  IncompatibleView,
  NearbyView,
  OfflineView,
  PairingView,
  SearchingView,
} from './Connection'
import { DeclinedView, DoneInView, DoneOutView, FailedView } from './Done'
import { IdleView } from './Idle'
import { OfferInView, OfferOutView } from './Offer'
import { SettingsView } from './Settings'
import { PausedView, TransferView } from './Transfer'
import { TransferList } from './TransferList'
import { SendMoreContext } from './sendMore'
import { ClipboardView } from './Clipboard'
import { HelpView } from './Help'
import { ShelfView } from './Shelf'
import { DetailsView } from '../Details'
import s from './Stage.module.css'

/** One transfer's own screen, by phase and direction */
export function SessionView({ session }: { session: Session }) {
  const out = session.direction === 'out'
  switch (session.phase) {
    case 'failed':
      return <FailedView session={session} />
    case 'blocked':
      return <BlockedPeerView session={session} />
    case 'declined':
      return <DeclinedView session={session} />
    case 'offer':
      return out ? <OfferOutView session={session} /> : <OfferInView session={session} />
    case 'transfer':
      return <TransferView session={session} />
    case 'paused':
      return <PausedView session={session} />
    case 'done':
      return out ? <DoneOutView session={session} /> : <DoneInView session={session} />
  }
}

/* The session is passed as a prop so an exiting view keeps the data it was
   rendered with while it animates out. */
function renderView(view: ViewKind, sessions: Session[], searchStartedAt: number, more: boolean) {
  switch (view) {
    case 'searching':
      return <SearchingView key={searchStartedAt} />
    case 'nearby':
      return <NearbyView />
    case 'pairing':
      return <PairingView />
    case 'connecting':
      return <ConnectingView />
    case 'offline':
      return <OfflineView />
    case 'disconnected':
      return <DisconnectedView />
    case 'incompatible':
      return <IncompatibleView />
    case 'settings':
      return <SettingsView />
    case 'connect':
      return <ConnectView />
    case 'add':
      return <AddView />
    case 'doctor':
      return <DoctorView />
    case 'clipboard':
      return <ClipboardView />
    case 'shelf':
      return <ShelfView />
    case 'help':
      return <HelpView />
    case 'details':
      return <DetailsView />
    case 'session':
      if (!sessions[0]) return <IdleView />
      // With other devices to send to, a transfer's screen offers that too
      return (
        <SendMoreContext.Provider value={more && ['transfer', 'paused', 'offer'].includes(sessions[0].phase) && !(sessions[0].phase === 'offer' && sessions[0].direction === 'in')}>
          <SessionView session={sessions[0]} />
        </SendMoreContext.Provider>
      )
    case 'sessions':
      return <TransferList sessions={sessions} more={more} />
    case 'idle':
      return <IdleView />
    case 'starting':
      return null
  }
}

/** The main area: a frosted plate in a tray (double bezel) */
export function Stage() {
  const { world } = usePortal()
  const reduced = useReducedMotionPref()
  const view = viewFor(world)
  const sessions = shownSessions(world)
  const key = view === 'session' ? `session-${sessions[0]?.id}-${sessions[0]?.phase}-${sessions[0]?.direction}` : view
  const t = reduced ? fadeOnly : materialize

  return (
    <div className={s.shell}>
      <div className={s.core}>
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.div
            key={key}
            className={s.view}
            initial={t.initial}
            animate={t.animate}
            exit={t.exit}
            transition={{ duration: reduced ? 0.2 : 0.4, ease: easeOut }}
          >
            {renderView(view, sessions, world.searchStartedAt, world.peers.filter((p) => p.paired).length > 1)}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  )
}
