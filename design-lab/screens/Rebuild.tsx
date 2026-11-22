import { APP_NAME } from '../../src/platform'
import { useState } from 'react'
import { CheckIcon, HammerIcon } from '@phosphor-icons/react'
import { Key } from '../../src/components/Key/Key'
import { Actions, Head, Pane, StateTile } from '../../src/window/stage/parts'
import { Blockers } from './parts'
import { download, handoffMarkdown } from '../handoff'
import { aiLevelName, osName } from '../report'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

/** 7 · Native apps: an honest "Rebuild", what blocks it, and the plan. No progress bar. */
export function RebuildView({ ctx }: { ctx: ScreenCtx }) {
  const plan = ctx.scenario.rebuild
  const r = ctx.scenario.report
  const [exported, setExported] = useState(false)
  const target = osName[r.target]
  if (!plan) {
    return (
      <Pane align="center">
        <Head
          title={`${r.project.name} doesn’t need a rebuild`}
          sub={`It isn’t an app for one OS. The report says what changes on ${target}.`}
        />
        <Actions>
          <Key variant="ink" onClick={() => ctx.go('report', 'summary')}>
            See the report
          </Key>
        </Actions>
      </Pane>
    )
  }
  const exportPlan = () => {
    download(`${r.project.name} migration plan.md`, handoffMarkdown(r))
    setExported(true)
  }

  if (ctx.state === 'plan') {
    return (
      <Pane>
        <Head title={`Moving ${r.project.name} to ${target}`} sub={`A plan for a person or a team. ${APP_NAME} doesn’t change any code for a rebuild.`} />
        <ol className={s.planSteps}>
          {plan.steps.map((step) => (
            <li key={step.title}>
              <strong>{step.title}</strong>
              <span>{step.detail}</span>
            </li>
          ))}
        </ol>
        <p className={s.estimate}>
          AI use (estimate): <strong>{aiLevelName[r.aiUsage.level]}</strong>. With Pro, the AI can write this plan out in
          detail for your code. It doesn’t touch the project.
        </p>
        <Actions>
          <Key variant="ink" icon={exported ? <CheckIcon /> : undefined} onClick={exportPlan}>
            {exported ? 'Migration plan saved' : 'Export migration plan'}
          </Key>
          <Key onClick={() => ctx.go('rebuild', 'verdict')}>Back</Key>
        </Actions>
      </Pane>
    )
  }

  return (
    <Pane>
      <StateTile tone="danger">
        <HammerIcon weight="light" />
      </StateTile>
      <Head title={`${r.project.name} needs a rebuild for ${target}`} sub={plan.summary} />
      <Blockers blockers={plan.blockers} />
      <p className={s.subhead}>What carries over</p>
      <ul className={s.keeps}>
        {plan.keeps.map((k) => (
          <li key={k}>
            <CheckIcon weight="bold" aria-hidden />
            <span>{k}</span>
          </li>
        ))}
      </ul>
      <Actions>
        <Key variant="ink" onClick={() => ctx.go('rebuild', 'plan')}>
          See the migration plan
        </Key>
        <Key icon={exported ? <CheckIcon /> : undefined} onClick={exportPlan}>
          {exported ? 'Hand-off note saved' : 'Export hand-off note'}
        </Key>
      </Actions>
    </Pane>
  )
}
