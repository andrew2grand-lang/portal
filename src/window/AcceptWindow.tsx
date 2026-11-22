import { useEffect, useRef } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { usePortal } from '../engine/context'
import { hostSide } from '../platform'
import stage from './stage/Stage.module.css'
import { OfferInView } from './stage/Offer'
import { WindowContext } from './WindowContext'
import s from './AcceptWindow.module.css'

/** How long to wait for an offer before deciding there's nothing to answer */
const GRACE_MS = 1500

/** The compact window a notification or the tray opens: the offers, one at a
    time, oldest first ("1 of 2"). Return accepts, Esc declines. Closes
    itself once nothing waits. */
export function AcceptWindow() {
  const { world, dispatch } = usePortal()
  const offers = world.sessions.filter((x) => x.direction === 'in' && x.phase === 'offer')
  const offer = offers[0]
  const shown = useRef(false)

  useEffect(() => {
    if (offer) {
      shown.current = true
      return
    }
    const close = () => void getCurrentWindow().close()
    if (shown.current) return close()
    const id = window.setTimeout(close, GRACE_MS)
    return () => window.clearTimeout(id)
  }, [offer])

  useEffect(() => {
    if (!offer) return
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.metaKey || e.ctrlKey || e.altKey) return
      // Only when no button or box has focus (they handle their own keys)
      const focused = document.activeElement
      if (focused && focused !== document.body && focused.tagName !== 'MAIN') return
      if (e.key === 'Enter') dispatch({ type: 'accept', id: offer.id })
      else if (e.key === 'Escape') dispatch({ type: 'decline', id: offer.id })
      else return
      e.preventDefault()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [offer, dispatch])

  return (
    <WindowContext value={hostSide}>
      <main className={s.accept} data-os={hostSide}>
        <h1 className="sr-only">Incoming files</h1>
        {offers.length > 1 && (
          <p className={s.count} aria-live="polite">
            1 of {offers.length}: the next one shows when you answer this one
          </p>
        )}
        <div className={stage.shell}>
          <div className={stage.core}>
            <div className={stage.view}>{offer && <OfferInView key={offer.id} session={offer} />}</div>
          </div>
        </div>
        <p className={s.keys}>Return accepts · Esc declines</p>
      </main>
    </WindowContext>
  )
}
