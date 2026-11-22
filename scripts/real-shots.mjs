// Screenshots of the real app's screens against the mock core (src/dev/mockCore.ts).
// Usage: npm run dev, then node scripts/real-shots.mjs [label] [--only=scenario]
import { chromium } from 'playwright'
import { mkdirSync } from 'node:fs'

const label = process.argv.find((a, i) => i > 1 && !a.startsWith('--')) ?? 'real'
const only = process.argv.find((a) => a.startsWith('--only='))?.slice(7)
const base = process.env.PORTAL_URL ?? 'http://localhost:5180/'
const out = 'shots/real'
mkdirSync(out, { recursive: true })

// [file suffix, scenario, optional interaction]
const cases = [
  ['searching', 'searching'],
  ['searching-long', 'searching-long'],
  ['nearby', 'nearby'],
  ['revoked', 'revoked'],
  ['pairing-incoming', 'pairing-incoming'],
  ['pairing-connecting', 'nearby', async (p) => {
    await p.getByRole('button', { name: 'Pair', exact: true }).click()
  }],
  ['pairing-code', 'nearby', async (p) => {
    await p.getByRole('button', { name: 'Pair', exact: true }).click()
    await p.waitForTimeout(1800)
  }],
  ['pairing-waiting', 'nearby', async (p) => {
    await p.getByRole('button', { name: 'Pair', exact: true }).click()
    await p.waitForTimeout(1800)
    await p.getByRole('button', { name: 'Codes match' }).click()
  }],
  ['paired', 'nearby', async (p) => {
    await p.getByRole('button', { name: 'Pair', exact: true }).click()
    await p.waitForTimeout(1800)
    await p.getByRole('button', { name: 'Codes match' }).click()
    await p.waitForTimeout(2000)
  }],
  ['online', 'online'],
  ['online-ethernet', 'online-ethernet'],
  ['connecting', 'connecting'],
  ['offline', 'offline'],
  ['disconnected', 'disconnected'],
  ['disconnect-pressed', 'offline', async (p) => p.getByRole('button', { name: 'Stop trying' }).click()],
  ['settings-disconnected', 'disconnected', async (p) => p.getByRole('button', { name: 'Settings' }).click()],
  ['incompatible', 'incompatible'],
  ['connect', 'searching', async (p) => {
    await p.getByRole('button', { name: 'Connect by IP' }).first().click()
    await p.getByLabel('IP address').fill('desk-pc')
    await p.getByRole('button', { name: 'Connect', exact: true }).click()
  }],
  ['settings', 'online', async (p) => p.getByRole('button', { name: 'Settings' }).click()],
  ['about', 'online', async (p) => {
    await p.getByRole('button', { name: 'Settings' }).click()
    await p.waitForTimeout(500)
    await p.getByText('Install updates automatically').scrollIntoViewIfNeeded()
  }],
  ['about-offline', 'online&update=offline', async (p) => {
    await p.getByRole('button', { name: 'Settings' }).click()
    await p.waitForTimeout(500)
    await p.getByText('Install updates automatically').scrollIntoViewIfNeeded()
  }],
  ['clip-idle', 'online&clip=history'],
  ['clip-panel', 'online&clip=history', async (p) => p.getByRole('button', { name: /4 received/ }).click()],
  ['clip-asking', 'online&clip=asking'],
  ['clip-request', 'online&clip=request'],
  ['clip-request-answered', 'online&clip=request', async (p) => p.getByRole('button', { name: 'Turn on' }).click()],
  ['clip-sync', 'online&clip=sync'],
  ['clip-sent', 'online', async (p) => p.getByRole('button', { name: 'Send clipboard' }).click()],
  ['clip-private', 'online&clip=private', async (p) => p.getByRole('button', { name: 'Send clipboard' }).click()],
  ['clip-settings', 'online&clip=taken', async (p) => {
    await p.getByRole('button', { name: 'Settings' }).click()
    await p.waitForTimeout(500)
    await p.getByText('Send clipboard shortcut').scrollIntoViewIfNeeded()
  }],
  ['update-ready', 'online&update=ready'],
  ['update-waiting', 'sending&update=waiting'],
  ['update-failed', 'online&update=failed'],
  ['project', 'online', async (p) => p.getByRole('radio', { name: /Project/ }).click()],
  ['offer-in', 'offer-in'],
  ['offer-out', 'offer-out'],
  ['sending', 'sending'],
  ['receiving', 'receiving'],
  ['stalled', 'stalled'],
  ['paused-out', 'paused-out'],
  ['paused-in', 'paused-in'],
  ['resuming', 'resuming'],
  ['incompatible-self', 'incompatible-self'],
  ['incompatible-self-ready', 'incompatible-self&update=ready'],
  ['cancel-ask', 'sending', async (p) => p.getByRole('button', { name: 'Cancel' }).click()],
  ['paused-cancel-ask', 'paused-in', async (p) => p.getByRole('button', { name: 'Cancel' }).click()],
  ['cancel-done', 'sending', async (p) => {
    await p.getByRole('button', { name: 'Cancel' }).click()
    await p.getByRole('button', { name: 'Stop sending' }).click()
  }],
  ['cancelled', 'cancelled'],
  ['cancelled-in', 'cancelled-in'],
  ['firewall-fix', 'searching-long', async (p) => {
    const fix = p.getByRole('button', { name: 'Fix it' })
    if (await fix.count()) await fix.click()
    await p.waitForTimeout(1800)
  }],
  ['connect-own', 'searching', async (p) => {
    await p.getByRole('button', { name: 'Connect by IP' }).first().click()
    await p.getByLabel('IP address').fill(await p.locator('dd').first().innerText())
    await p.getByRole('button', { name: 'Connect', exact: true }).click()
  }],
  ['done-in', 'done-in'],
  ['done-out', 'done-out'],
  ['failed', 'failed'],
  ['blocked-peer', 'blocked-peer'],
  ['declined', 'declined'],
  ['accept-window', 'offer-in'],
  ['refused', 'refused', async (p) => {
    await p.getByRole('button', { name: /Drop files/ }).click()
    await p.waitForTimeout(2000)
    await p.getByRole('button', { name: 'Send', exact: true }).click()
  }],
  ['queue-counting', 'online', async (p) => {
    await p.getByRole('button', { name: /Drop files/ }).click()
  }],
  ['queue', 'online', async (p) => {
    await p.getByRole('button', { name: /Drop files/ }).click()
    await p.waitForTimeout(1800)
    await p.getByRole('button', { name: 'Add folder' }).click()
    await p.waitForTimeout(2000)
  }],
  ['queue-sent', 'online', async (p) => {
    await p.getByRole('button', { name: /Drop files/ }).click()
    await p.waitForTimeout(2000)
    await p.getByRole('button', { name: 'Send', exact: true }).click()
  }],
]

const browser = await chromium.launch({ channel: 'chrome' })
let failures = 0
for (const os of ['mac', 'win']) {
  for (const theme of ['light', 'dark']) {
    for (const [name, scenario, act] of cases) {
      if (only && only !== name) continue
      const size = name.startsWith('accept') ? { width: 440, height: 560 } : { width: 552, height: 780 }
      const ctx = await browser.newContext({ viewport: size, deviceScaleFactor: 2, colorScheme: theme })
      const page = await ctx.newPage()
      const errors = []
      page.on('pageerror', (e) => errors.push(e.message))
      page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
      const view = name.startsWith('accept') ? '&view=accept' : ''
      await page.goto(`${base}?mock=${scenario}&os=${os}${view}`)
      await page.locator('main').waitFor()
      await page.waitForTimeout(1200)
      if (act) {
        await act(page)
        await page.waitForTimeout(600)
      }
      const file = `${out}/${label}-${name}-${os}-${theme}.png`
      await page.screenshot({ path: file })
      if (errors.length) failures++
      console.log(file, errors.length ? `ERRORS: ${errors.join(' | ')}` : 'ok')
      await ctx.close()
    }
  }
}
await browser.close()
process.exit(failures ? 1 : 0)
