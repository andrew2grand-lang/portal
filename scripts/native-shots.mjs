// Screenshots of the app's single-window ("native") layout, rendered in Chrome
// with a stubbed Tauri bridge. Usage: npm run dev, then node scripts/native-shots.mjs [label]
import { chromium } from 'playwright'
import { mkdirSync } from 'node:fs'

const label = process.argv[2] ?? 'native'
const url = process.env.PORTAL_URL ?? 'http://localhost:5180/'
const out = 'shots/native'
mkdirSync(out, { recursive: true })

const agents = {
  mac: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)',
  win: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0',
}

const browser = await chromium.launch({ channel: 'chrome' })
for (const os of ['mac', 'win']) {
  for (const theme of ['light', 'dark']) {
    const ctx = await browser.newContext({ viewport: { width: 552, height: 780 }, deviceScaleFactor: 2, userAgent: agents[os] })
    await ctx.addInitScript(({ theme }) => {
      localStorage.setItem('portal.dev', JSON.stringify({ theme }))
      if (!window.__TAURI_INTERNALS__) {
        window.__TAURI_INTERNALS__ = {
          invoke: async () => null,
          transformCallback: () => Math.floor(Math.random() * 1e9),
          metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
        }
      }
    }, { theme })
    const page = await ctx.newPage()
    const errors = []
    page.on('pageerror', (e) => errors.push(e.message))
    page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
    await page.goto(url)
    await page.waitForTimeout(900)
    const file = `${out}/${label}-${os}-${theme}.png`
    await page.screenshot({ path: file })
    console.log(file, errors.length ? `ERRORS: ${errors.join(' | ')}` : 'ok')
    await ctx.close()
  }
}
await browser.close()
