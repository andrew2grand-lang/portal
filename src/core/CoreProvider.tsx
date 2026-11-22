import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { animate, motionValue, MotionConfig } from 'motion/react'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart'
import { PortalContext } from '../engine/context'
import type { Live } from '../engine/context'
import { loadDev, saveDev } from '../engine/devPrefs'
import { useThemeEffect } from '../engine/theme'
import type { Action, QueuedItem } from '../engine/types'
import { shortcutLabel } from '../engine/shortcut'
import { etaWords } from '../engine/format'
import { names } from '../engine/words'
import { targets } from '../engine/views'
import { hostSide } from '../platform'
import { feedbackMail } from '../links'
import { core, EVENT, UPDATE_EVENT } from './bridge'
import type { CoreEvent, CoreSnapshot, CoreTransfer, CoreUpdate } from './bridge'
import { appendLog, buildWorld, defaultUi } from './buildWorld'
import type { CoreUi, LogRun } from './buildWorld'
import { moment, startClock } from './moment'
import type { Shown } from './moment'

const MAX_LOG = 300
const WORKING_OUT = etaWords({ kind: 'working_out' })

function systemTheme(): 'light' | 'dark' {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

const newLive = (): Live => ({ progress: motionValue(0), rate: motionValue(0), eta: motionValue(WORKING_OUT) })

/** The real app: the world comes from the Rust core; actions become commands. */
export function CoreProvider({ children }: { children: ReactNode }) {
  const [snap, setSnap] = useState<{ value: CoreSnapshot; at: number } | null>(null)
  /** The newest clipboard item seen (null: no snapshot yet) */
  const lastClip = useRef<number | null>(null)
  const [logs, setLogs] = useState<LogRun[]>([])
  const [ui, setUi] = useState<CoreUi>(() => {
    // The app follows the system appearance; the dev panel can override it for this session
    const base = defaultUi()
    return { ...base, dev: { ...base.dev, ...loadDev(), theme: systemTheme() } }
  })
  const [mountedAt] = useState(() => Date.now())
  // The arrival moment, per transfer: a near-instant one still shows the
  // light and the landing
  const [shown, setShown] = useState<Record<string, Shown>>({})
  const lives = useRef(new Map<string, Live>())
  const live = useCallback((id: string) => {
    let l = lives.current.get(id)
    if (!l) {
      l = newLive()
      lives.current.set(id, l)
    }
    return l
  }, [])
  const searchAnchor = useRef(0)
  const patchUi = useCallback((p: Partial<CoreUi>) => setUi((u) => ({ ...u, ...p })), [])
  // What Send clipboard said, shown for a few seconds
  const clipTimer = useRef(0)
  const sayClip = useCallback(
    (ok: boolean, text: string, files?: string[]) => {
      const at = Date.now()
      patchUi({ clipResult: { ok, text, at, files } })
      window.clearTimeout(clipTimer.current)
      // An offer stays longer than a message: it asks for a press
      const shown = files?.length ? 20_000 : 6000
      clipTimer.current = window.setTimeout(() => setUi((u) => (u.clipResult?.at === at ? { ...u, clipResult: undefined } : u)), shown)
    },
    [patchUi],
  )

  // Snapshot + history first, then live events
  useEffect(() => {
    let alive = true
    const seen = new Map<string, CoreTransfer['phase']>()
    const accept = (value: CoreSnapshot) => {
      const now = Date.now()
      setShown((prev) => {
        const next: Record<string, Shown> = {}
        for (const t of value.transfers) next[t.id] = moment({ id: t.id, phase: t.phase }, prev[t.id]?.clock ?? startClock(), now)
        return next
      })
      for (const t of value.transfers) {
        const l = live(t.id)
        // A new transfer starts the bar where the core says it is; a paused
        // or resumed one shows what's kept, and works time left out again
        const was = seen.get(t.id)
        if (was === undefined || t.phase === 'paused' || (was !== 'running' && t.phase === 'running')) {
          l.progress.set(t.totalBytes ? t.bytes / t.totalBytes : 0)
          l.rate.set(0)
          l.eta.set(WORKING_OUT)
        }
        seen.set(t.id, t.phase)
      }
      for (const id of [...lives.current.keys()]) {
        if (!value.transfers.some((t) => t.id === id)) {
          lives.current.delete(id)
          seen.delete(id)
        }
      }
      // A clipboard that just arrived: said briefly in the window (Mission 05
      // §3.1). Not on the first snapshot (that's history, not news).
      const newest = value.clips[0]
      if (newest && lastClip.current !== null && newest.id > lastClip.current) {
        sayClip(true, `${newest.private ? 'Private text' : 'Clipboard'} from ${newest.from}: ready to paste.`)
      }
      lastClip.current = newest?.id ?? lastClip.current ?? 0
      // Keep the search clock steady across snapshots (it only moves on "Try again")
      const start = now - value.searchMs
      if (Math.abs(start - searchAnchor.current) > 1500) searchAnchor.current = start
      setSnap({ value, at: searchAnchor.current + value.searchMs })
    }
    const unlisten = listen<CoreEvent>(EVENT, ({ payload }) => {
      if (!alive) return
      if (payload.type === 'snapshot') accept(payload)
      else if (payload.type === 'log') setLogs((l) => appendLog(l, payload, MAX_LOG))
      else if (payload.type === 'progress') {
        const l = live(payload.id)
        l.progress.set(payload.total ? payload.bytes / payload.total : 0)
        l.rate.set(payload.speed ?? 0)
        // Everything has crossed: the other side is checking and saving the last of it
        l.eta.set(payload.total && payload.bytes >= payload.total ? 'Finishing…' : etaWords(payload.eta))
        setUi((u) => (u.currentFile[payload.id] === payload.file ? u : { ...u, currentFile: { ...u.currentFile, [payload.id]: payload.file } }))
      } else if (payload.type === 'notice' && payload.kind === 'clipboard') {
        // Also in the window: where it is, and how to paste it
        const paste = hostSide === 'mac' ? '⌘V' : 'Ctrl+V'
        sayClip(true, `${payload.title}: ${payload.body.replace(/ready to paste\.$/, `ready to paste with ${paste}.`)}`)
      }
    })
    const unlistenUpdate = listen<CoreUpdate>(UPDATE_EVENT, ({ payload }) => alive && patchUi({ update: payload }))
    core.updateStatus().then((u) => alive && u && patchUi({ update: u }), () => {})
    core.shortcut().then((k) => alive && patchUi({ shortcut: k }), () => {})
    core.snapshot().then((s) => alive && accept(s), console.error)
    core.logs().then(
      (history) => alive && setLogs((cur) => [...history.reduce((runs: LogRun[], l) => appendLog(runs, l, MAX_LOG), []), ...cur].slice(-MAX_LOG)),
      console.error,
    )
    isEnabled().then((v) => alive && patchUi({ openAtLogin: v }), () => {})
    return () => {
      alive = false
      void unlisten.then((f) => f())
      void unlistenUpdate.then((f) => f())
    }
  }, [patchUi, sayClip, live])

  // Follow the system appearance unless the dev panel picks one
  useEffect(() => {
    const mq = window.matchMedia?.('(prefers-color-scheme: dark)')
    if (!mq) return
    const onChange = () => setUi((u) => ({ ...u, dev: { ...u.dev, theme: systemTheme() } }))
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }, [])
  // Settings → Appearance: System follows the OS (and the dev panel's pick)
  const appearance = snap?.value.me.appearance ?? 'system'
  useThemeEffect(appearance === 'system' ? ui.dev.theme : appearance)

  // The arrival moments' clock also moves when a moment's time is up
  const transfers = snap?.value.transfers
  useEffect(() => {
    const untils = Object.values(shown)
      .map((x) => x.until)
      .filter((u): u is number => u !== null)
    if (!untils.length || !transfers) return
    const until = Math.min(...untils)
    // Never earlier than the deadline: the wall clock can step back, or a timer fire a little early
    const id = window.setTimeout(() => {
      const now = Math.max(Date.now(), until)
      setShown((prev) => {
        const next: Record<string, Shown> = {}
        for (const t of transfers) next[t.id] = moment({ id: t.id, phase: t.phase }, prev[t.id]?.clock ?? startClock(), now)
        return next
      })
    }, Math.max(0, until - Date.now()))
    return () => window.clearTimeout(id)
  }, [shown, transfers])
  // Landing without a bar: the receiving edge fills quickly, then the flash
  const landing = transfers?.find((t) => t.phase === 'done' && shown[t.id]?.phase === 'running' && !shown[t.id]?.bar)?.id ?? null
  const reducedMotion = ui.dev.reducedMotion
  useEffect(() => {
    if (!landing) return
    const progress = live(landing).progress
    if (reducedMotion || window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) {
      progress.set(1)
      return
    }
    const controls = animate(progress, [0, 1], { duration: 0.6, ease: [0.16, 1, 0.3, 1] })
    return () => controls.stop()
  }, [landing, reducedMotion, live])

  const world = useMemo(() => {
    const value = snap?.value ?? null
    const held = (t: CoreTransfer) => shown[t.id]?.phase && shown[t.id].phase !== t.phase
    const patched = value && {
      ...value,
      transfers: value.transfers.map((t) => (held(t) ? { ...t, phase: shown[t.id].phase! } : t)),
    }
    const w = buildWorld(patched, logs, ui, snap?.at ?? mountedAt)
    return {
      ...w,
      sessions: w.sessions.map((s) => {
        const t = value?.transfers.find((x) => x.id === s.id)
        const sh = shown[s.id]
        const isHeld = !!t && !!sh?.phase && sh.phase !== t.phase
        const brief = !!sh && sh.phase === 'running' && !sh.bar
        return isHeld || brief ? { ...s, brief, held: isHeld } : s
      }),
    }
  }, [snap, logs, ui, mountedAt, shown])
  const latest = useRef(world)
  useEffect(() => {
    latest.current = world
  })

  /** Sends the paths to the chosen devices: one transfer each. Says per
      device what didn't start; true if any did. */
  const sendPaths = useCallback(
    (paths: string[]) => {
      // Ticked and connected; a busy one gets them next (the core queues it)
      const { to: ready, busy, away } = targets(latest.current)
      const to = [...ready, ...busy].map((p) => p.id)
      if (paths.length === 0) return Promise.resolve(false)
      if (!to.length) {
        const why = away.length
            ? `Not sent: ${names(away.map((p) => p.name))} ${away.length > 1 ? 'aren’t' : 'isn’t'} connected right now.`
            : 'Not sent: tick a computer to send to first.'
        patchUi({ sendErrors: [why] })
        return Promise.resolve(false)
      }
      // The core decides whether sending is allowed (pairing, Files from, busy)
      return core.send(to, paths).then(
        (results) => {
          const errors = results.filter((r) => r.error).map((r) => r.error!)
          patchUi({ sendErrors: errors })
          return results.some((r) => !r.error)
        },
        (e: unknown) => {
          patchUi({ sendErrors: [String(e)] })
          return false
        },
      )
    },
    [patchUi],
  )

  /** Adds paths to the send list (once each), then asks the core what they hold */
  const queuePaths = useCallback((paths: string[]) => {
    const fresh = [...new Set(paths)].filter((p) => !latest.current.queue.some((q) => q.path === p))
    if (!fresh.length) return
    const added: QueuedItem[] = fresh.map((path) => ({ path, name: path.split(/[\\/]/).filter(Boolean).pop() ?? path, kind: null }))
    setUi((u) => ({ ...u, queue: [...u.queue, ...added] }))
    core.describe(fresh).then(
      (found) =>
        setUi((u) => ({
          ...u,
          queue: u.queue.map((q) => {
            const i = fresh.indexOf(q.path)
            if (i < 0) return q
            const d = found[i]
            return d
              ? { ...q, name: d.name, kind: d.folder ? 'folder' : 'file', bytes: d.bytes, count: d.folder ? d.files : undefined }
              : { ...q, kind: 'file', unusable: true }
          }),
        })),
      (e: unknown) => console.error('describe', e),
    )
  }, [])

  const dispatch = useCallback(
    (a: Action) => {
      const report = (e: unknown) => console.error(a.type, e)
      switch (a.type) {
        case 'setMode':
          patchUi({ mode: a.mode })
          return
        case 'setTarget':
          patchUi({ target: a.ids })
          void core.saveSettings({ sendTo: a.ids }).catch(report)
          return
        case 'openPanel':
          patchUi({ panel: a.panel, helpTopic: a.topic, connect: { busy: false } })
          // Pair another computer: look again, so every computer nearby shows (Mission 05 §1.8)
          if (a.panel === 'add') void core.retry().catch(report)
          return
        case 'connect':
          patchUi({ connect: { busy: true } })
          core.connect(a.address).then(
            (found) => {
              const host = found.addr?.replace(/:\d+$/, '') ?? a.address
              if (found.paired) {
                // Nothing to pair: say so, so nobody wonders whether pairing was lost
                patchUi({ panel: null, connect: { busy: false } })
                sayClip(true, `Reached ${found.name} at ${host}. It’s already paired.`)
              } else {
                // With other devices paired, the list of nearby computers has its Pair key
                const others = latest.current.peers.some((p) => p.paired)
                patchUi({ panel: others ? 'add' : null, connect: { busy: false } })
              }
            },
            (e: unknown) => patchUi({ connect: { busy: false, error: String(e) } }),
          )
          return
        case 'retrySearch':
          void core.retry().catch(report)
          return
        case 'pair':
          patchUi({ panel: null })
          void core.pair(a.id).catch(report)
          return
        case 'pairing': {
          const pr = latest.current.pairing
          const who = pr && latest.current.peers.find((p) => p.id === pr.peer)
          void core.pairing({ type: a.action }).then(() => {
            if (a.action === 'ignore' && pr) {
              // Say what Ignore did, and remember it so the next screen doesn't push Pair
              setUi((u) => ({ ...u, ignored: [...(u.ignored ?? []), pr.peer] }))
              sayClip(true, `Ignored ${who?.name ?? 'the request'} (${pr.peerAddr}). Nothing was paired.`)
            } else if (a.action === 'allowAgain') {
              sayClip(true, 'Pairing requests are allowed again.')
            }
          }, report)
          return
        }
        case 'pairCode':
          void core.pairing({ type: 'code', code: a.code }).catch(report)
          return
        case 'setPeerIncoming':
          void core.setIncoming(a.id, a.value).catch(report)
          return
        case 'setConnected':
          void core.setConnected(a.id, a.value).catch(report)
          return
        case 'renamePeer':
          void core.renamePeer(a.id, a.name).catch(report)
          return
        case 'removePeer':
          void core.removePeer(a.id).catch(report)
          return
        case 'saveSettings':
          // Test builds on: look at the beta feed now, not in four hours
          void core
            .saveSettings(a.patch)
            .then(() => (a.patch.testBuilds ? core.checkUpdates() : undefined))
            .catch(report)
          return
        case 'chooseReceiveDir':
          void open({ directory: true, multiple: false, defaultPath: latest.current.settings.receiveDir || undefined })
            .then((dir) => (typeof dir === 'string' ? core.saveSettings({ receiveDir: dir }) : undefined))
            .catch(report)
          return
        case 'setOpenAtLogin':
          void (a.value ? enable() : disable())
            .then(() => isEnabled())
            .then((v) => patchUi({ openAtLogin: v }))
            .catch(report)
          return
        case 'setAppearance':
          core.saveSettings({ appearance: a.value }).catch(report)
          return
        case 'pickFiles':
          void open({ multiple: true, directory: !!a.folders })
            .then((picked) => queuePaths(picked === null ? [] : Array.isArray(picked) ? picked : [picked]))
            .catch(report)
          return
        case 'queuePaths':
          queuePaths(a.paths)
          return
        case 'unqueue':
          setUi((u) => ({ ...u, queue: u.queue.filter((q) => q.path !== a.path) }))
          return
        case 'clearQueue':
          patchUi({ queue: [] })
          return
        case 'sendQueue': {
          const paths = latest.current.queue.filter((q) => !q.unusable).map((q) => q.path)
          // One transfer per chosen device; the list empties once any has
          // started, with what can't be sent (already said to be left out)
          void sendPaths(paths).then((ok) => ok && setUi((u) => ({ ...u, queue: u.queue.filter((q) => !paths.includes(q.path) && !q.unusable) })))
          return
        }
        case 'dismissErrors':
          patchUi({ sendErrors: [] })
          return
        case 'openReceived':
          void core.openReceived().catch(report)
          return
        case 'openFile':
          void core.openFile(a.path).catch(report)
          return
        case 'reveal':
          if (a.path) void revealItemInDir(a.path).catch(report)
          return
        case 'openUrl':
          void openUrl(a.url).catch(report)
          return
        case 'checkUpdates':
          void core.checkUpdates().catch(report)
          return
        case 'sendClipboard':
          core.sendClipboard(a.ids).then(
            (sent) => sayClip(true, sent.said, sent.files.length ? sent.files : undefined),
            (text) => sayClip(false, String(text)),
          )
          return
        case 'sendText':
          core.sendText(a.ids, a.text, a.private ?? false).then(
            (said) => sayClip(true, said),
            (e) => sayClip(false, String(e)),
          )
          return
        case 'sendPaths':
          patchUi({ clipResult: undefined })
          void sendPaths(a.paths)
          return
        case 'copyTechnicalLog':
          core.copyTechnicalLog().then(
            () => sayClip(true, 'Technical log copied. Paste it into a message for support.'),
            (e) => sayClip(false, `The technical log wasn’t copied: ${String(e)}`),
          )
          return
        case 'sendFeedback': {
          // The log as a file to attach (a mailto link can't attach it), shown
          // in Finder / Explorer; then the mail, once the address is confirmed
          const me = latest.current.settings
          const mail = feedbackMail({ version: me.appVersion, os: me.osVersion, withLog: a.withLog })
          const saved = a.withLog || !mail ? core.saveTechnicalLog() : Promise.resolve('')
          saved.then(
            (path) => {
              if (path) void revealItemInDir(path).catch(() => {})
              if (mail) void openUrl(mail).catch(report)
              else sayClip(true, `Feedback by email isn’t set up yet. The technical log is saved in Downloads${path ? `: ${path.split(/[\\/]/).pop()}` : ''}.`)
            },
            (e) => sayClip(false, `The technical log wasn’t saved: ${String(e)}`),
          )
          return
        }
        case 'saveTechnicalLog':
          core.saveTechnicalLog().then(
            (path) => revealItemInDir(path).catch(() => sayClip(true, `Technical log saved: ${path}`)),
            (e) => sayClip(false, `The technical log wasn’t saved: ${String(e)}`),
          )
          return
        case 'copyClip': {
          const priv = latest.current.clipboard?.items.some((c) => c.id === a.id && c.private)
          core.copyClip(a.id).then(
            () => sayClip(true, priv ? 'On the clipboard again, kept out of clipboard history.' : 'On the clipboard again, ready to paste.'),
            (text) => sayClip(false, String(text)),
          )
          return
        }
        case 'clearClips': {
          const n = latest.current.clipboard?.items.length ?? 0
          core.clearClips().then(
            () => sayClip(true, `Shelf cleared (${n === 1 ? '1 item' : `${n} items`}). Your clipboard is as it was.`),
            report,
          )
          return
        }
        case 'removeClip':
          void core.removeClip(a.id).catch(report)
          return
        // The Shelf (Mission 05 §3.2); its messages share the clipboard's line
        case 'shelfAdd':
          core.shelfAdd(a.name, a.text, a.private).then(
            () => sayClip(true, a.private ? 'Kept on the Shelf, private.' : 'Kept on the Shelf.'),
            (e) => sayClip(false, String(e)),
          )
          return
        case 'shelfRename':
          core.shelfEdit(a.id, a.name, null).catch((e) => sayClip(false, String(e)))
          return
        case 'shelfRemove': {
          void core.shelfRemove(a.ids).catch(report)
          const until = Date.now() + 8000
          patchUi({ shelfUndoUntil: until })
          window.setTimeout(() => setUi((u) => (u.shelfUndoUntil === until ? { ...u, shelfUndoUntil: undefined } : u)), 8000)
          return
        }
        case 'shelfUndo':
          patchUi({ shelfUndoUntil: undefined })
          core.shelfUndo().catch((e) => sayClip(false, String(e)))
          return
        case 'shelfCopy': {
          const priv = latest.current.shelf.some((i) => i.id === a.id && i.private)
          core.shelfCopy(a.id).then(
            () => sayClip(true, priv ? 'Copied, kept out of clipboard history.' : 'Copied. Paste it anywhere.'),
            (e) => sayClip(false, String(e)),
          )
          return
        }
        case 'shelfReveal':
          core.shelfReveal(a.id).then((text) => {
            if (text === null) return
            patchUi({ revealed: { id: a.id, text } })
            // Hidden again after a few seconds; never kept anywhere else
            window.setTimeout(() => setUi((u) => (u.revealed?.id === a.id ? { ...u, revealed: undefined } : u)), 6000)
          }, report)
          return
        case 'setPicture':
          void core.setPicture(a.picture).catch(report)
          return
        case 'setShelfShare':
          void core.setShelfShare(a.id, a.on).catch(report)
          return
        case 'shelfPutMarked':
          core.shelfPutMarked(a.count).then(
            () => sayClip(true, 'Kept on the Shelf, private.'),
            (e) => sayClip(false, String(e)),
          )
          return
        case 'shelfPutReceived': {
          const priv = latest.current.clipboard?.items.some((c) => c.id === a.id && c.private)
          core.shelfPutReceived(a.id).then(
            () => sayClip(true, priv ? 'Kept on the Shelf, private.' : 'Kept on the Shelf.'),
            (e) => sayClip(false, String(e)),
          )
          return
        }
        case 'saveClipImage': {
          const d = new Date()
          const p2 = (n: number) => String(n).padStart(2, '0')
          const name = `Clipboard image ${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())} ${p2(d.getHours())}.${p2(d.getMinutes())}.${p2(d.getSeconds())}`
          core.saveClipImage(a.id, name).then(
            (path) => {
              sayClip(true, `Saved as ${path.split(/[\\/]/).pop()} in your received files folder.`)
              void revealItemInDir(path).catch(report)
            },
            (e) => sayClip(false, String(e)),
          )
          return
        }
        case 'dismissClipNotice':
          void core.dismissClipNotice().catch(report)
          return
        case 'setClipSync':
          void core.setClipSync(a.id, a.on).catch(report)
          return
        case 'clipSyncOff':
          void core.clipSyncOff().catch(report)
          return
        case 'pauseClipSync':
          void core.pauseClipSync(a.minutes).catch(report)
          return
        case 'resumeClipSync':
          void core.resumeClipSync().catch(report)
          return
        case 'setClipTo':
          void core.setClipTo(a.id, a.to).catch(report)
          return
        case 'setShortcut':
          core.setShortcut(a.shortcut).then(
            (k) => patchUi({ shortcut: k }),
            // Name the one that was refused, and the one that still works
            () =>
              setUi((u) => {
                if (!u.shortcut) return u
                const tried = shortcutLabel(a.shortcut, hostSide)
                const kept = shortcutLabel(u.shortcut.shortcut, hostSide)
                const error = `${tried} can’t be used: another app has it, or the system keeps it. ${kept} still works.`
                return { ...u, shortcut: { ...u.shortcut, error } }
              }),
          )
          return
        case 'restartToUpdate':
          void core.restartToUpdate().catch(report)
          return
        case 'updateLater':
          void core.updateLater().catch(report)
          return
        case 'accept':
          void core.accept(a.id, !!a.always).catch(report)
          return
        case 'decline':
          void core.decline(a.id).catch(report)
          return
        case 'fixFirewall':
          patchUi({ fix: { busy: true } })
          core.fixFirewall().then(
            (text) => patchUi({ fix: { busy: false, ok: true, text } }),
            (text) => patchUi({ fix: { busy: false, ok: false, text: String(text) } }),
          )
          return
        case 'sendAgain':
          core.sendAgain(a.id).catch((e: unknown) => patchUi({ sendErrors: [String(e)] }))
          return
        case 'cancel':
          void core.cancel(a.id).catch(report)
          return
        case 'continue':
          void core.continueTransfer(a.id).catch(report)
          return
        case 'unqueueNext':
          void core.unqueueNext(a.id).catch(report)
          return
        case 'continueNext':
          void core.continueNext(a.peer).catch(report)
          return
        case 'dismiss':
          void core.dismiss(a.id).catch(report)
          return
        case 'dev':
          setUi((u) => {
            const dev = { ...u.dev, ...a.patch }
            saveDev(dev)
            return { ...u, dev }
          })
          return
      }
    },
    [patchUi, sendPaths, queuePaths, sayClip],
  )

  // Native drag and drop: hover state for the drop zone; dropped paths join the send list
  useEffect(() => {
    const off = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === 'enter' || payload.type === 'over') {
        if (!latest.current.dragOver) patchUi({ dragOver: true })
      } else if (payload.type === 'leave') {
        patchUi({ dragOver: false })
      } else if (payload.type === 'drop') {
        patchUi({ dragOver: false })
        if (latest.current.peers.some((p) => p.paired && p.status === 'connected')) queuePaths(payload.paths)
      }
    })
    return () => void off.then((f) => f())
  }, [patchUi, queuePaths])

  return (
    <PortalContext value={{ world, dispatch, live }}>
      <MotionConfig reducedMotion={ui.dev.reducedMotion ? 'always' : 'user'}>{children}</MotionConfig>
    </PortalContext>
  )
}
