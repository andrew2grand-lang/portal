/* Dev only: the edge glow alone, in a window-sized box, for the performance
   trace (scripts/glow-trace.mjs, Mission 04 §1.3). /?glow=sending|receiving|working */
import { EdgeGlow } from '../components/EdgeGlow/EdgeGlow'
import type { GlowMode } from '../engine/views'
import shell from '../window/AppWindow.module.css'

export function GlowBench({ mode }: { mode: GlowMode }) {
  return (
    <main style={{ display: 'grid', placeItems: 'center', minHeight: '100vh' }}>
      <section className={shell.shell} data-os="mac" data-native="" style={{ width: 552, height: 780 }} data-glow-bench="">
        <EdgeGlow mode={mode} link="wifi" arrival={null} />
        <div className={shell.window} />
      </section>
    </main>
  )
}
