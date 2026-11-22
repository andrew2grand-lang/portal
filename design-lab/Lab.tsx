import { useCallback, useEffect, useState } from 'react'
import { ArrowUpIcon, FolderIcon, FolderOpenIcon, MagnifyingGlassIcon, SidebarSimpleIcon } from '@phosphor-icons/react'
import { Checkbox } from '../src/components/Checkbox/Checkbox'
import { Key } from '../src/components/Key/Key'
import { other } from './twoDevices'
import type { LogLine, Side, Theme } from '../src/engine/types'
import { LabProvider } from './LabProvider'
import { LabWindow } from './LabWindow'
import type { TargetOs } from './report'
import { osName } from './report'
import { scenarioById, scenarios } from './scenarios'
import { receiverStates, screenById, screens } from './screens'
import type { ScreenCtx } from './screens/types'
import { browse } from './useRealScan'
import type { Folder } from './useRealScan'
import { useRealScan } from './useRealScan'
import s from './lab.module.css'

type WindowChoice = 'auto' | 'mac' | 'win' | 'both'

interface LabState {
  screen: string
  state: string
  windows: WindowChoice
  theme: Theme
  scenario: string
  reduced: boolean
  panel: boolean
}

/** Everything the lab shows is in the URL, so any state can be linked or screenshotted */
function readUrl(): LabState {
  const q = new URLSearchParams(location.search)
  const screen = screenById(q.get('screen'))
  const state = screen.states.find((x) => x.id === q.get('state'))?.id ?? screen.states[0].id
  const windows = (['auto', 'mac', 'win', 'both'] as const).find((w) => w === q.get('os')) ?? 'auto'
  return {
    screen: screen.id,
    state,
    windows,
    theme: q.get('theme') === 'dark' ? 'dark' : 'light',
    scenario: scenarioById(q.get('scenario')).id,
    reduced: q.get('motion') === 'reduced',
    panel: q.get('panel') !== 'hidden',
  }
}

function writeUrl(l: LabState) {
  const q = new URLSearchParams({ screen: l.screen, state: l.state, os: l.windows, theme: l.theme, scenario: l.scenario })
  if (l.reduced) q.set('motion', 'reduced')
  if (!l.panel) q.set('panel', 'hidden')
  history.replaceState(null, '', `?${q}`)
}

function Segment<T extends string>({ label, value, options, onChange }: {
  label: string
  value: T
  options: { value: T; label: string }[]
  onChange: (v: T) => void
}) {
  return (
    <div className={s.segment} role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className={s.segmentOption}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  )
}

/** Type a path or walk the folders; the bridge lists folder names only */
function RealScanPanel({ busy, onScan }: { busy: boolean; onScan: (path: string, target: TargetOs, online: boolean) => void }) {
  const [path, setPath] = useState('')
  const [target, setTarget] = useState<TargetOs>('windows')
  const [online, setOnline] = useState(false)
  const [folder, setFolder] = useState<Folder | null>(null)
  const open = async (p: string) => {
    const f = await browse(p)
    if (f) {
      setFolder(f)
      setPath(f.path)
    }
  }
  return (
    <section className={s.block} aria-labelledby="real-scan">
      <h2 id="real-scan" className={s.blockTitle}>
        Scan a real folder
      </h2>
      <p className={s.hint}>Runs the real portal-scan on this computer. It reads the folder and runs nothing.</p>
      <div className={s.pathRow}>
        <input
          className={s.input}
          value={path}
          onChange={(e) => setPath(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && path && onScan(path, target, online)}
          placeholder="~/Projects/my-app"
          spellCheck={false}
          aria-label="Folder path"
        />
        <Key size="sm" icon={<FolderOpenIcon />} aria-label="Browse folders" onClick={() => open(path || '~')} />
      </div>
      {folder && (
        <div className={s.browser}>
          <div className={s.browserHead}>
            <Key size="sm" icon={<ArrowUpIcon />} aria-label="Up one folder" disabled={!folder.parent} onClick={() => folder.parent && open(folder.parent)} />
            <span className={s.browserPath} title={folder.path}>
              {folder.path.replace(folder.home, '~')}
            </span>
          </div>
          <ul className={s.browserList}>
            {folder.entries.length === 0 && <li className={s.hint}>No folders inside.</li>}
            {folder.entries.map((e) => (
              <li key={e.path}>
                <button type="button" className={s.browserItem} onClick={() => open(e.path)}>
                  <FolderIcon aria-hidden /> {e.name}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
      <Segment
        label="Check for"
        value={target}
        options={[
          { value: 'windows', label: 'For Windows' },
          { value: 'macos', label: 'For macOS' },
        ]}
        onChange={setTarget}
      />
      <Checkbox checked={online} onChange={setOnline}>
        Check packages on PyPI (online)
      </Checkbox>
      <Key variant="ink" size="sm" icon={<MagnifyingGlassIcon />} disabled={!path || busy} onClick={() => onScan(path, target, online)}>
        {busy ? 'Scanning…' : 'Scan'}
      </Key>
    </section>
  )
}

export function Lab() {
  const [lab, setLab] = useState<LabState>(readUrl)
  const { real, start } = useRealScan()
  const set = useCallback((patch: Partial<LabState>) => setLab((l) => ({ ...l, ...patch })), [])
  const go = useCallback(
    (screen: string, state?: string) => set({ screen, state: state ?? screenById(screen).states[0].id }),
    [set],
  )
  useEffect(() => writeUrl(lab), [lab])

  // "\" hides the panel for clean screenshots
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === '\\' && !(e.target instanceof HTMLInputElement)) set({ panel: !lab.panel })
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [lab.panel, set])

  // A finished real scan moves on to its report after a beat, so the ticked steps are seen
  useEffect(() => {
    if (real.status !== 'done') return
    const id = window.setTimeout(() => setLab((l) => (l.screen === 'scanning' && l.state === 'real' ? { ...l, screen: 'report' } : l)), 700)
    return () => window.clearTimeout(id)
  }, [real.status])

  const scenario = scenarioById(lab.scenario)
  const screen = screenById(lab.screen)
  const scanReal = (path: string, target: TargetOs, online: boolean) => {
    start(path, target, online)
    set({ screen: 'scanning', state: 'real' })
  }

  // The natural window: the sender's for scans, the receiver's for conversions
  const onReceiver = screen.on === 'receiver' || receiverStates.has(`${screen.id}:${lab.state}`)
  const realFrom: Side | null = lab.state === 'real' ? (real.target === 'windows' ? 'mac' : 'win') : null
  const natural: Side = realFrom ?? (onReceiver ? other(scenario.from) : scenario.from)
  const sides: Side[] = lab.windows === 'both' ? ['mac', 'win'] : [lab.windows === 'auto' ? natural : lab.windows]

  const ctxFor = (side: Side): ScreenCtx => ({
    scenario,
    screen: screen.id,
    side,
    state: lab.state,
    go,
    real,
    startDemoScan: () => go('scanning', 'live'),
    checkOnline: real.path ? () => scanReal(real.path, real.target, true) : undefined,
  })
  const lines = (side: Side): LogLine[] => (screen.log?.(ctxFor(side)) ?? []).map((l, i) => ({ ...l, id: i + 1 }))
  const logs = { mac: lines('mac'), win: lines('win') }

  const arrival = screen.id === 'final' && lab.state === 'summary' ? 'final' : null

  return (
    <div className={s.lab} data-panel={lab.panel ? '' : undefined}>
      {lab.panel ? (
        <aside className={s.panel} aria-label="Design lab">
          <div className={s.panelHead}>
            <h1 className={s.labTitle}>Project Send lab</h1>
            <Key size="sm" icon={<SidebarSimpleIcon />} aria-label="Hide the lab panel (\)" onClick={() => set({ panel: false })} />
          </div>
          <section className={s.block}>
            <Segment
              label="Window"
              value={lab.windows}
              options={[
                { value: 'auto', label: 'Auto' },
                { value: 'mac', label: 'macOS' },
                { value: 'win', label: 'Windows' },
                { value: 'both', label: 'Both' },
              ]}
              onChange={(windows) => set({ windows })}
            />
            <Segment
              label="Theme"
              value={lab.theme}
              options={[
                { value: 'light', label: 'Light' },
                { value: 'dark', label: 'Dark' },
              ]}
              onChange={(theme) => set({ theme })}
            />
            <Checkbox checked={lab.reduced} onChange={(reduced) => set({ reduced })}>
              Reduced motion
            </Checkbox>
          </section>
          <section className={s.block} aria-labelledby="scenario">
            <h2 id="scenario" className={s.blockTitle}>
              Demo scenario
            </h2>
            <select className={s.select} value={lab.scenario} onChange={(e) => set({ scenario: e.target.value })} aria-labelledby="scenario">
              {scenarios.map((sc) => (
                <option key={sc.id} value={sc.id}>
                  {sc.label}
                </option>
              ))}
            </select>
            <p className={s.hint}>
              Report: real scanner output for design-lab/projects/{scenario.report.project.name}, checked for{' '}
              {osName[scenario.report.target]}.
            </p>
            {sides.some((x) => x !== natural) && (
              <p className={s.hint}>
                This screen belongs on {natural === 'mac' ? 'the Mac' : 'the PC'} in this scenario. The other window
                style shows the design; for its own story, pick a scenario that starts on the other computer.
              </p>
            )}
          </section>
          <nav className={s.block} aria-labelledby="screens">
            <h2 id="screens" className={s.blockTitle}>
              Screens
            </h2>
            <ol className={s.index}>
              {screens.map((sc) => (
                <li key={sc.id}>
                  <span className={s.indexScreen} data-current={sc.id === lab.screen ? '' : undefined}>
                    {sc.title}
                  </span>
                  <ul className={s.indexStates}>
                    {sc.states.map((st) => {
                      const current = sc.id === lab.screen && st.id === lab.state
                      const unavailable = st.needsReal && !real.report && real.status !== 'running'
                      return (
                        <li key={st.id}>
                          <button
                            type="button"
                            className={s.indexState}
                            aria-current={current ? 'true' : undefined}
                            disabled={unavailable}
                            title={unavailable ? 'Scan a real folder first' : undefined}
                            onClick={() => set({ screen: sc.id, state: st.id })}
                          >
                            {st.label}
                          </button>
                        </li>
                      )
                    })}
                  </ul>
                </li>
              ))}
            </ol>
          </nav>
          <RealScanPanel busy={real.status === 'running'} onScan={scanReal} />
          <p className={s.hint}>
            Demo only: conversion, AI and accounts don’t exist yet. Nothing here ships in the app. Press \ to hide this panel.
          </p>
        </aside>
      ) : (
        <button type="button" className={s.reveal} onClick={() => set({ panel: true })} aria-label="Show the lab panel">
          <SidebarSimpleIcon aria-hidden />
        </button>
      )}
      <LabProvider theme={lab.theme} reducedMotion={lab.reduced} logs={logs}>
        <main className={s.canvas} data-count={sides.length}>
          <h1 className="sr-only">
            {screen.title}: {screen.states.find((x) => x.id === lab.state)?.label}
          </h1>
          {sides.map((side) => {
            const ctx = ctxFor(side)
            return (
              <LabWindow
                key={side}
                side={side}
                viewKey={`${screen.id}:${lab.state}:${scenario.id}`}
                glow={screen.glow?.(ctx) ?? 'off'}
                arrival={arrival}
                settingsOpen={screen.id === 'accounts'}
                onMode={(m) => m === 'files' && go('entry', 'drop')}
              >
                {screen.render(ctx)}
              </LabWindow>
            )
          })}
        </main>
      </LabProvider>
    </div>
  )
}
