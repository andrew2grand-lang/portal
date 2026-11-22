import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './styles/base.css'

// Dev only: /?mock=<scenario>&os=mac|win runs the real app's UI against a fake core
const params = new URLSearchParams(window.location.search)
const mock = import.meta.env.DEV ? params.get('mock') : null
if (mock) {
  const { installMockCore } = await import('./dev/mockCore')
  installMockCore(mock, params.get('os') === 'win' ? 'win' : 'mac')
}

// Dev only: /?glow=sending|receiving|working shows the edge glow alone (performance trace)
const glow = import.meta.env.DEV ? params.get('glow') : null
if (glow === 'sending' || glow === 'receiving' || glow === 'working') {
  const { GlowBench } = await import('./dev/GlowBench')
  createRoot(document.getElementById('root')!).render(<GlowBench mode={glow} />)
} else {
  const { default: App } = await import('./App.tsx')
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
}
