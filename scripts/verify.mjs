// Click-through checks of every rule and path, using only the app's own controls.
import { chromium } from 'playwright'
const url = process.argv[2] ?? 'http://localhost:5180/'
const browser = await chromium.launch({ channel: 'chrome' })
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } })
const errors = []
page.on('pageerror', (e) => errors.push(String(e)))
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()))
await page.goto(url)
await page.waitForFunction(() => !!window.portal)
await page.evaluate(() => { window.portal.dispatch({ type: 'reset' }); window.portal.dispatch({ type: 'dev', patch: { speed: 2, projectKind: 'converts' } }) })
const mac = page.locator('section[aria-label="Portal on MacBook Pro"]')
const win = page.locator('section[aria-label="Portal on Desk PC"]')
const ok = async (label, loc) => { await loc.first().waitFor({ timeout: 15000 }); console.log('✓', label) }

// "Files from MacBook Pro: Never" on the PC (Settings → Devices)
const filesFrom = (choice) =>
  win.getByRole('radiogroup', { name: 'Files from MacBook Pro' }).getByRole('radio', { name: choice }).click()
await win.getByRole('button', { name: 'Settings' }).click()
await filesFrom('Never')
await win.getByRole('button', { name: 'Settings' }).click()
await mac.getByRole('button', { name: /Drop files/ }).click()
await ok('sender told which computer and which setting', mac.getByText('Desk PC doesn’t accept files from this Mac'))
await win.getByRole('button', { name: 'Settings' }).click()
await filesFrom('Ask every time')
await win.getByRole('button', { name: 'Settings' }).click()
await ok('allowing files again resumes as an offer', win.getByText('MacBook Pro wants to send'))
// Always accept, then accept
await win.getByRole('checkbox', { name: 'Always accept from MacBook Pro' }).check()
await win.getByRole('button', { name: 'Accept', exact: true }).click()
await ok('transfer runs', mac.getByText('Sending to Desk PC'))
await ok('files done on receiver', win.getByText('Received from MacBook Pro'))
await win.getByRole('button', { name: 'Show in File Explorer', exact: true }).click()
await win.getByRole('button', { name: 'Clear', exact: true }).click()
await mac.getByRole('button', { name: 'Done', exact: true }).click()
// Always accept skips the offer
await mac.getByRole('button', { name: /Drop files/ }).click()
await ok('always accept skips the offer', win.getByText('Accepted automatically'))
await ok('done again', win.getByText('Received from MacBook Pro'))
await win.getByRole('button', { name: 'Clear', exact: true }).click()
await mac.getByRole('button', { name: 'Done', exact: true }).click()

// Project, converts: Windows -> Mac this time, with Run
await win.getByRole('radio', { name: /Project/ }).click()
await win.getByRole('button', { name: /Drop a project folder/ }).click()
await mac.getByRole('button', { name: 'Accept', exact: true }).click()
await ok('Mac converting checklist', mac.getByText('Converting for Mac'))
await ok('Windows route chip', win.getByText(/converting for Mac/))
await ok('ready to run', mac.getByText('Converts · ready to run'))
await mac.getByRole('button', { name: 'Run', exact: true }).click()
await ok('running', mac.getByText(/Running at/))
await mac.getByRole('button', { name: 'Clear', exact: true }).click()
await win.getByRole('button', { name: 'Done', exact: true }).click()

// Project, native: Mac -> Windows, report actions
await page.evaluate(() => window.portal.dispatch({ type: 'dev', patch: { projectKind: 'native' } }))
await mac.getByRole('radio', { name: /Project/ }).click()
await mac.getByRole('button', { name: /Drop a project folder/ }).click()
await ok('rebuild report', win.getByText(/Rebuild · 3 fatal blockers/))
await win.getByRole('button', { name: 'Hand-off plan', exact: true }).click()
await ok('hand-off confirmation', win.getByText('Hand-off plan saved to Desktop.'))

// Decline path
await win.getByRole('button', { name: 'Clear', exact: true }).click()
await mac.getByRole('button', { name: 'Done', exact: true }).click()
await page.evaluate(() => window.portal.dispatch({ type: 'setAlways', side: 'win', value: false }))
await mac.getByRole('radio', { name: 'Files' }).click()
await mac.getByRole('button', { name: /Drop files/ }).click()
await win.getByRole('button', { name: 'Decline', exact: true }).click()
await ok('declined', mac.getByText('Desk PC declined'))

// Dev panel toggles with backtick
await page.keyboard.press('`')
await ok('dev panel opens', page.getByRole('complementary', { name: 'Dev panel' }))
await page.keyboard.press('`')
console.log(errors.length ? `ERRORS:\n${errors.join('\n')}` : 'no console errors')
await browser.close()
