// Screenshot every key state in both windows and both themes.
// Usage: node scripts/shots.mjs [url] [--mobile] [--only=light-offer,...]
// Uses the installed Google Chrome (no Playwright browser download needed).
import { mkdirSync } from 'node:fs'
import { chromium } from 'playwright'

const args = process.argv.slice(2)
const url = args.find((a) => a.startsWith('http')) ?? 'http://localhost:5180/'
const mobile = args.includes('--mobile')
const only = args.find((a) => a.startsWith('--only='))?.slice(7).split(',')
const outDir = new URL('../shots/', import.meta.url).pathname
mkdirSync(outDir, { recursive: true })

const states = [
  ['idle', 'mac'],
  ['offer', 'mac'],
  ['transfer', 'mac'],
  ['transfer-cable', 'mac'],
  ['converting', 'mac'],
  ['done-files', 'mac'],
  ['done-project', 'mac'],
  ['done-native', 'mac'],
  ['blocked-receiver', 'mac'],
  ['declined', 'mac'],
  ['converting', 'win'],
  ['done-project', 'win'],
  ['done-native', 'win'],
]

const browser = await chromium.launch({ channel: 'chrome' })
for (const theme of ['light', 'dark']) {
  const page = await browser.newPage({
    viewport: mobile ? { width: 390, height: 1700 } : { width: 1440, height: 1000 },
    deviceScaleFactor: mobile ? 2 : 1,
  })
  await page.goto(url)
  await page.waitForFunction(() => !!window.portal)
  await page.evaluate((t) => {
    window.portal.theme(t)
    window.portal.pause(true)
  }, theme)
  for (const [state, from] of states) {
    const name = `${mobile ? 'mobile-' : ''}${theme}-${state}${from === 'win' ? '-from-win' : ''}`
    if (only && !only.includes(name)) continue
    await page.evaluate(([to, f]) => window.portal.jump(to, f), [state, from])
    await page.waitForTimeout(700)
    await page.screenshot({ path: `${outDir}${name}.png`, fullPage: mobile })
    console.log('saved', name)
  }
  await page.close()
}
await browser.close()
