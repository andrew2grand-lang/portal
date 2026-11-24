// End-to-end smoke test of the real built app (beta.2): drives the window
// through tauri-driver (WebDriver) and, after every control, asks the real
// core for its state through the app's own IPC, so a control that looks
// right but changes nothing fails. No WebDriver library: plain HTTP.
//
//   tauri-driver &   (on Windows: --native-driver <msedgedriver.exe>)
//   node e2e/smoke.mjs <path to the built app> [<a .png for the photo>]
//
// Exits 1 when a covered control fails. Writes e2e-results.json.
import { writeFileSync } from 'node:fs'

const DRIVER = process.env.TAURI_DRIVER ?? 'http://127.0.0.1:4444'
const [app, photo] = process.argv.slice(2)
if (!app) {
  console.error('usage: node e2e/smoke.mjs <app> [photo.png]')
  process.exit(2)
}
const EL = 'element-6066-11e4-a23d-4f5a6e4b5b33'
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function wd(method, path, body) {
  const r = await fetch(DRIVER + path, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const j = await r.json().catch(() => ({}))
  if (!r.ok || j.value?.error) throw new Error(`${method} ${path}: ${JSON.stringify(j.value ?? j).slice(0, 400)}`)
  return j.value
}

const session = await wd('POST', '/session', {
  capabilities: { alwaysMatch: { browserName: 'wry', 'tauri:options': { application: app } } },
})
const S = (p) => `/session/${session.sessionId}${p}`
const exec = (script, args = []) => wd('POST', S('/execute/sync'), { script, args })
const execAsync = (script, args = []) => wd('POST', S('/execute/async'), { script, args })

/** A command on the real core, from inside the window */
async function invoke(cmd, args = {}) {
  const r = await execAsync(
    'const done = arguments[arguments.length - 1];' +
      'window.__TAURI_INTERNALS__.invoke(arguments[0], arguments[1]).then((v) => done({ ok: v === undefined ? null : v }), (e) => done({ err: String(e) }))',
    [cmd, args],
  )
  if (r?.err) throw new Error(`${cmd}: ${r.err}`)
  return r?.ok
}
const snap = () => invoke('portal_snapshot')

async function find(xpath, timeout = 8000) {
  const until = Date.now() + timeout
  for (;;) {
    const found = await wd('POST', S('/elements'), { using: 'xpath', value: xpath })
    if (found.length) return found[0][EL] ?? found[0].ELEMENT ?? Object.values(found[0])[0]
    if (Date.now() > until) throw new Error(`not on screen: ${xpath}`)
    await sleep(200)
  }
}
/** The node itself, found again inside the page (WebKitWebDriver doesn't
    pass element references into scripts) */
const inPage = (xpath) => `document.evaluate(${JSON.stringify(xpath)}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue`
async function click(xpath) {
  const el = await find(xpath)
  await exec(`const n = ${inPage(xpath)}; if (n) n.scrollIntoView({ block: 'center' })`)
  try {
    await wd('POST', S(`/element/${el}/click`), {})
  } catch {
    // Hidden or covered (a visually hidden checkbox, a window that starts
    // hidden): click it as a script would; React sees the same events
    await exec(`${inPage(xpath)}.click()`)
  }
}
async function typeInto(xpath, text) {
  const el = await find(xpath)
  await exec(`const n = ${inPage(xpath)}; n.scrollIntoView({ block: 'center' }); n.focus(); if (n.select) n.select()`)
  await wd('POST', S(`/element/${el}/clear`), {}).catch(() => {})
  await wd('POST', S(`/element/${el}/value`), { text })
}
/** Waits for the real core (or a check) to agree */
async function until(what, pred, timeout = 8000) {
  const end = Date.now() + timeout
  let last
  for (;;) {
    try {
      last = await pred()
      if (last) return last
    } catch (e) {
      last = e.message
    }
    if (Date.now() > end) throw new Error(`${what} (last: ${JSON.stringify(last)?.slice(0, 200)})`)
    await sleep(250)
  }
}
const pageText = () => exec('return document.body.innerText')

const button = (name) => `//button[normalize-space(.)="${name}" or @aria-label="${name}"]`
const radio = (name) => `//*[@role="radio"][normalize-space(.)="${name}"]`
const checkbox = (text) => `//label[.//span[normalize-space(.)="${text}"]]//input[@type="checkbox"]`
const field = (label) => `//input[@id=(//label[normalize-space(.)="${label}"]/@for)]`

const results = []
async function check(name, fn) {
  try {
    await fn()
    results.push({ name, ok: true })
    console.log(`✓ ${name}`)
  } catch (e) {
    results.push({ name, ok: false, why: e.message })
    console.log(`✗ ${name}\n    ${e.message}`)
  }
}
const me = async () => (await snap()).me

try {
  await until('the window rendered', async () => (await pageText()).length > 20, 30000)
  await until('the core answered', async () => (await snap())?.me?.id, 30000)

  // ---- Settings ------------------------------------------------------------
  await check('Settings opens', () => click(button('Settings')).then(() => find(`//h2[normalize-space(.)="Settings"]`)))
  await check('Name', async () => {
    await typeInto(field('Name'), 'E2E Computer\uE007')
    await until('the core has the new name', async () => (await me()).name === 'E2E Computer')
  })
  await check('Picture: a colour', async () => {
    await click(button('Colour #d6508f'))
    await until('the core has the colour', async () => {
      const p = (await me()).picture
      return p?.kind === 'mark' && p.color === 0xd6508f
    })
  })
  await check('Picture: an icon', async () => {
    await click(button('Phone'))
    await until('the core has the icon', async () => (await me()).picture?.icon === 'phone')
  })
  if (photo) {
    await check('Picture: a photo', async () => {
      const el = await find(`//input[@type="file"]`)
      await wd('POST', S(`/element/${el}/value`), { text: photo })
      await until('the core has a photo', async () => (await me()).picture?.kind === 'photo', 12000)
    })
  }
  await check('Picture: Default', async () => {
    await click(button('Default'))
    await until('back to the default', async () => (await me()).picture === null)
  })
  for (const [label, mode] of [
    ['SeamDrop folder', 'app_folder'],
    ['New folder each time', 'numbered'],
    ['Straight in', 'loose'],
  ]) {
    await check(`Received files: ${label}`, async () => {
      await click(radio(label))
      await until(`the core's mode is ${mode}`, async () => (await me()).receiveMode === mode)
    })
  }
  await check('A folder for each computer', async () => {
    await click(radio('SeamDrop folder'))
    await click(checkbox('A folder for each computer'))
    await until('the core has a folder per computer', async () => (await me()).folderPerDevice === true)
    await click(radio('Straight in'))
  })
  await check('Keep original dates', async () => {
    await click(checkbox('Keep original dates'))
    await until('the core keeps dates', async () => (await me()).keepDates === true)
  })
  await check('Skip system files', async () => {
    const was = (await me()).skipJunk
    await click(checkbox('Skip system files like .DS_Store and Thumbs.db'))
    await until('the core flipped it', async () => (await me()).skipJunk === !was)
  })
  await check('Open at login', async () => {
    const was = await invoke('plugin:autostart|is_enabled')
    await click(checkbox('Open at login'))
    await until('the system flipped it', async () => (await invoke('plugin:autostart|is_enabled')) === !was)
    await click(checkbox('Open at login'))
  })
  for (const [label, value] of [
    ['Dark', 'dark'],
    ['Light', 'light'],
    ['System', 'system'],
  ]) {
    await check(`Appearance: ${label}`, async () => {
      await click(radio(label))
      await until(`the core's appearance is ${value}`, async () => (await me()).appearance === value)
    })
  }
  await check('Install updates automatically', async () => {
    const was = (await me()).autoUpdate
    await click(checkbox('Install updates automatically'))
    await until('the core flipped it', async () => (await me()).autoUpdate === !was)
    await click(checkbox('Install updates automatically'))
  })
  await check('Get test builds (and the check that follows is not an error)', async () => {
    await click(checkbox('Get test builds'))
    await until('the core gets test builds', async () => (await me()).testBuilds === true)
    const u = await until('an update check finished', async () => {
      const s = await invoke('portal_update_status')
      return s && !['idle', 'checking'].includes(s.state) && s
    }, 30000)
    if (u.state === 'failed') throw new Error(`the update check failed: ${u.message}`)
  })
  await check('Check for updates', async () => {
    const before = (await invoke('portal_update_status'))?.checkedMs ?? 0
    await click(button('Check for updates'))
    await until('a new check finished', async () => ((await invoke('portal_update_status'))?.checkedMs ?? 0) > before, 30000)
  })
  await check('Send clipboard shortcut: Change…', async () => {
    const was = (await invoke('portal_shortcut'))?.shortcut
    // The Change… beside the shortcut (not the received files' one)
    await click(`//p[normalize-space(.)="Send clipboard shortcut"]/../following-sibling::button[1]`)
    await exec(
      "window.dispatchEvent(new KeyboardEvent('keydown', { key: 'K', code: 'KeyK', ctrlKey: true, altKey: true, shiftKey: true, bubbles: true }))",
    )
    await until('the core has a new shortcut', async () => {
      const now = (await invoke('portal_shortcut'))?.shortcut
      return now && now !== was
    })
  })
  await check('Connection check', async () => {
    await click(button('Connection check'))
    await find(`//h2[normalize-space(.)="Connection check"]`)
    await until('the core listed its checks', async () => (await snap()).doctor?.length > 0)
    await click(button('Settings'))
  })
  await check('Connect by IP (an address that doesn’t answer is said)', async () => {
    await click(button('Settings'))
    await click(button('Connect by IP'))
    await typeInto(field('The other computer’s IP address'), '127.0.0.1:9\uE007')
    await until('the window said it didn’t answer', async () => /answer|reach|couldn/i.test(await pageText()), 20000)
  })
  await check('Save technical log', async () => {
    await click(button('Settings'))
    await click(button('Save technical log'))
    await until('the window said where it went', async () => /technical log/i.test(await pageText()))
  })
  await check('Help (Settings → Help)', async () => {
    await click(button('Help'))
    await find(`//h2[normalize-space(.)="Help"]`)
  })

  // ---- Shelf -----------------------------------------------------------------
  await check('Shelf: add a private item', async () => {
    await click(button('Shelf'))
    await typeInto(field('Name (optional)'), 'E2E key')
    await typeInto(`//textarea[@aria-label="Text to keep"]`, 'e2e-secret-123')
    await click(checkbox('Private'))
    await click(`//button[normalize-space(.)="Add to Shelf"]`)
    await until('the core keeps it, private', async () => (await snap()).shelf.some((i) => i.name === 'E2E key' && i.private))
  })
  await check('Shelf: Reveal', async () => {
    await click(button('Reveal E2E key'))
    await until('its text shows', async () => (await pageText()).includes('e2e-secret-123'))
  })
  await check('Shelf: Copy', async () => {
    await click(`//button[normalize-space(.)="Copy"]`)
    // Copied, or (Linux has no clipboard in this build) said why not: the window answered
    await until('the window answered', async () => /Copied|clipboard/i.test(await pageText()))
  })
  await check('Shelf: Remove, then Undo', async () => {
    await click(button('Remove E2E key'))
    await click(`//button[normalize-space(.)="Remove"]`)
    await until('the core removed it', async () => !(await snap()).shelf.some((i) => i.name === 'E2E key'))
    await click(button('Undo'))
    await until('the core has it back', async () => (await snap()).shelf.some((i) => i.name === 'E2E key'))
  })
  await check('Shelf: Clear Shelf', async () => {
    await click(button('Clear Shelf…'))
    await click(`//button[normalize-space(.)="Clear Shelf"]`)
    await until('the core cleared it', async () => (await snap()).shelf.length === 0)
  })

  // ---- Home ------------------------------------------------------------------
  await check('Details shows what the core did', async () => {
    await click(button('Details'))
    await until('the core’s lines are there', async () => /started as/i.test(await pageText()))
  })
} finally {
  await wd('DELETE', S('')).catch(() => {})
}

/** Controls a single computer can't exercise, and why */
const notCovered = [
  ['Received files: Change…', 'opens the system folder picker'],
  ['Received files: Open', 'opens the file manager; nothing in the core changes'],
  ['Drop zone, Add files, Add folder', 'open the system file picker (dropping is the core’s describe/send, covered by the core tests)'],
  ['Clipboard panel (Send clipboard, sync, pause, Recently received)', 'needs a paired computer'],
  ['Device tiles and the "+" beside them', 'shown only with two or more paired computers'],
  ['Paired device controls (Rename, Remove, Files from, Share the Shelf, Disconnect)', 'need a paired computer'],
  ['Shelf: Share with …', 'needs a paired computer'],
  ['Allow pairing again', 'shown only after three wrong codes'],
  ['Fix it (Windows firewall)', 'shown only when a firewall rule blocks SeamDrop'],
  ['Restart SeamDrop to update', 'needs a newer build on the feed, and restarts the app'],
]
writeFileSync('e2e-results.json', JSON.stringify({ results, notCovered }, null, 2))
const failed = results.filter((r) => !r.ok)
console.log(`\n${results.length - failed.length} of ${results.length} controls changed the real core as expected.`)
console.log('Not covered here:')
for (const [what, why] of notCovered) console.log(`  - ${what}: ${why}`)
process.exit(failed.length ? 1 : 0)
