import { useEffect, useState } from 'react'
import type { Dispatch, ReactNode } from 'react'
import { ArrowsClockwiseIcon, CheckIcon, LinkBreakIcon, MagnifyingGlassIcon } from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { APP_NAME } from '../../platform'
import { TextField } from '../../components/TextField/TextField'
import { unknownPeer, usePortal } from '../../engine/context'
import type { Action, Peer } from '../../engine/types'
import { restartLabel } from '../../engine/updates'
import { nearbyRows } from '../../engine/views'
import { DeviceIcon } from '../PortalWindow'
import s from './Stage.module.css'
import { Actions, Countdown, DoctorList, Head, PairCode, Pane, StateTile, Waiting } from './parts'

/** How long the search runs before the doctor's suggestions appear (brief §6.1) */
const SUGGEST_AFTER_MS = 10_000

/** The device these screens talk about */
function useFocus() {
  const { world } = usePortal()
  return { peer: world.focus ?? unknownPeer(''), me: world.me }
}

/** Try again · Connect by IP (· Stop trying, for a paired device). Returns the
    <Actions> element itself (not a wrapper component) so Pane docks it below
    the scroll area. */
function rescueActions(dispatch: Dispatch<Action>, stop?: Peer, onTry?: () => void) {
  return (
    <Actions>
      <Key
        variant="ink"
        icon={<ArrowsClockwiseIcon />}
        onClick={() => {
          dispatch({ type: 'retrySearch' })
          onTry?.()
        }}
      >
        Try again
      </Key>
      <Key onClick={() => dispatch({ type: 'openPanel', panel: 'connect' })}>Connect by IP</Key>
      {/* Not "Stop trying" (review 05: it sounded like unpairing): it stays paired */}
      {stop && (
        <Key icon={<LinkBreakIcon />} onClick={() => dispatch({ type: 'setConnected', id: stop.id, value: false })}>
          Stop for now
        </Key>
      )}
    </Actions>
  )
}

/** Nothing found yet. After ten seconds, the doctor's suggestions instead of an
    empty screen. Keyed by the search start, so "Try again" starts it over. */
export function SearchingView() {
  const { world, dispatch } = usePortal()
  const startedAt = world.searchStartedAt
  const [long, setLong] = useState(() => Date.now() - startedAt >= SUGGEST_AFTER_MS)
  useEffect(() => {
    const left = startedAt + SUGGEST_AFTER_MS - Date.now()
    if (left <= 0) return
    const id = window.setTimeout(() => setLong(true), left)
    return () => window.clearTimeout(id)
  }, [startedAt])

  if (long) {
    return (
      <Pane>
        <Head
          title="Still looking"
          sub={`Is ${APP_NAME} open on the other computer, on this network?`}
        />
        <DoctorList checks={world.doctor} />
        {rescueActions(dispatch)}
      </Pane>
    )
  }
  return (
    <Pane align="center">
      <StateTile>
        <MagnifyingGlassIcon weight="light" />
      </StateTile>
      <Head
        title="Looking for your other computer"
        sub={`Open ${APP_NAME} on your other computer, on this network.`}
      />
      <Waiting>This usually takes a few seconds</Waiting>
    </Pane>
  )
}

/** Found, not paired yet (or the other side removed this one) */
export function NearbyView() {
  const { world, dispatch } = usePortal()
  const revoked = world.peers.find((p) => p.status === 'revoked')
  if (revoked) {
    return (
      <Pane align="center">
        <StateTile>
          <DeviceIcon device={revoked} />
        </StateTile>
        <Head title={`${revoked.name} removed this pairing.`} sub="Nothing goes between them now." />
        <Actions>
          <Key variant="ink" onClick={() => dispatch({ type: 'pair', id: revoked.id })}>
            Pair again
          </Key>
          {/* OK: just a computer nearby, nothing pushing to pair (Mission 05 §2.1) */}
          <Key onClick={() => dispatch({ type: 'removePeer', id: revoked.id })}>OK</Key>
        </Actions>
      </Pane>
    )
  }
  return (
    <Pane>
      <Head title="Computers nearby" sub="Pair with your own. It shows a code; you type it here." />
      <NearbyList />
      <Actions>
        <Key onClick={() => dispatch({ type: 'openPanel', panel: 'connect' })}>Connect by IP</Key>
      </Actions>
    </Pane>
  )
}

/** Every computer nearby with its name, system and address, each with Pair
    (Mission 05 §1.8, §2.2); paired ones say so */
function NearbyList() {
  const { world, dispatch } = usePortal()
  const rows = nearbyRows(world)
  if (!rows.some((r) => !r.paired)) return <Waiting>Looking on this network…</Waiting>
  return (
    <ul className={s.rows} aria-label="Nearby computers">
      {rows.map(({ peer: p, paired }) => {
        const ignored = world.ignored.includes(p.id)
        return (
          <li key={p.id} className={s.row}>
            <span className={s.itemTile} aria-hidden>
              <DeviceIcon device={p} />
            </span>
            <div className={s.rowText}>
              <p className={s.rowTitle}>{p.name}</p>
              <p className={s.rowMeta}>
                {p.os}
                {/* Two computers can share a name: the address tells them apart */}
                {p.addr ? ` · ${p.addr.replace(/:\d+$/, '')}` : ''}
                {p.status === 'revoked' ? ' · removed this computer' : p.status === 'incompatible' ? ' · runs another version' : ignored ? ' · you ignored its request' : ''}
              </p>
            </div>
            <div className={s.rowKeys}>
              {paired ? (
                <span className={s.pairedMark}>
                  <CheckIcon weight="bold" aria-hidden /> Paired
                </span>
              ) : (
                <Key size="sm" variant={ignored ? 'plain' : 'ink'} disabled={p.status === 'incompatible'} onClick={() => dispatch({ type: 'pair', id: p.id })}>
                  Pair
                </Key>
              )}
            </div>
          </li>
        )
      })}
    </ul>
  )
}

/** Pair another computer (with devices already paired): what's nearby */
export function AddView() {
  const { dispatch } = usePortal()
  const close = () => dispatch({ type: 'openPanel', panel: null })
  return (
    <Pane onClose={close}>
      <Head title="Pair another computer" sub={`Open ${APP_NAME} on it, on this network.`} />
      <PairingLocked />
      <NearbyList />
      <Actions>
        <Key onClick={() => dispatch({ type: 'openPanel', panel: 'connect' })}>Connect by IP</Key>
        <Key onClick={close}>Done</Key>
      </Actions>
    </Pane>
  )
}

/** Too many wrong codes were typed for this computer: pairing requests are
    turned down until the person here allows them again (Mission 04 §2) */
export function PairingLocked() {
  const { world, dispatch } = usePortal()
  const st = world.settings
  if (!st.pairingLocked) return null
  const last = st.pairingLockedBy ? ` Last from ${st.pairingLockedBy}${st.pairingLockedAt ? ` at ${st.pairingLockedAt}` : ''}.` : ''
  return (
    <div className={s.locked} role="status">
      <p>
        <strong>Pairing is locked: someone typed three wrong codes.</strong> Nothing was paired.{last} Allow it again only to pair your own computer.
      </p>
      <Key size="sm" onClick={() => dispatch({ type: 'pairing', action: 'allowAgain' })}>
        Allow pairing again
      </Key>
    </div>
  )
}

/** Pairing by typed code (Mission 04 §2). Where Pair was pressed, the person
    types the code; the other computer asks its person first, then shows it. */
export function PairingView() {
  const { world, dispatch } = usePortal()
  const p = world.pairing
  const peer = world.peers.find((x) => x.id === p?.peer) ?? unknownPeer(p?.peer ?? '')
  if (!p) return null
  const act = (action: 'showCode' | 'ignore' | 'cancel' | 'dismiss') => dispatch({ type: 'pairing', action })
  const cancel = <Key onClick={() => act('cancel')}>Cancel</Key>

  if (p.failed) {
    // Where Pair was pressed, the way forward is one press away
    const again = p.mine && world.peers.some((x) => x.id === p.peer)
    return (
      <Pane align="center">
        <Head title="Not paired" sub={p.failed} />
        <Actions>
          {again && (
            <Key
              variant="ink"
              onClick={() => {
                act('dismiss')
                dispatch({ type: 'pair', id: p.peer })
              }}
            >
              Pair again
            </Key>
          )}
          <Key variant={again ? 'plain' : 'ink'} onClick={() => act('dismiss')}>
            OK
          </Key>
        </Actions>
      </Pane>
    )
  }
  if (p.step === 'paired') {
    return (
      <Pane align="center">
        <StateTile tone="send">
          <CheckIcon weight="bold" />
        </StateTile>
        <Head title={`Paired with ${peer.name}`} sub={`Files and the clipboard can go between ${world.me.self} and ${peer.name} now.`} />
        <Actions>
          <Key
            variant="ink"
            onClick={() => {
              act('dismiss')
              // Back to the list with it marked, so the next pairing is quick
              if (world.peers.some((x) => !x.paired && x.id !== peer.id)) dispatch({ type: 'openPanel', panel: 'add' })
            }}
          >
            OK
          </Key>
        </Actions>
      </Pane>
    )
  }

  // The computer that was asked
  if (!p.mine) {
    if (p.step === 'asked') {
      return (
        <Pane align="center">
          <StateTile>
            <DeviceIcon device={peer} />
          </StateTile>
          <Head
            title={`${peer.name} wants to pair`}
            sub={`From ${p.peerAddr}. Show a code only if it’s you pairing: once paired, it can offer you files and send its clipboard.`}
          />
          <Countdown at={p.expiresAt}>{(left) => `The request goes away in ${left} if nobody answers.`}</Countdown>
          <Actions>
            <Key variant="ink" onClick={() => act('showCode')}>
              Show code
            </Key>
            <Key onClick={() => act('ignore')}>Ignore</Key>
          </Actions>
        </Pane>
      )
    }
    return (
      <Pane align="center">
        <Head
          title={`Type this code on ${peer.name}`}
          sub={
            p.step === 'wrong'
              ? `Wrong code typed on ${peer.name}: ${p.triesLeft === 1 ? '1 try left' : `${p.triesLeft} tries left`}. Not you? Press Cancel.`
              : `It works once, and only on ${peer.name}.`
          }
        />
        <PairCode code={p.code} />
        {p.step === 'checking' ? <Waiting>Checking…</Waiting> : <Countdown at={p.expiresAt}>{(left) => `The code works for ${left} more.`}</Countdown>}
        {/* Someone else may be guessing: stopping is the main thing to do */}
        <Actions>
          <Key variant={p.step === 'wrong' ? 'ink' : 'plain'} onClick={() => act('cancel')}>
            Cancel
          </Key>
        </Actions>
      </Pane>
    )
  }

  // Where Pair was pressed
  if (p.step === 'asking') {
    return (
      <Pane align="center">
        <StateTile>
          <DeviceIcon device={peer} />
        </StateTile>
        <Head
          title={`Asking ${peer.name} to pair`}
          sub={`On ${peer.name}, press Show code, then type the code here.`}
        />
        <Waiting>Waiting for {peer.name}…</Waiting>
        <Countdown at={p.expiresAt}>{(left) => `It waits ${left} more for an answer.`}</Countdown>
        <Actions>{cancel}</Actions>
      </Pane>
    )
  }
  // Keyed by the tries left: a wrong code starts again from an empty box
  return (
    <TypeCode
      key={p.triesLeft}
      name={peer.name}
      wrong={p.step === 'wrong'}
      checking={p.step === 'checking'}
      triesLeft={p.triesLeft}
      expiresAt={p.expiresAt}
      cancel={cancel}
    />
  )
}

/** The asker: six digits, typed as shown */
function TypeCode({
  name,
  wrong,
  checking,
  triesLeft,
  expiresAt,
  cancel,
}: {
  name: string
  wrong: boolean
  checking: boolean
  triesLeft: number
  expiresAt: number
  cancel: ReactNode
}) {
  const { dispatch } = usePortal()
  const [typed, setTyped] = useState('')
  const digits = typed.replace(/\D/g, '')
  const ready = digits.length === 6 && !checking
  const submit = () => ready && dispatch({ type: 'pairCode', code: digits })
  return (
    <Pane align="center">
      <Head title={`Type the code shown on ${name}`} sub={`On ${name}, press Show code, then type its six digits here.`} />
      <input
        className={s.codeInput}
        aria-label={`Code shown on ${name}`}
        inputMode="numeric"
        autoComplete="one-time-code"
        autoFocus
        maxLength={7}
        placeholder="___ ___"
        value={typed}
        disabled={checking}
        aria-invalid={wrong || undefined}
        onChange={(e) => setTyped(e.target.value.replace(/[^\d ]/g, ''))}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            submit()
          }
        }}
      />
      {wrong && !checking && (
        <p className={s.codeError} role="alert">
          That’s not it. Check the 6 digits on {name}: it’s the same code. {triesLeft === 1 ? '1 try left' : `${triesLeft} tries left`}, then you’ll
          need a new one.
        </p>
      )}
      {checking ? <Waiting>Checking…</Waiting> : <Countdown at={expiresAt}>{(left) => `The code works for ${left} more.`}</Countdown>}
      <Actions>
        <Key variant="ink" disabled={!ready} onClick={submit}>
          Pair
        </Key>
        {cancel}
      </Actions>
    </Pane>
  )
}

/** Paired, not connected yet: the core is still trying. Nothing to press. */
export function ConnectingView() {
  const { peer } = useFocus()
  return (
    <Pane align="center">
      <StateTile>
        <DeviceIcon device={peer} />
      </StateTile>
      <Head title={`Connecting to ${peer.name}…`} sub={`It connects by itself when ${APP_NAME} is open there.`} />
      <Waiting>{peer.reach?.doing ?? 'Looking on this network'}</Waiting>
    </Pane>
  )
}

/** Paired, and the core says it isn't reachable (after its grace period) */
export function OfflineView() {
  const { peer } = useFocus()
  const { world, dispatch } = usePortal()
  const failure = peer.reach?.lastFailure
  // Try again says it tried (review 05: nothing seemed to happen)
  const [tried, setTried] = useState(false)
  return (
    <Pane>
      {/* The likeliest cause first, in plain words; the numbers after */}
      <Head
        title={`${peer.name} isn’t reachable`}
        sub={`Is ${peer.name} on and awake, with ${APP_NAME} open? It connects by itself when it’s back.`}
      />
      {tried ? <p className={s.aside}>Tried again just now: no answer yet.</p> : failure && <p className={s.aside}>{failure}.</p>}
      <DoctorList checks={world.doctor} />
      {rescueActions(dispatch, peer, () => setTried(true))}
    </Pane>
  )
}

/** Disconnect was pressed: not connecting either way, still paired */
export function DisconnectedView() {
  const { peer } = useFocus()
  const { dispatch } = usePortal()
  return (
    <Pane align="center">
      <StateTile>
        <LinkBreakIcon weight="light" />
      </StateTile>
      <Head
        title={`Disconnected from ${peer.name}`}
        sub="Still paired. Press Connect to use it again."
      />
      <Actions>
        <Key variant="ink" onClick={() => dispatch({ type: 'setConnected', id: peer.id, value: true })}>
          Connect
        </Key>
      </Actions>
    </Pane>
  )
}

/** The other side runs another protocol version: say which computer to update */
export function IncompatibleView() {
  const { peer, me } = useFocus()
  const { world, dispatch } = usePortal()
  const here = peer.outdated === 'self'
  return (
    <Pane align="center">
      <StateTile>
        <ArrowsClockwiseIcon weight="light" />
      </StateTile>
      <Head
        title={`Update ${APP_NAME} on ${here ? me.self : peer.name}`}
        sub={
          here
            ? `${peer.name} runs a newer version that can’t talk to this one. Update ${me.self}; you stay paired.`
            : `It runs an older version that can’t talk to this one. Update ${APP_NAME} there; you stay paired.`
        }
      />
      <Actions>
        {here && world.update && (world.update.state === 'ready' || world.update.state === 'available') ? (
          <Key variant="ink" onClick={() => dispatch({ type: 'restartToUpdate' })}>
            {restartLabel(world.update)}
          </Key>
        ) : here && world.update ? (
          <Key
            variant="ink"
            disabled={['checking', 'downloading', 'installing'].includes(world.update.state)}
            onClick={() => dispatch({ type: 'checkUpdates' })}
          >
            {world.update.state === 'checking' ? 'Checking…' : 'Check for updates'}
          </Key>
        ) : null}
        <Key variant={here && world.update ? 'plain' : 'ink'} onClick={() => dispatch({ type: 'retrySearch' })}>
          Try again
        </Key>
      </Actions>
    </Pane>
  )
}

/** Connect by IP: for networks that hide devices from each other */
export function ConnectView() {
  const { world, dispatch } = usePortal()
  const [address, setAddress] = useState('')
  const [own, setOwn] = useState(false)
  const close = () => dispatch({ type: 'openPanel', panel: null })
  const submit = () => {
    const typed = address.trim().replace(/:\d+$/, '')
    if (!typed) return
    // The easy mistake: typing the address shown just below, which is this one's
    if (world.settings.addresses.some((a) => a.address === typed)) return setOwn(true)
    setOwn(false)
    dispatch({ type: 'connect', address })
  }
  return (
    <Pane onClose={close}>
      <Head
        title="Connect by IP"
        sub={`It’s in ${APP_NAME}’s Settings on the other computer.`}
      />
      <TextField
        label="The other computer’s IP address"
        placeholder="for example 192.168.1.20"
        mono
        autoFocus
        value={address}
        onChange={(v) => {
          setAddress(v)
          setOwn(false)
        }}
        onSubmit={submit}
        error={own ? 'That’s this computer’s own address. Type the one shown on the other computer.' : world.connect.error}
        inputMode="url"
      />
      {world.settings.addresses.length > 0 && (
        <div className={s.section}>
          <h3 className={s.sectionTitle}>This computer’s addresses</h3>
          <dl className={s.addresses}>
            {world.settings.addresses.map((a) => (
              <div key={a.address} style={{ display: 'contents' }}>
                <dt>{a.label}</dt>
                <dd>{a.address}</dd>
              </div>
            ))}
          </dl>
        </div>
      )}
      <Actions>
        <Key variant="ink" onClick={submit} disabled={world.connect.busy || !address.trim()}>
          {world.connect.busy ? 'Connecting…' : 'Connect'}
        </Key>
        <Key onClick={close}>Cancel</Key>
      </Actions>
    </Pane>
  )
}

/** The doctor on demand */
export function DoctorView() {
  const { world, dispatch } = usePortal()
  return (
    <Pane onClose={() => dispatch({ type: 'openPanel', panel: null })}>
      <Head title="Connection check" sub={`What ${APP_NAME} can see from here.`} />
      <DoctorList checks={world.doctor} />
      {rescueActions(dispatch)}
    </Pane>
  )
}
