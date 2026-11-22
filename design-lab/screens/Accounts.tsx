import { APP_NAME } from '../../src/platform'
import { useState } from 'react'
import { EnvelopeSimpleIcon, GiftIcon, KeyIcon } from '@phosphor-icons/react'
import { Key } from '../../src/components/Key/Key'
import { TextField } from '../../src/components/TextField/TextField'
import { useDevices } from '../twoDevices'
import { Actions, Head, Pane, Section, StateTile } from '../../src/window/stage/parts'
import { usage } from '../scenarios'
import { Meter } from './parts'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

/* Demo only: no account system exists. Nothing on these screens is sent anywhere. */
const person = { name: 'Sam Rivera', email: 'sam@example.com', initials: 'SR' }

const plans = [
  { id: 'free', name: 'Free', price: '$0', text: 'Scans and reports, as many as you like. Nothing is changed or run.' },
  {
    id: 'pro',
    name: 'Pro',
    price: '$12 a month',
    text: `Conversions, with up to $${usage.cap} of AI use a month included. Past that, you choose whether to allow more.`,
  },
  {
    id: 'own',
    name: 'Own key',
    price: '$5 a month',
    text: 'Conversions using your own AI key. You pay your AI provider directly for what it uses.',
  },
]

function SignIn({ ctx }: { ctx: ScreenCtx }) {
  const [email, setEmail] = useState(ctx.state === 'signin' ? '' : person.email)
  return (
    <Pane onClose={() => ctx.go('entry', 'drop')}>
      <Head
        title="Sign in to convert projects"
        sub="Scans work without an account. Conversions need one, so AI use can be counted against your plan."
      />
      <div className={s.fields}>
        <TextField
          label="Email"
          type="email"
          value={email}
          onChange={setEmail}
          placeholder="you@example.com"
          hint="We’ll email you a link to sign in. No password."
        />
      </div>
      <Actions>
        <Key variant="ink" icon={<EnvelopeSimpleIcon />} disabled={!email.includes('@')} onClick={() => ctx.go('accounts', 'account')}>
          Email me a link
        </Key>
        <Key onClick={() => ctx.go('entry', 'drop')}>Not now</Key>
      </Actions>
    </Pane>
  )
}

function Account({ ctx }: { ctx: ScreenCtx }) {
  const devices = useDevices()
  return (
    <Pane onClose={() => ctx.go('entry', 'drop')}>
      <div className={s.who}>
        <span className={s.avatar} aria-hidden>
          {person.initials}
        </span>
        <span className={s.whoName}>{person.name}</span>
        <span className={s.whoMeta}>
          {person.email} · Pro, renews {usage.renews}
        </span>
      </div>
      <Section title="AI use">
        <Meter spent={usage.spent} cap={usage.cap} label="This month" />
        <p className={s.noteText}>Counted per conversion. Scans never use AI.</p>
      </Section>
      <Section title="Computers">
        <p className={s.noteText}>
          {devices.mac.name} and {devices.win.name}. Pro works on both.
        </p>
      </Section>
      <Actions>
        <Key onClick={() => ctx.go('accounts', 'plans')}>Change plan</Key>
        <Key onClick={() => ctx.go('accounts', 'ownkey')}>Use my own key</Key>
        <Key>Sign out</Key>
      </Actions>
    </Pane>
  )
}

function Plans({ ctx }: { ctx: ScreenCtx }) {
  return (
    <Pane onClose={() => ctx.go('accounts', 'account')}>
      <Head title="Plans" sub="Change or cancel at any time. Scans stay free on every plan." />
      <ul className={s.plans}>
        {plans.map((p) => (
          <li key={p.id} className={s.plan}>
            <span className={s.planName}>
              {p.name}
              <span className={s.planPrice}>{p.price}</span>
            </span>
            <span className={s.planText}>{p.text}</span>
            <span className={s.planAction}>
              {p.id === 'pro' ? (
                <span className={s.planCurrent}>Your plan</span>
              ) : (
                <Key size="sm" onClick={() => p.id === 'own' && ctx.go('accounts', 'ownkey')}>
                  {p.id === 'free' ? 'Switch to Free' : 'Choose'}
                </Key>
              )}
            </span>
          </li>
        ))}
      </ul>
    </Pane>
  )
}

function Gift({ ctx }: { ctx: ScreenCtx }) {
  return (
    <Pane align="center">
      <StateTile>
        <GiftIcon weight="light" />
      </StateTile>
      <Head
        title={`You’ve been given $${usage.gift} of AI usage`}
        sub="It’s added to this month’s allowance and used first. It doesn’t run out at the end of the month."
      />
      <div className={s.meterBox}>
        <Meter spent={usage.spent} cap={usage.cap} gift={usage.gift} />
      </div>
      <Actions>
        <Key variant="ink" onClick={() => ctx.go('accounts', 'account')}>
          Got it
        </Key>
      </Actions>
    </Pane>
  )
}

function OwnKey({ ctx }: { ctx: ScreenCtx }) {
  const [key, setKey] = useState('')
  const devices = useDevices()
  const store = ctx.side === 'mac' ? 'the Mac’s keychain' : 'Windows Credential Manager'
  return (
    <Pane onClose={() => ctx.go('accounts', 'account')}>
      <StateTile>
        <KeyIcon weight="light" />
      </StateTile>
      <Head
        title="Use your own AI key"
        sub={`Conversions call the AI with your key instead of ${APP_NAME}’s. Your AI provider bills you for what it uses; the Own key plan is $5 a month.`}
      />
      <TextField
        label="API key"
        value={key}
        onChange={setKey}
        mono
        placeholder="sk-…"
        hint={`Kept in ${store} on ${devices[ctx.side].name}. It’s only ever sent to your AI provider, never to ${APP_NAME}.`}
      />
      <Actions>
        <Key variant="ink" disabled={key.trim().length < 8} onClick={() => ctx.go('accounts', 'account')}>
          Save key
        </Key>
        <Key onClick={() => ctx.go('accounts', 'account')}>Cancel</Key>
      </Actions>
    </Pane>
  )
}

/** What happens at the cap: nothing is charged without asking */
export function CapReached({ ctx, during }: { ctx: ScreenCtx; during?: boolean }) {
  return (
    <Pane>
      <Head
        title={during ? 'AI use for this month is used up' : 'You’ve used this month’s AI allowance'}
        sub={
          during
            ? `The AI stopped before attempt 3. ${APP_NAME} kept everything that passed its checks; 1 check still fails. Nothing more is spent unless you allow it.`
            : `Conversions stop before their next AI step until ${usage.renews}. Scans and fixes from known patterns keep working.`
        }
      />
      <Meter spent={usage.cap} cap={usage.cap} />
      <p className={s.noteText}>Allowing more adds it to this month only, and you’re charged only for what’s used.</p>
      <Actions>
        <Key variant="ink">Allow $10 more</Key>
        <Key onClick={() => ctx.go('accounts', 'ownkey')}>Use my own key</Key>
        {during && <Key onClick={() => ctx.go('final', 'summary')}>Finish without it</Key>}
      </Actions>
    </Pane>
  )
}

export function AccountsView({ ctx }: { ctx: ScreenCtx }) {
  switch (ctx.state) {
    case 'signin':
      return <SignIn ctx={ctx} />
    case 'plans':
      return <Plans ctx={ctx} />
    case 'gift':
      return <Gift ctx={ctx} />
    case 'ownkey':
      return <OwnKey ctx={ctx} />
    case 'cap':
      return <CapReached ctx={ctx} />
    default:
      return <Account ctx={ctx} />
  }
}
