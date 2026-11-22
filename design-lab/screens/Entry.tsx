import { APP_NAME } from '../../src/platform'
import { CodeBlockIcon } from '@phosphor-icons/react'
import { useDevices } from '../twoDevices'
import { other } from '../twoDevices'
import st from '../../src/window/stage/Stage.module.css'
import s from './screens.module.css'
import type { ScreenCtx } from './types'

/** 1 · The Project tab: drop a project; what the free scan and Pro do */
export function EntryView({ ctx }: { ctx: ScreenCtx }) {
  const devices = useDevices()
  const peer = devices[other(ctx.side)]
  const os = ctx.side === 'mac' ? 'Windows' : 'macOS'
  const over = ctx.state === 'over'
  return (
    <>
      <button
        type="button"
        className={st.drop}
        data-over={over ? '' : undefined}
        onClick={ctx.startDemoScan}
        aria-describedby="project-tiers"
      >
        <span className={st.dropMark} aria-hidden>
          <span className={st.rings}>
            <span />
            <span />
            <span />
          </span>
          <span className={st.dropTile}>
            <CodeBlockIcon weight="light" />
          </span>
        </span>
        <span className={st.dropTitle}>{over ? 'Drop to check it' : 'Drop a project folder'}</span>
        <span className={st.dropSub}>
          {APP_NAME} reads it and tells you what would break on {os}. Nothing is run or changed.
        </span>
      </button>
      <div className={s.tiers} id="project-tiers">
        <div>
          <strong>Free scan</strong>
          <span>A report of what blocks it on {os} and what needs a fix.</span>
        </div>
        <div>
          <strong>Pro conversion</strong>
          <span>{APP_NAME} fixes it on {peer.name}, checks it runs, and shows every change.</span>
        </div>
      </div>
    </>
  )
}
