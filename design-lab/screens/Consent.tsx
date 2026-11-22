import { APP_NAME } from '../../src/platform'
import { useState } from 'react'
import { RobotIcon, TerminalWindowIcon } from '@phosphor-icons/react'
import { Key } from '../../src/components/Key/Key'
import { useDevices } from '../twoDevices'
import { formatBytes, plural } from '../../src/engine/format'
import { Actions, Head, Pane, StateTile } from '../../src/window/stage/parts'
import { osName } from '../report'
import { usage } from '../scenarios'
import { useSay } from './say'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

/** 4a · Before running anything from the project */
function RunCommand({ ctx }: { ctx: ScreenCtx }) {
  const devices = useDevices()
  const say = useSay()
  const c = ctx.scenario.convert!.consent
  const me = devices[ctx.side]
  return (
    <Pane>
      <StateTile>
        <TerminalWindowIcon weight="light" />
      </StateTile>
      <Head
        title={
          <>
            This project wants to run <code className={s.command}>{c.command}</code>. Run it?
          </>
        }
        sub={say(c.why)}
      />
      <p className={s.noteText}>
        It runs on {me.self} as you, in the project folder, for up to 2 minutes, then {APP_NAME} stops it. It comes from{' '}
        <span className={s.mono}>{c.from}</span>.
      </p>
      <p className={s.never}>If you skip it, starting it is marked Couldn’t check. Nothing else changes.</p>
      <Actions>
        <Key variant="ink" onClick={() => ctx.go('converting', 'verify')}>
          Run it
        </Key>
        <Key onClick={() => ctx.go('edge', 'declined')}>Skip this check</Key>
      </Actions>
    </Pane>
  )
}

/** 4b · Before sending any file to the AI: exactly what, and what never */
function SendToAi({ ctx }: { ctx: ScreenCtx }) {
  const c = ctx.scenario.convert!.consent
  const [showAll, setShowAll] = useState(ctx.state === 'ai-files')
  const total = c.aiFiles.reduce((n, f) => n + f.bytes, 0)
  const count = plural(c.aiFiles.length, 'file')
  const shown = showAll ? c.aiFiles : c.aiFiles.slice(0, 3)
  return (
    <Pane>
      <StateTile>
        <RobotIcon weight="light" />
      </StateTile>
      <Head title={`Send ${count} to the AI to fix this?`} sub="Here’s exactly what’s sent. No secrets." />
      <p className={s.noteText}>{c.aiProblem}</p>
      <ul className={s.files} aria-label="Files that would be sent">
        {shown.map((f) => (
          <li key={f.path}>
            <span>{f.path}</span>
            <span>{formatBytes(f.bytes)}</span>
          </li>
        ))}
      </ul>
      {c.aiFiles.length > 3 && (
        <Key size="sm" className={s.inlineKey} onClick={() => setShowAll((v) => !v)}>
          {showAll ? 'Show fewer' : `Show all ${c.aiFiles.length}`}
        </Key>
      )}
      <p className={s.never}>
        {count}, {formatBytes(total)} in all. Never sent: {c.neverSent.join(', ')}.
      </p>
      <p className={s.estimate}>
        About $0.05 of AI usage. You’ve used <strong>${usage.spent.toFixed(2)}</strong> of ${usage.cap.toFixed(2)} this
        month.
      </p>
      <Actions>
        <Key variant="ink" onClick={() => ctx.go('converting', 'repair')}>
          Send {count}
        </Key>
        <Key onClick={() => ctx.go('edge', 'declined')}>Don’t send</Key>
      </Actions>
    </Pane>
  )
}

export function ConsentView({ ctx }: { ctx: ScreenCtx }) {
  if (!ctx.scenario.convert) return <NothingToConvert ctx={ctx} />
  return ctx.state === 'command' ? <RunCommand ctx={ctx} /> : <SendToAi ctx={ctx} />
}

/** Scenarios that never reach a conversion say why, in the window's voice */
export function NothingToConvert({ ctx }: { ctx: ScreenCtx }) {
  const r = ctx.scenario.report
  const native = r.tier.tier === 'native'
  return (
    <Pane align="center">
      <Head
        title={native ? `${r.project.name} needs a rebuild, not a conversion` : `${r.project.name} has nothing to convert`}
        sub={
          native
            ? `It’s an app for one OS, so ${APP_NAME} doesn’t pretend to convert it. The rebuild plan says what moving it to ${osName[r.target]} takes.`
            : `It arrives clean on ${osName[r.target]}: send it as files.`
        }
      />
      <Actions>
        <Key variant="ink" onClick={() => ctx.go(native ? 'rebuild' : 'report', native ? 'verdict' : 'summary')}>
          {native ? 'See the rebuild plan' : 'Back to the report'}
        </Key>
      </Actions>
    </Pane>
  )
}
