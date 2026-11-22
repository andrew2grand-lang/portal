import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { animate, motion, useMotionValue, useReducedMotionConfig, useSpring, useTransform } from 'motion/react'
import type { Link } from '../../engine/types'
import type { GlowMode } from '../../engine/views'
import { glowLap } from '../../styles/motion'
import s from './EdgeGlow.module.css'

interface EdgeGlowProps {
  /** sending: light travels clockwise around the edge. receiving: the same
      light, warm, turning the other way. working: a slow orbit while the
      receiver converts a project. Paused is `off`: still. */
  mode: GlowMode
  /** Lap speed follows the link: the cable is visibly faster */
  link: Link
  /** A new value triggers the arrival flash */
  arrival: string | null
}

/** One lap at this many seconds is playback rate 1. */
const BASE_LAP = glowLap.wifi
/** A fading orbit keeps turning this long after it ends, then stops. */
const FADE_MS = 800

/**
 * The signature moment. Lives in the window shell (not inside the clipped
 * window), so the ring sits exactly on the window edge and follows its
 * corner radius via --win-radius, while the halo blooms outside it.
 *
 * Runs on the compositor (Mission 04 §1.3): the travelling light is a
 * gradient painted once on a square layer, turned by a Web Animations API
 * `transform` animation and masked to the ring, so it keeps going when
 * JavaScript is busy. Lap changes ease the playback rate (no jump).
 * Receiving turns the same way a sender's light would arrive: the other
 * way round, in the warm landing colours (Mission 05 §1.1: the old progress
 * fill crawled a degree at a time on a big transfer; the bar in the window
 * already shows progress).
 */
export function EdgeGlow({ mode, link, arrival }: EdgeGlowProps) {
  // The OS setting, or the dev panel's (via MotionConfig)
  const reduced = !!useReducedMotionConfig()

  // Keep painting the last visible mode while fading out
  const [paint, setPaint] = useState<GlowMode>(mode)
  if (mode !== 'off' && mode !== paint) setPaint(mode)

  const root = useRef<HTMLDivElement>(null)
  const spins = useRef<(HTMLDivElement | null)[]>([])

  // The spinning layers must cover the ring at any angle: their side is the diagonal.
  useLayoutEffect(() => {
    const el = root.current
    if (!el) return
    const size = () => el.style.setProperty('--glow-diag', `${Math.ceil(Math.hypot(el.offsetWidth, el.offsetHeight)) + 16}px`)
    size()
    const ro = new ResizeObserver(size)
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  // The orbit: one compositor animation per spinning layer, in step.
  // Started once the ring has painted and is visible: an animation started
  // on an invisible or not-yet-painted layer stays on the main thread
  // (Chromium: "no visible change", "invalid compositing state").
  const orbiting = !reduced && paint !== 'off'
  const running = !reduced && mode !== 'off'
  const turn = paint === 'receiving' ? '-1turn' : '1turn'
  const lap = paint === 'working' ? glowLap.working : glowLap[link]
  const anims = useRef<Animation[]>([])
  const rateNow = useRef(BASE_LAP / lap)
  const presence = useSpring(0, { bounce: 0, duration: 0.7 })
  useEffect(() => {
    if (!orbiting) return
    let frame = 0
    let alive = true
    const start = () => {
      if (!alive) return
      if (presence.get() < 0.05) {
        frame = requestAnimationFrame(start)
        return
      }
      const els = spins.current.filter((e): e is HTMLDivElement => !!e && e.offsetParent !== null)
      const startTime = document.timeline.currentTime
      anims.current = els.map((el) => {
        const a = el.animate([{ transform: 'rotate(0turn)' }, { transform: `rotate(${turn})` }], {
          duration: BASE_LAP * 1000,
          iterations: Infinity,
          easing: 'linear',
        })
        a.startTime = startTime
        a.playbackRate = rateNow.current
        return a
      })
    }
    // Two frames: the layer is laid out and painted first
    frame = requestAnimationFrame(() => (frame = requestAnimationFrame(start)))
    return () => {
      alive = false
      cancelAnimationFrame(frame)
      for (const a of anims.current) a.cancel()
      anims.current = []
    }
  }, [orbiting, presence, turn])
  // A finished orbit fades out still turning, then stops.
  useEffect(() => {
    if (running || !orbiting) return
    const id = window.setTimeout(() => setPaint('off'), FADE_MS)
    return () => window.clearTimeout(id)
  }, [running, orbiting])
  // Speed changes (Wi-Fi to cable) ease in instead of jumping.
  useEffect(() => {
    const target = BASE_LAP / lap
    const id = window.setInterval(() => {
      const next = rateNow.current + (target - rateNow.current) * 0.35
      rateNow.current = Math.abs(target - next) < 0.01 ? target : next
      for (const a of anims.current) a.updatePlaybackRate(rateNow.current)
      if (rateNow.current === target) window.clearInterval(id)
    }, 50)
    return () => window.clearInterval(id)
  }, [lap])

  useEffect(() => {
    presence.set(mode === 'off' ? 0 : 1)
  }, [mode, presence])

  // Brightness is opacity on its own layer: no repaint
  const ringOpacity = presence
  const innerOpacity = useTransform(presence, (p) => p * 0.7)

  // Arrival flash, once per delivery
  const flash = useMotionValue(0)
  const seen = useRef(arrival)
  useEffect(() => {
    if (arrival === null || arrival === seen.current) {
      seen.current = arrival
      return
    }
    seen.current = arrival
    const controls = reduced
      ? animate(flash, [0, 0.5, 0], { duration: 1.6, times: [0, 0.35, 1], ease: 'easeInOut' })
      : animate(flash, [0, 1, 0.45, 0], { duration: 1.9, times: [0, 0.09, 0.38, 1], ease: 'easeOut' })
    return () => controls.stop()
  }, [arrival, flash, reduced])

  return (
    <div ref={root} className={s.root} data-mode={paint} data-reduced={reduced ? '' : undefined} aria-hidden>
      <motion.div className={s.halo} style={{ opacity: ringOpacity }}>
        <div className={s.spin} ref={(el) => void (spins.current[0] = el)} />
      </motion.div>
      <motion.div className={s.inner} style={{ opacity: innerOpacity }} />
      <motion.div className={s.ring} style={{ opacity: ringOpacity }}>
        <div className={s.spin} ref={(el) => void (spins.current[1] = el)} />
      </motion.div>
      <motion.div className={`${s.halo} ${s.flash}`} style={{ opacity: flash }} />
      <motion.div className={`${s.inner} ${s.flash}`} style={{ opacity: flash }} />
      <motion.div className={`${s.ring} ${s.flash}`} style={{ opacity: flash }} />
    </div>
  )
}
