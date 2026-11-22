import { IconContext } from '@phosphor-icons/react'
import { CoreProvider } from './core/CoreProvider'
import { DevPanel } from './dev/DevPanel'
import { APP_NAME, hostSide, inTauri } from './platform'
import { AcceptWindow } from './window/AcceptWindow'
import { PortalWindow } from './window/PortalWindow'
import s from './App.module.css'

const iconDefaults = { weight: 'regular' as const, size: 16, 'aria-hidden': true }

/** In the app (or a browser with ?mock=…): this machine's window, driven by
    the core. In a plain browser: the list of mock scenarios to open. */
export default function App() {
  if (inTauri && new URLSearchParams(location.search).get('view') === 'accept') {
    return (
      <IconContext value={iconDefaults}>
        <CoreProvider>
          <AcceptWindow />
        </CoreProvider>
      </IconContext>
    )
  }

  if (inTauri) {
    return (
      <IconContext value={iconDefaults}>
        <CoreProvider>
          <main className={s.app}>
            <h1 className="sr-only">{APP_NAME}</h1>
            <PortalWindow side={hostSide} native />
          </main>
          <DevPanel />
        </CoreProvider>
      </IconContext>
    )
  }

  return <ScenarioIndex />
}

/** Dev server only: every state of the real window, against the mock core */
function ScenarioIndex() {
  const list = import.meta.env.DEV ? (import.meta.glob('./dev/scenarioList.ts', { eager: true }) as Record<string, { scenarioList: readonly string[] }>) : {}
  const scenarios = Object.values(list)[0]?.scenarioList ?? []
  return (
    <main className={s.index}>
      <h1>{APP_NAME}: window states</h1>
      <p>Each opens the real window against a mock core. Add &amp;devices=6 for six paired devices.</p>
      <ul>
        {scenarios.map((sc) => (
          <li key={sc}>
            <span>{sc}</span>
            <a href={`/?mock=${sc}&os=mac`}>Mac</a>
            <a href={`/?mock=${sc}&os=win`}>Windows</a>
          </li>
        ))}
      </ul>
    </main>
  )
}
