import { APP_NAME } from '../../src/platform'
import { useState } from 'react'
import {
  ArrowUpRightIcon,
  CheckIcon,
  CubeIcon,
  FilesIcon,
  FolderDashedIcon,
  FolderSimpleDashedIcon,
  LockKeyIcon,
  HammerIcon,
  KeyIcon,
  StackIcon,
  TerminalWindowIcon,
} from '@phosphor-icons/react'
import type { ReactNode } from 'react'
import { Chip } from '../../src/components/Chip/Chip'
import { Key } from '../../src/components/Key/Key'
import { useDevices } from '../twoDevices'
import { formatBytes, plural } from '../../src/engine/format'
import { other } from '../twoDevices'
import { Actions, Head, Pane, StateTile } from '../../src/window/stage/parts'
import { download, handoffMarkdown } from '../handoff'
import type { Report, Tier } from '../report'
import { aiLevelName, groupFindings, osName, stateOrder, tierName } from '../report'
import { Arrive, FindingSection } from './parts'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

const tierIcon: Record<Tier, ReactNode> = {
  static: <FilesIcon />,
  scripts: <TerminalWindowIcon />,
  apps: <CubeIcon />,
  systems: <StackIcon />,
  native: <HammerIcon />,
}

/** "a FastAPI server and a local model via Ollama" from "Apps: a FastAPI…" */
const reasonOf = (r: Report) => r.tier.reason.replace(/^[A-Za-z ]+: /, '')

/** What stays behind: environments rebuilt there, ignored files, OS clutter */
export function LeftOut({ r, peer }: { r: Report; peer: string }) {
  if (!r.leftOut.items.length) return null
  const rebuilt = r.leftOut.items.filter((i) => i.rebuildable)
  return (
    <div className={s.note}>
      <span className={s.noteTile}>
        <FolderSimpleDashedIcon weight="light" />
      </span>
      <p className={s.noteTitle}>{formatBytes(r.leftOut.bytes)} won’t be sent</p>
      <div className={s.noteText}>
        {rebuilt.length > 0
          ? `${rebuilt.map((i) => i.path).join(' and ')} ${rebuilt.length === 1 ? 'is' : 'are'} rebuilt on ${peer} from the project’s own files. The rest is ignored by the project or clutter.`
          : 'Ignored by the project, or clutter the other computer doesn’t need.'}
        <ul className={s.sizes}>
          {r.leftOut.items.slice(0, 5).map((i) => (
            <li key={i.path}>
              <span className={i.kind === 'osJunk' ? undefined : s.mono}>{i.path}</span>
              <span>{i.bytes ? formatBytes(i.bytes) : plural(i.files, 'file')}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}

export function Secrets({ r, peer, self }: { r: Report; peer: string; self: string }) {
  if (!r.secrets.length) return null
  const keys = r.secrets.flatMap((f) => f.keys)
  return (
    <div className={s.note}>
      <span className={s.noteTile}>
        <KeyIcon weight="light" />
      </span>
      <p className={s.noteTitle}>Secrets stay on {self}</p>
      <p className={s.noteText}>
        {r.secrets.map((f) => (
          <span key={f.path}>
            <span className={s.mono}>{f.path}</span>{' '}
          </span>
        ))}
        {keys.length > 0 ? (
          <>
            holds {plural(keys.length, 'key')}:{' '}
            {keys.slice(0, 6).map((k, i) => (
              <span key={k}>
                {i > 0 && ', '}
                <span className={s.mono}>{k}</span>
              </span>
            ))}
            . {APP_NAME} only read their names, never the values. Add them on {peer} yourself.
          </>
        ) : (
          <>looks like a key or credentials file. It isn’t sent; copy it to {peer} yourself if it’s needed.</>
        )}
      </p>
    </div>
  )
}

export function Unreadable({ r }: { r: Report }) {
  const files = r.stats.unreadable
  if (!files.length) return null
  return (
    <div className={s.note}>
      <span className={s.noteTile}>
        <LockKeyIcon weight="light" />
      </span>
      <p className={s.noteTitle}>{plural(files.length, 'file')} couldn’t be read</p>
      <div className={s.noteText}>
        This computer won’t let {APP_NAME} open {files.length === 1 ? 'it' : 'them'}, so {files.length === 1 ? 'it wasn’t' : 'they weren’t'} checked
        and won’t be sent. Check their permissions, then scan again.
        <ul className={s.sizes}>
          {files.slice(0, 5).map((f) => (
            <li key={f}>
              <span className={s.mono}>{f}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  )
}

/** A folder with nothing in it: say so, don't pretend it passed */
function EmptyFolder({ ctx, r }: { ctx: ScreenCtx; r: Report }) {
  return (
    <Pane align="center">
      <StateTile>
        <FolderDashedIcon weight="light" />
      </StateTile>
      <Head
        title="This folder is empty"
        sub={`There’s nothing in ${r.project.name} to check or send. Choose the folder that holds the project’s files.`}
      />
      <Actions>
        <Key variant="ink" onClick={() => ctx.go('entry', 'drop')}>
          Choose another folder
        </Key>
      </Actions>
    </Pane>
  )
}

/** 3 · The free report: the answer, the tier, every finding by state */
export function ReportView({ ctx, report }: { ctx: ScreenCtx; report: Report }) {
  const devices = useDevices()
  const me = devices[ctx.side]
  const peer = devices[other(ctx.side)]
  const r = report
  const target = osName[r.target]
  const firstOpen = groupFindings(r.findings, stateOrder.find((st) => groupFindings(r.findings, st).length) ?? 'info')[0]
  const offlineOpen = groupFindings(r.findings, 'couldntCheck').find((g) => g.first.rule === 'wheels-unchecked')
  const [open, setOpen] = useState<string | null>(
    ctx.state === 'expanded' ? (firstOpen?.key ?? null) : ctx.state === 'offline' ? (offlineOpen?.key ?? null) : null,
  )
  const [exported, setExported] = useState(ctx.state === 'exported')
  const native = r.tier.tier === 'native'
  const nothing = r.counts.blocker + r.counts.willNeedAFix + r.counts.needsYou === 0
  const fixable = r.counts.willNeedAFix

  const exportNote = () => {
    download(`${r.project.name} hand-off.md`, handoffMarkdown(r))
    setExported(true)
  }

  if (r.stats.files === 0) return <EmptyFolder ctx={ctx} r={r} />
  let i = 0
  return (
    <Pane>
      <Head title={r.headline} sub={`${r.project.name}, read on ${me.self}. Nothing was run or changed.`} />
      <Arrive as="div" i={i++}>
        <div className={s.verdict}>
          <Chip icon={tierIcon[r.tier.tier]}>{tierName[r.tier.tier]}</Chip>
          <span className={s.verdictText}>{reasonOf(r)}</span>
        </div>
      </Arrive>
      <Unreadable r={r} />
      {stateOrder.map((state) => (
        <FindingSection
          key={state}
          state={state}
          groups={groupFindings(r.findings, state)}
          target={target}
          openKey={open}
          setOpen={setOpen}
          i={i++}
          actionFor={(f) =>
            f.rule === 'wheels-unchecked' && !r.network.online ? (
              <Key size="sm" onClick={ctx.checkOnline}>
                Check packages online
              </Key>
            ) : null
          }
        />
      ))}
      {(r.leftOut.items.length > 0 || r.secrets.length > 0) && <div className={s.rule} />}
      <LeftOut r={r} peer={peer.name} />
      <Secrets r={r} peer={peer.name} self={me.self} />
      <p className={s.estimate}>
        AI use for a conversion (estimate): <strong>{aiLevelName[r.aiUsage.level]}</strong>. {r.aiUsage.reason}
      </p>
      {!native && !nothing && (
        <div className={s.pro}>
          <p className={s.proTitle}>Pro converts it for you</p>
          <p className={s.proText}>
            {fixable === 1 ? 'The fix above happens' : fixable > 1 ? `The ${fixable} fixes above happen` : 'The changes happen'} on{' '}
            {peer.name}, then {APP_NAME} checks that it installs, passes its tests and starts. You see every change and can undo
            any of them.
          </p>
        </div>
      )}
      <Actions>
        {native ? (
          <Key variant="ink" onClick={() => ctx.go('rebuild', 'verdict')}>
            See the rebuild plan
          </Key>
        ) : nothing ? (
          <Key variant="ink" icon={<ArrowUpRightIcon />}>
            Send as files
          </Key>
        ) : (
          <Key variant="ink" onClick={() => ctx.go('consent', 'command')}>
            Convert with Pro
          </Key>
        )}
        <Key icon={exported ? <CheckIcon /> : undefined} onClick={exportNote}>
          {exported ? 'Hand-off note saved' : 'Export hand-off note'}
        </Key>
      </Actions>
    </Pane>
  )
}

function reportFor(ctx: ScreenCtx): Report | null {
  if (ctx.state === 'real') return ctx.real.report
  return ctx.scenario.report
}

export function ReportScreen({ ctx }: { ctx: ScreenCtx }) {
  const r = reportFor(ctx)
  if (!r) {
    return (
      <Pane align="center">
        <Head title="No real scan yet" sub="Type a folder’s path under “Scan a real folder” in the lab panel, then press Scan." />
      </Pane>
    )
  }
  return <ReportView key={`${r.project.path}-${ctx.state}`} ctx={ctx} report={r} />
}

