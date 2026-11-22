// Mission 04 §1.3: the edge glow runs on the compositor.
// Records a Chromium performance trace of the glow alone (/?glow=…, dev
// server) and asserts:
//   - sending, receiving and working: no paints of the glow after the first
//     second, and frames keep coming while the main thread is blocked for
//     500 ms (Mission 05 §1.1: receiving is a travelling light too, not a
//     progress fill that crawled at a degree a second on a big transfer).
// Usage: npm run dev, then node scripts/glow-trace.mjs
//   GLOW_CHANNEL=msedge  use the installed Edge (the WebView2 engine) instead
//   PORTAL_URL=…         the dev server (default http://localhost:5180/)
import { chromium } from 'playwright'

const base = process.env.PORTAL_URL ?? 'http://localhost:5180/'
const channel = process.env.GLOW_CHANNEL || undefined
const categories = [
  'devtools.timeline',
  'disabled-by-default-devtools.timeline',
  'disabled-by-default-devtools.timeline.frame',
  'blink.user_timing',
  'cc',
]

const browser = await chromium.launch({ channel })
console.log(`browser: ${channel ?? 'chromium'} ${browser.version()}`)
const page = await browser.newPage({ viewport: { width: 700, height: 900 } })

async function trace(mode) {
  await page.goto(`${base}?glow=${mode}`)
  await page.waitForSelector('[data-glow-bench]')
  // The first second: layers are created and painted once.
  await page.waitForTimeout(1200)
  await browser.startTracing(page, { categories })
  await page.evaluate(() => performance.mark('glow-steady'))
  await page.waitForTimeout(2000)
  await page.evaluate(() => {
    performance.mark('block-start')
    const t = performance.now()
    while (performance.now() - t < 500) {
      /* the main thread is busy */
    }
    performance.mark('block-end')
  })
  await page.waitForTimeout(400)
  const events = JSON.parse((await browser.stopTracing()).toString()).traceEvents
  const mark = (name) => events.find((e) => e.name === name && e.cat.includes('blink.user_timing'))?.ts
  const [steady, blockStart, blockEnd] = [mark('glow-steady'), mark('block-start'), mark('block-end')]
  if (!steady || !blockStart || !blockEnd) throw new Error(`marks missing in the ${mode} trace`)
  const paints = events.filter((e) => e.name === 'Paint' && e.ts >= steady && e.ts < blockStart)
  // One repaint can paint a few layers (the ring and its mask): count frames that paint
  const rounds = new Set(paints.map((e) => Math.floor(e.ts / 8000))).size
  const seconds = (blockStart - steady) / 1e6
  // Compositor frames drawn while the main thread was blocked
  const frames = events.filter((e) => (e.name === 'DrawFrame' || e.name === 'Graphics.Pipeline.DrawAndSwap') && e.ts > blockStart && e.ts < blockEnd)
  const drawn = new Set(frames.map((e) => Math.round(e.ts / 1000))).size
  return { mode, paintsPerSecond: rounds / seconds, framesWhileBlocked: drawn, frameEvents: frames.length }
}

const failures = []
for (const mode of ['sending', 'working', 'receiving']) {
  const r = await trace(mode)
  console.log(`${mode}: ${r.paintsPerSecond.toFixed(1)} repaints/s while running; ${r.framesWhileBlocked} frames drawn during a 500 ms main-thread block`)
  if (r.paintsPerSecond > 0.5) failures.push(`${mode} repaints ${r.paintsPerSecond.toFixed(1)} times a second (expected none after the first second)`)
  if (r.framesWhileBlocked < 10) failures.push(`${mode} drew ${r.framesWhileBlocked} frames while the main thread was blocked for 500 ms (the light stopped)`)
}
await browser.close()
if (failures.length) {
  console.error(`\nFAIL\n- ${failures.join('\n- ')}`)
  process.exit(1)
}
console.log('\nOK: the glow runs on the compositor')
