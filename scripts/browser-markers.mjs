// Mission 04 §4.3: which markers browsers put on what they copy, on Windows.
//
// For Firefox (a normal window, and a private one: private browsing on from
// the start, as Tor Browser runs), Edge and Chrome (a normal window, and an
// InPrivate / incognito one), it copies a sentence twice, with Ctrl+C on a
// selection and with a page's copy button (navigator.clipboard.writeText),
// then lists the clipboard by format name with examples/clip_formats (never
// the content) and what SeamDrop makes of it.
//
// "Normal" needs launchPersistentContext: Playwright's default contexts are
// off the record, like incognito. Firefox is Playwright's build; Edge and
// Chrome are the ones installed on the runner.
//
// Usage (Windows): cargo build -p portal-sys --example clip_formats
//                  node scripts/browser-markers.mjs [path to clip_formats.exe]
// Writes browser-markers.json and prints a table. Exits 0: it observes.
import { chromium, firefox } from 'playwright'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const tool = process.argv[2] ?? join('target', 'debug', 'examples', 'clip_formats.exe')
const PRIVATE = ['ExcludeClipboardContentFromMonitorProcessing', 'Clipboard Viewer Ignore', 'CanIncludeInClipboardHistory', 'CanUploadToCloudClipboard']

// A secure context (127.0.0.1), so navigator.clipboard works
const page = `<!doctype html><meta charset="utf-8"><p id="t">The quick brown fox jumps over the lazy dog</p>
<button id="b" onclick="navigator.clipboard.writeText('Written by the page').then(() => document.title = 'copied', e => document.title = 'refused: ' + e)">Copy</button>`
const server = createServer((_, res) => res.end(page)).listen(0, '127.0.0.1')
await new Promise((r) => server.once('listening', r))
const url = `http://127.0.0.1:${server.address().port}/`

const profile = () => mkdtempSync(join(tmpdir(), 'markers-'))
const configs = [
  ['Firefox', 'normal', () => firefox.launchPersistentContext(profile(), { headless: false })],
  [
    'Firefox',
    'private',
    () => firefox.launchPersistentContext(profile(), { headless: false, firefoxUserPrefs: { 'browser.privatebrowsing.autostart': true } }),
  ],
  ['Edge', 'normal', () => chromium.launchPersistentContext(profile(), { channel: 'msedge', headless: false })],
  ['Edge', 'InPrivate', async () => (await chromium.launch({ channel: 'msedge', headless: false })).newContext()],
  ['Chrome', 'normal', () => chromium.launchPersistentContext(profile(), { channel: 'chrome', headless: false })],
  ['Chrome', 'incognito', async () => (await chromium.launch({ channel: 'chrome', headless: false })).newContext()],
]

function inspect() {
  try {
    return JSON.parse(execFileSync(tool, { encoding: 'utf8' }).trim())
  } catch (e) {
    return { error: String(e) }
  }
}

const results = []
for (const [browser, mode, open] of configs) {
  let context
  try {
    context = await open()
  } catch (e) {
    results.push({ browser, mode, error: `couldn't start: ${String(e).split('\n')[0]}` })
    continue
  }
  try {
    if (browser !== 'Firefox') await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: url })
    const p = context.pages()[0] ?? (await context.newPage())
    await p.goto(url)
    await p.bringToFront()
    // Ctrl+C on a selection
    await p.evaluate(() => getSelection().selectAllChildren(document.getElementById('t')))
    await p.keyboard.press('Control+C')
    await p.waitForTimeout(600)
    results.push({ browser, mode, method: 'Ctrl+C', ...inspect() })
    // The page's copy button
    await p.click('#b')
    await p.waitForTimeout(600)
    results.push({ browser, mode, method: 'writeText', title: await p.title(), ...inspect() })
  } catch (e) {
    results.push({ browser, mode, error: String(e).split('\n')[0] })
  } finally {
    await context.close().catch(() => {})
  }
}
server.close()

writeFileSync('browser-markers.json', JSON.stringify(results, null, 2))
const rows = results.map((r) => {
  if (r.error) return `${r.browser.padEnd(8)} ${r.mode.padEnd(10)} ${(r.method ?? '').padEnd(10)} ERROR ${r.error}`
  const marks = r.formats.filter((f) => PRIVATE.includes(f))
  const can = `history=${r.canIncludeInClipboardHistory ?? '-'} cloud=${r.canUploadToCloudClipboard ?? '-'}`
  return `${r.browser.padEnd(8)} ${r.mode.padEnd(10)} ${r.method.padEnd(10)} ${(marks.join(', ') || 'no markers').padEnd(60)} ${can.padEnd(22)} source=${r.source ?? '-'}  SeamDrop: ${r.seamdrop}`
})
console.log(['browser  mode       method     markers' + ' '.repeat(53) + 'Can… values', ...rows].join('\n'))
process.exit(0)
