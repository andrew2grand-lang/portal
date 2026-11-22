import { PictureChooser } from '../picture/PictureChooser'
import { useEffect, useState } from 'react'
import { FolderOpenIcon, GlobeSimpleIcon, PlusIcon } from '@phosphor-icons/react'
import { Checkbox } from '../../components/Checkbox/Checkbox'
import { Choice } from '../../components/Choice/Choice'
import { Key } from '../../components/Key/Key'
import { TextField } from '../../components/TextField/TextField'
import { usePortal } from '../../engine/context'
import type { Appearance, ClipboardInfo, Incoming, Peer, UpdateInfo } from '../../engine/types'
import { names } from '../../engine/words'
import { DeviceIcon } from '../PortalWindow'
import { acceleratorFrom, shortcutLabel } from '../../engine/shortcut'
import { restartLabel, updateLine } from '../../engine/updates'
import { APP_NAME } from '../../platform'
import { useSide } from '../WindowContext'
import s from './Stage.module.css'
import { Head, Pane, Section } from './parts'
import { shortFolder } from './where'
import { PairingLocked } from './Connection'
import { FeedbackKeys } from './Feedback'
import { LINKS, websiteLink } from '../../links'

/** Device name, receive folder, startup, and the trusted devices list */
export function SettingsView() {
  const { world, dispatch } = usePortal()
  const st = world.settings
  const paired = world.peers.filter((p) => p.paired)

  return (
    <Pane onClose={() => dispatch({ type: 'openPanel', panel: null })}>
      <Head title="Settings" />
      <Section title="This computer">
        <NameField key={st.deviceName} saved={st.deviceName} />
        <PictureChooser />
        <ReceivedFiles />
        <Checkbox checked={st.skipJunk} onChange={(skipJunk) => dispatch({ type: 'saveSettings', patch: { skipJunk } })}>
          Skip system files like .DS_Store and Thumbs.db
        </Checkbox>
        <Checkbox checked={st.openAtLogin} onChange={(value) => dispatch({ type: 'setOpenAtLogin', value })}>
          Open at login
        </Checkbox>
        <div className={s.incoming}>
          <p className={s.incomingLabel}>Appearance</p>
          <Choice
            id="appearance"
            label="Appearance"
            value={world.appearance}
            options={appearanceOptions}
            onChange={(value) => dispatch({ type: 'setAppearance', value })}
          />
        </div>
      </Section>

      <Section title="Paired devices">
        <PairingLocked />
        {/* First, not after every device's options: it's what people come here for */}
        <div className={s.deviceKeys}>
          <Key size="sm" icon={<PlusIcon />} onClick={() => dispatch({ type: 'openPanel', panel: 'add' })}>
            Pair another computer
          </Key>
        </div>
        {paired.length === 0 ? (
          <p className={s.aside}>None yet. Devices appear here once you pair them.</p>
        ) : (
          paired.map((d) => (
            <DeviceRow
              key={d.id}
              device={d}
              shelved={world.shelf.filter((i) => i.from === d.id).length}
            />
          ))
        )}
      </Section>

      {st.addresses.length > 0 && (
        <Section title="This computer’s addresses">
          <dl className={s.addresses}>
            {st.addresses.map((a) => (
              <div key={a.address} style={{ display: 'contents' }}>
                <dt>{a.label}</dt>
                <dd>{a.address}</dd>
              </div>
            ))}
          </dl>
        </Section>
      )}

      {world.clipboard && <ClipboardSection clip={world.clipboard} />}

      <Section title="Connection">
        <div className={s.deviceKeys}>
          <Key size="sm" onClick={() => dispatch({ type: 'openPanel', panel: 'doctor' })}>
            Connection check
          </Key>
          <Key size="sm" onClick={() => dispatch({ type: 'openPanel', panel: 'connect' })}>
            Connect by IP
          </Key>
        </div>
      </Section>

      {world.update && <AboutSection update={world.update} />}
    </Pane>
  )
}

/** The shared clipboard: its shortcut, where it goes, and the privacy line */
function ClipboardSection({ clip }: { clip: ClipboardInfo }) {
  const side = useSide()
  const { world, dispatch } = usePortal()
  const to = world.peers.filter((p) => p.paired && world.target.includes(p.id)).map((p) => p.name)
  const several = world.peers.filter((p) => p.paired).length > 1
  const [recording, setRecording] = useState(false)
  useEffect(() => {
    if (!recording) return
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault()
      e.stopPropagation()
      if (e.key === 'Escape') return setRecording(false)
      const shortcut = acceleratorFrom(e, side)
      if (shortcut) {
        setRecording(false)
        dispatch({ type: 'setShortcut', shortcut })
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [recording, side, dispatch])
  return (
    <Section title="Clipboard">
      <div className={s.fieldRow}>
        <div>
          <p className={s.deviceMeta}>Send clipboard shortcut</p>
          <p className={s.aboutName} aria-live="polite">
            {recording ? 'Press the new shortcut (Esc keeps this one)…' : shortcutLabel(clip.shortcut, side)}
          </p>
          {!recording && side === 'mac' && <p className={s.deviceMeta}>{spelledOut(clip.shortcut)}</p>}
        </div>
        <Key size="sm" onClick={() => setRecording((r) => !r)}>
          {recording ? 'Cancel' : 'Change…'}
        </Key>
        {!recording && clip.defaultShortcut && clip.shortcut !== clip.defaultShortcut && (
          <Key size="sm" onClick={() => dispatch({ type: 'setShortcut', shortcut: '' })}>
            Use {shortcutLabel(clip.defaultShortcut, side)}
          </Key>
        )}
      </div>
      {clip.shortcutError && (
        <p className={s.clipResult} data-tone="warn" role="status">
          {clip.shortcutError}
        </p>
      )}
      {to.length > 0 && <p className={s.aside}>Sends to {several ? 'the ticked computers' : names(to)}.</p>}
      <Key size="sm" onClick={() => dispatch({ type: 'openPanel', panel: 'clipboard' })}>
        Automatic sync…
      </Key>
    </Section>
  )
}

/** "Control + Option + Command + C": for anyone who doesn't read ⌃⌥⌘ */
function spelledOut(accelerator: string): string {
  const names: Record<string, string> = { Control: 'Control', Alt: 'Option', Shift: 'Shift', Super: 'Command' }
  return accelerator
    .split('+')
    .map((p) => names[p] ?? p.replace(/^(Key|Digit)/, ''))
    .join(' + ')
}

/** Placeholder until the site exists (Mission 03 §6: to be confirmed) */
/** The website: a link once confirmed; until then the address as text,
    marked "not live yet" (we don't own it; Mission 04 §5) */
function Website() {
  const { dispatch } = usePortal()
  const url = websiteLink()
  const shown = LINKS.website.replace(/^https:\/\//, '')
  if (!url) {
    return (
      <p className={s.aside}>
        Website: {shown} (not live yet)
      </p>
    )
  }
  return (
    <div className={s.deviceKeys}>
      <Key size="sm" icon={<GlobeSimpleIcon />} onClick={() => dispatch({ type: 'openUrl', url })}>
        {shown}
      </Key>
    </div>
  )
}

/** Version, updates and the website (Mission 03 §6) */
function AboutSection({ update: u }: { update: UpdateInfo }) {
  const { dispatch } = usePortal()
  const busy = ['checking', 'downloading', 'installing'].includes(u.state)
  const offer = u.state === 'ready' || u.state === 'available'
  return (
    <Section title="About">
      <div className={s.fieldRow}>
        <div>
          <p className={s.aboutName}>
            {APP_NAME} {u.current}
          </p>
          <p className={s.deviceMeta} aria-live="polite">
            {updateLine(u)}
          </p>
        </div>
        {offer ? (
          <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'restartToUpdate' })}>
            {restartLabel(u)}
          </Key>
        ) : (
          <Key size="sm" disabled={busy} onClick={() => dispatch({ type: 'checkUpdates' })}>
            Check for updates
          </Key>
        )}
      </div>
      <Checkbox checked={u.auto} onChange={(autoUpdate) => dispatch({ type: 'saveSettings', patch: { autoUpdate } })}>
        Install updates automatically
      </Checkbox>
      {!u.auto && <p className={s.aside}>It still checks, and asks before downloading.</p>}
      <Checkbox checked={u.testBuilds} onChange={(testBuilds) => dispatch({ type: 'saveSettings', patch: { testBuilds } })}>
        Get test builds
      </Checkbox>
      <Website />
      <FeedbackKeys />
    </Section>
  )
}

/** Remounts when the saved name changes, so it never fights the user's typing */
function NameField({ saved }: { saved: string }) {
  const { dispatch } = usePortal()
  const [name, setName] = useState(saved)
  const save = () => {
    const next = name.trim()
    if (!next) setName(saved)
    else if (next !== saved) dispatch({ type: 'saveSettings', patch: { deviceName: next } })
  }
  return (
    <TextField
      label="Name"
      hint="Every computer nearby running SeamDrop sees it."
      value={name}
      maxLength={63}
      onChange={setName}
      onSubmit={save}
      onBlur={save}
    />
  )
}

/* Mission 04 §1.7: the menu bar / tray has the same switch */
const appearanceOptions: { value: Appearance; label: string }[] = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
]

const incomingOptions: { value: Incoming; label: string }[] = [
  { value: 'ask', label: 'Ask every time' },
  { value: 'always', label: 'Always accept' },
  { value: 'never', label: 'Never' },
]

const statusWords: Record<Peer['status'], string> = {
  connected: 'Connected',
  connecting: 'Connecting…',
  unreachable: 'Not reachable',
  disconnected: 'Disconnected',
  nearby: 'Not paired',
  incompatible: 'Runs another version',
  revoked: 'Removed this computer',
}

function DeviceRow({ device, shelved }: { device: Peer; shelved: number }) {
  const { dispatch } = usePortal()
  const [mode, setMode] = useState<'view' | 'rename' | 'remove'>('view')
  const [name, setName] = useState(device.name)
  const rename = () => {
    const next = name.trim()
    if (next && next !== device.name) dispatch({ type: 'renamePeer', id: device.id, name: next })
    setMode('view')
  }
  const disconnected = device.status === 'disconnected'

  return (
    <div className={s.device}>
      <span className={s.itemTile} aria-hidden>
        <DeviceIcon device={device} />
      </span>
      <div>
        <p className={s.deviceName}>{device.name}</p>
        <p className={s.deviceMeta}>
          {device.os} · {statusWords[device.status]}
        </p>
      </div>
      {mode === 'view' && (
        <div className={s.deviceKeys}>
          <Key size="sm" onClick={() => setMode('rename')}>
            Rename
          </Key>
          <Key size="sm" onClick={() => setMode('remove')}>
            Remove
          </Key>
        </div>
      )}
      <div className={s.deviceMore}>
        {mode === 'view' && (
          <div className={s.deviceOptions}>
            <div className={s.incoming}>
              <p className={s.incomingLabel}>Files from {device.name}</p>
              <Choice
                id={`incoming-${device.id}`}
                label={`Files from ${device.name}`}
                value={device.incoming}
                options={incomingOptions}
                onChange={(value) => dispatch({ type: 'setPeerIncoming', id: device.id, value })}
              />
            </div>
            {/* Clipboard sync is switched in one place only: Clipboard (Mission 05 §3.1) */}
            <Checkbox checked={!!device.shelf?.mine} onChange={(on) => dispatch({ type: 'setShelfShare', id: device.id, on })}>
              Share the Shelf with {device.name}
            </Checkbox>
            {device.shelf?.mine && !device.shelf.theirs && <p className={s.deviceMeta}>Waiting for {device.name} to agree.</p>}
            <Key size="sm" onClick={() => dispatch({ type: 'setConnected', id: device.id, value: disconnected })}>
              {disconnected ? 'Connect' : 'Disconnect'}
            </Key>
          </div>
        )}
        {mode === 'rename' && (
          <div className={s.fieldRow}>
            <TextField label="New name" value={name} onChange={setName} onSubmit={rename} maxLength={63} autoFocus />
            <Key size="sm" variant="ink" onClick={rename}>
              Save
            </Key>
            <Key size="sm" onClick={() => setMode('view')}>
              Cancel
            </Key>
          </div>
        )}
        {mode === 'remove' && (
          <div className={s.fieldRow}>
            <p className={s.deviceMeta}>
              {`Remove ${device.name}? Nothing more goes either way${shelved ? `, and its Shelf items leave` : ''}. Received files stay.`}
            </p>
            <Key size="sm" variant="ink" onClick={() => dispatch({ type: 'removePeer', id: device.id })}>
              Remove
            </Key>
            <Key size="sm" onClick={() => setMode('view')}>
              Keep
            </Key>
          </div>
        )}
      </div>
    </div>
  )
}

const receiveOptions: { value: 'loose' | 'app_folder' | 'numbered'; label: string }[] = [
  { value: 'loose', label: 'Straight in' },
  { value: 'app_folder', label: 'SeamDrop folder' },
  { value: 'numbered', label: 'New folder each time' },
]

/** Received files (Mission 05 §4): one clear choice, its folder, Change… */
function ReceivedFiles() {
  const side = useSide()
  const { world, dispatch } = usePortal()
  const st = world.settings
  const folder = shortFolder(st.receiveDir, side)
  const example =
    st.receiveMode === 'loose'
      ? `Files and folders go straight into ${folder}.`
      : st.receiveMode === 'numbered'
        ? `Each delivery gets its own folder in ${folder}, like “SeamDrop 012 – from Desk PC”.`
        : st.folderPerDevice
          ? `In ${folder}, one folder for each computer.`
          : `Everything goes into ${folder}.`
  return (
    <div className={s.receivedFiles}>
      <div className={s.incoming}>
        <p className={s.incomingLabel}>Received files</p>
        <Choice
          id="receive-mode"
          label="Received files"
          value={st.receiveMode}
          options={receiveOptions}
          onChange={(receiveMode) => dispatch({ type: 'saveSettings', patch: { receiveMode } })}
        />
      </div>
      <div className={s.fieldRow}>
        <p className={s.path}>{example}</p>
        <div className={s.deviceKeys}>
          <Key size="sm" icon={<FolderOpenIcon />} onClick={() => dispatch({ type: 'openReceived' })}>
            Open
          </Key>
          <Key size="sm" onClick={() => dispatch({ type: 'chooseReceiveDir' })}>
            Change…
          </Key>
        </div>
      </div>
      {st.receiveMode === 'app_folder' && (
        <Checkbox checked={st.folderPerDevice} onChange={(folderPerDevice) => dispatch({ type: 'saveSettings', patch: { folderPerDevice } })}>
          A folder for each computer
        </Checkbox>
      )}
      <Checkbox checked={st.keepDates} onChange={(keepDates) => dispatch({ type: 'saveSettings', patch: { keepDates } })}>
        Keep original dates
      </Checkbox>
    </div>
  )
}
