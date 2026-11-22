// Screenshots of the design lab: every screen state, in both window styles
// and both themes, with the lab panel hidden. Run the lab first (npm run lab).
// Usage: node design-lab/scripts/lab-shots.mjs [--out=dir] [--only=report,consent]
//        [--scenario=local-chat] [--theme=light,dark]
//        --docs   the key screens for docs/screens (1x, one theme each)
// Uses the installed Google Chrome (no Playwright browser download needed).
import { mkdirSync } from 'node:fs'
import { resolve } from 'node:path'
import { chromium } from 'playwright'

const arg = (name) => process.argv.find((a) => a.startsWith(`--${name}=`))?.split('=')[1]
const list = (name) => arg(name)?.split(',')
const out = resolve(arg('out') ?? 'shots/lab')
const base = 'http://localhost:5190/'
mkdirSync(out, { recursive: true })

/** Every screen and state. With os "auto", the window is the one that shows
    it in that scenario: local-chat and shop-web start on the Mac, ledger-tool
    and InvoiceDesk on the PC, so each screen is shot in both window styles. */
const pairs = {
  sender: [
    ['local-chat', 'auto'],
    ['ledger-tool', 'auto'],
  ],
  receiver: [
    ['local-chat', 'auto'],
    ['ledger-tool', 'auto'],
  ],
  rebuild: [
    ['sketchbook', 'auto'],
    ['invoicedesk', 'auto'],
  ],
  both: [
    ['local-chat', 'mac'],
    ['local-chat', 'win'],
  ],
}
const shots = [
  ['entry', 'drop', 'sender'],
  ['entry', 'over', 'sender'],
  ['scanning', 'reading', 'sender'],
  ['scanning', 'code', 'sender'],
  ['scanning', 'packages', 'sender'],
  ['report', 'summary', 'sender'],
  ['report', 'expanded', 'sender'],
  ['report', 'exported', 'sender'],
  ['report', 'summary', [['shop-web', 'auto'], ['handbook', 'auto']]],
  ['report', 'summary', 'rebuild'],
  ['consent', 'command', 'receiver'],
  ['consent', 'ai', 'receiver'],
  ['converting', 'plan', 'receiver'],
  ['converting', 'apply', 'receiver'],
  ['converting', 'verify', 'receiver'],
  ['converting', 'repair', 'receiver'],
  ['final', 'summary', 'receiver'],
  ['final', 'summary', [['shop-web', 'auto']]],
  ['final', 'patches', 'receiver'],
  ['final', 'undone', 'receiver'],
  ['final', 'exported', 'receiver'],
  ['rebuild', 'verdict', 'rebuild'],
  ['rebuild', 'plan', 'rebuild'],
  ['accounts', 'signin', 'both'],
  ['accounts', 'account', 'both'],
  ['accounts', 'plans', 'both'],
  ['accounts', 'gift', 'both'],
  ['accounts', 'ownkey', 'both'],
  ['accounts', 'cap', 'both'],
  ['edge', 'empty', 'both'],
  ['edge', 'huge', 'sender'],
  ['edge', 'unreadable', 'sender'],
  ['edge', 'offline', 'sender'],
  ['edge', 'budget', 'receiver'],
  ['edge', 'declined', 'receiver'],
]

/** The key screens for docs/screens: each in its natural window, light and dark mixed */
const docs = [
  ['entry', 'drop', 'local-chat', 'light'],
  ['scanning', 'code', 'ledger-tool', 'dark'],
  ['report', 'summary', 'local-chat', 'light'],
  ['report', 'expanded', 'ledger-tool', 'dark'],
  ['report', 'summary', 'handbook', 'light'],
  ['consent', 'command', 'local-chat', 'light'],
  ['consent', 'ai', 'ledger-tool', 'dark'],
  ['converting', 'apply', 'local-chat', 'light'],
  ['converting', 'repair', 'ledger-tool', 'dark'],
  ['final', 'summary', 'local-chat', 'light'],
  ['final', 'patches', 'ledger-tool', 'dark'],
  ['rebuild', 'verdict', 'sketchbook', 'light'],
  ['rebuild', 'verdict', 'invoicedesk', 'dark'],
  ['accounts', 'plans', 'local-chat', 'light'],
  ['accounts', 'cap', 'ledger-tool', 'dark'],
  ['edge', 'empty', 'local-chat', 'dark'],
  ['edge', 'declined', 'local-chat', 'light'],
]
if (process.argv.includes('--docs')) {
  const browser = await chromium.launch({ channel: 'chrome' })
  const page = await browser.newPage({ viewport: { width: 760, height: 900 }, deviceScaleFactor: 1 })
  const dir = resolve(arg('out') ?? 'docs/screens')
  mkdirSync(dir, { recursive: true })
  for (const [screen, state, scenario, theme] of docs) {
    const q = new URLSearchParams({ screen, state, os: 'auto', theme, scenario, panel: 'hidden' })
    await page.goto(`${base}?${q}`)
    await page.waitForSelector('section[aria-label]')
    await page.waitForTimeout(1200)
    const side = await page.locator('main section').first().getAttribute('data-os')
    await page.locator('main section').first().screenshot({ path: `${dir}/lab-${screen}-${state}-${scenario}-${side}-${theme}.png` })
  }
  await browser.close()
  console.log(`${docs.length} screenshots in ${dir}`)
  process.exit(0)
}

const only = list('only')
const scenarioArg = arg('scenario')
const themes = list('theme') ?? ['light', 'dark']
const browser = await chromium.launch({ channel: 'chrome' })
const page = await browser.newPage({ viewport: { width: 760, height: 900 }, deviceScaleFactor: 2 })
const problems = []
page.on('console', (m) => (m.type() === 'error' || m.type() === 'warning') && problems.push(m.text()))
page.on('pageerror', (e) => problems.push(String(e)))

let count = 0
for (const [screen, state, which] of shots) {
  if (only && !only.includes(screen)) continue
  const combos = typeof which === 'string' ? pairs[which] : which
  for (const theme of themes) {
    for (const [scenario, os] of combos) {
      if (scenarioArg && scenario !== scenarioArg) continue
      const q = new URLSearchParams({ screen, state, os, theme, scenario, panel: 'hidden' })
      await page.goto(`${base}?${q}`)
      await page.waitForSelector('section[aria-label]')
      await page.waitForTimeout(900)
      const side = await page.locator('main section').first().getAttribute('data-os')
      const name = `${screen}-${state}-${scenario}-${side}-${theme}`
      await page.locator('main section').first().screenshot({ path: `${out}/${name}.png` })
      count++
    }
  }
}
await browser.close()
console.log(`${count} screenshots in ${out}`)
if (problems.length) {
  console.log('Console problems:')
  for (const p of [...new Set(problems)]) console.log(' ', p)
}
