// Drive a real, unpaused run through the app's own controls and capture the
// moments a static jump can't: the comet mid-transfer, the cable switch,
// the arrival flash, keyboard focus and reduced motion. Reports console errors.
// Usage: node scripts/flow.mjs [url]
import { mkdirSync } from 'node:fs'
import { chromium } from 'playwright'

const url = process.argv[2] ?? 'http://localhost:5180/'
const out = new URL('../shots/', import.meta.url).pathname
mkdirSync(out, { recursive: true })
const errors = []

const browser = await chromium.launch({ channel: 'chrome' })
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } })
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
page.on('pageerror', (e) => errors.push(String(e)))
await page.goto(url)
await page.waitForFunction(() => !!window.portal)
await page.evaluate(() => {
  window.portal.dispatch({ type: 'reset' })
  window.portal.theme('light')
})

const mac = page.locator('section[aria-label="Portal on MacBook Pro"]')
const win = page.locator('section[aria-label="Portal on Desk PC"]')
const phase = () => page.evaluate(() => document.querySelector('[role=progressbar]')?.getAttribute('aria-valuenow'))

// Files: click the Mac drop zone, accept on Windows, watch the glow
await mac.getByRole('button', { name: /Drop files or folders/ }).click()
await page.waitForTimeout(500)
await win.getByRole('button', { name: 'Accept' }).click()
await page.waitForTimeout(2600)
await page.screenshot({ path: `${out}flow-1-comet.png` })
console.log('progress', await phase())

// Plug in the cable mid-transfer (dev hook stands in for the physical cable)
await page.evaluate(() => window.portal.dispatch({ type: 'setLink', link: 'cable' }))
await page.waitForTimeout(500)
await page.screenshot({ path: `${out}flow-2-cable.png` })

// Arrival flash: capture just after the receiving window lands
await win.getByText('Received from MacBook Pro').waitFor({ timeout: 15000 })
await page.waitForTimeout(120)
await page.screenshot({ path: `${out}flow-3-arrival.png` })
await page.waitForTimeout(2200)
await page.screenshot({ path: `${out}flow-4-settled.png` })

// Details log expanded
await mac.getByRole('button', { name: 'Details' }).click()
await page.waitForTimeout(500)
await page.screenshot({ path: `${out}flow-5-details.png` })

// Keyboard focus: tab into the Windows window's controls
await page.keyboard.press('Tab')
await page.keyboard.press('Tab')
await page.keyboard.press('Tab')
await page.waitForTimeout(250)
await page.screenshot({ path: `${out}flow-6-focus.png` })

// Reduced motion during a project transfer
await page.emulateMedia({ reducedMotion: 'reduce', colorScheme: 'dark' })
await page.evaluate(() => {
  window.portal.theme('dark')
  window.portal.jump('transfer', 'win')
})
await page.waitForTimeout(800)
await page.screenshot({ path: `${out}flow-7-reduced.png` })

console.log(errors.length ? `CONSOLE ERRORS:\n${errors.join('\n')}` : 'no console errors')
await browser.close()
