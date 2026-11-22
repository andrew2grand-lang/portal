// Mission 04 §1.5: no text overlaps or is cut off, in any state, at any size.
//
// Walks every state of the real window against the mock core (the dev
// server's /?mock=…), at 420×640 (the minimum), 552×780, 800×900 and
// 1280×900, light and dark, with 1 and with 6 paired devices, and asserts:
//   - no two visible pieces of text overlap (their line boxes, not their
//     elements' boxes);
//   - no text is cut off: by an ancestor that clips without scrolling, by the
//     window's edge, or by an ellipsis or line clamp that hides part of it.
//     (Text below the fold of a scrolling area isn't cut off: it scrolls.)
//
// Left out, decided up front:
//   - aria-hidden subtrees (decoration: the glow, icons, the orb);
//   - visually hidden text (.sr-only and anything 1×1 with clip);
//   - invisible elements (display none, visibility hidden, opacity 0);
//   - anything mid-animation: each check waits until the text's boxes are
//     the same twice in a row.
//
// Usage: node scripts/with-dev-server.mjs scripts/overlap.mjs [--os=mac|win]
//   [--engine=chromium|webkit|msedge|chrome] [--themes=light,dark]
//   [--only=scenario] [--shots=dir]
// CI: the Mac chrome in WebKit on a macOS runner, the Windows chrome in Edge
// on a Windows runner, light theme (the themes change colours, not sizes;
// both pass locally), main and manual runs only.
// --shots: one picture per state at the default window size (552×780): the
// Mac chrome light with one device, the Windows chrome dark with six.
import { chromium, webkit } from 'playwright'
import { mkdirSync } from 'node:fs'

const arg = (name, fallback) => process.argv.find((a) => a.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback
const base = process.env.PORTAL_URL ?? 'http://localhost:5180/'
const oses = arg('os', 'mac,win').split(',')
const engine = arg('engine', 'chromium')
const only = arg('only', '')
const shots = arg('shots', '')
const SIZES = [
  // Mission 05 §5: mini mode (under 360 wide or 480 high), and the new minimum
  [300, 360],
  [340, 460],
  [420, 640],
  [552, 780],
  [800, 900],
  [1280, 900],
]
const THEMES = arg('themes', 'light,dark').split(',')
const DEVICES = [1, 6]

const pick = (p, name) => p.getByRole('button', { name, exact: true }).first().click()
// [name, query, optional interaction]: every state the window can be in
const CASES = [
  ['searching', 'searching'],
  ['searching-long', 'searching-long'],
  ['nearby', 'nearby'],
  ['revoked', 'revoked'],
  ['pairing-incoming', 'pairing-incoming'],
  ['pairing-asking', 'nearby', async (p) => {
    await pick(p, 'Pair')
    await p.waitForTimeout(300)
  }],
  ['pairing-code', 'nearby', async (p) => {
    await pick(p, 'Pair')
    await p.waitForTimeout(1800)
  }],
  ['pairing-show', 'pairing-show'],
  ['pairing-wrong-there', 'pairing-wrong-there'],
  ['pairing-type', 'pairing-type'],
  ['pairing-wrong-here', 'pairing-wrong-here'],
  ['pairing-paired', 'pairing-paired'],
  ['pairing-locked', 'pairing-locked', async (p) => {
    await pick(p, 'Settings')
    await p.waitForTimeout(500)
    await pick(p, 'Pair another computer')
  }],
  ['online', 'online'],
  ['online-ethernet', 'online-ethernet'],
  ['connecting', 'connecting'],
  ['offline', 'offline'],
  ['disconnected', 'disconnected'],
  ['incompatible', 'incompatible'],
  ['incompatible-self', 'incompatible-self&update=ready'],
  ['offer-in', 'offer-in'],
  ['offer-out', 'offer-out'],
  ['sending', 'sending'],
  ['receiving', 'receiving'],
  ['stalled', 'stalled'],
  ['cancel-ask', 'sending', (p) => pick(p, 'Cancel')],
  ['paused-out', 'paused-out'],
  ['paused-in', 'paused-in'],
  ['resuming', 'resuming'],
  ['done-in', 'done-in'],
  ['done-out', 'done-out'],
  ['first-arrival', 'done-in&first=1'],
  ['failed', 'failed'],
  ['cancelled', 'cancelled'],
  ['cancelled-in', 'cancelled-in'],
  ['blocked-peer', 'blocked-peer'],
  ['declined', 'declined'],
  ['refused', 'refused'],
  ['queue', 'online', async (p) => {
    await p.locator('button[class*="drop"]').first().click()
    await p.waitForTimeout(1800)
  }],
  ['update-ready', 'online&update=ready'],
  ['update-failed', 'online&update=failed'],
  // Mission 05 §3: Clipboard and Shelf are two places, each from its key
  ['clip-sent', 'online&clip=history', async (p) => {
    await pick(p, 'Clipboard')
    await p.waitForTimeout(400)
    await pick(p, 'Send clipboard')
  }],
  ['clip-private', 'online&clip=private', async (p) => {
    await pick(p, 'Clipboard')
    await p.waitForTimeout(400)
    await pick(p, 'Send clipboard')
  }],
  ['clip-files', 'online&clip=files', async (p) => {
    await pick(p, 'Clipboard')
    await p.waitForTimeout(400)
    await pick(p, 'Send clipboard')
  }],
  ['clip-request', 'online&clip=request'],
  ['clip-sync', 'online&clip=sync'],
  ['clipboard', 'online&clip=history', (p) => pick(p, 'Clipboard')],
  ['clipboard-paused', 'online&clip=paused', (p) => pick(p, 'Clipboard')],
  ['clipboard-sync', 'online&clip=sync', (p) => pick(p, 'Clipboard')],
  ['clipboard-notice', 'online&clip=notice', (p) => pick(p, 'Clipboard')],
  ['clipboard-toobig', 'online&clip=toobig', (p) => pick(p, 'Clipboard')],
  ['shelf', 'online&shelf=1', (p) => pick(p, 'Shelf')],
  ['shelf-empty', 'online', (p) => pick(p, 'Shelf')],
  ['shelf-ask', 'online&shelf=1&devices=2'],
  ['help', 'online', async (p) => {
    await pick(p, 'Clipboard')
    await p.waitForTimeout(400)
    await pick(p, 'How the clipboard works')
  }],
  // Review 05: a copy that wasn't synced is said in the main window
  ['clip-notice', 'online&clip=notice'],
  ['settings', 'online&update=ready', (p) => pick(p, 'Settings')],
  // Mission 05 §4: received files in a SeamDrop folder, one for each computer
  ['settings-folders', 'online', async (p) => {
    await pick(p, 'Settings')
    await p.waitForTimeout(500)
    await p.getByRole('radio', { name: 'SeamDrop folder' }).click({ force: true })
    await p.waitForTimeout(300)
    // The box itself, pressed where it is (its input lies over the label here;
    // WebKit won't click the input under its drawn box without force)
    await p.getByRole('checkbox', { name: 'A folder for each computer' }).click({ force: true })
  }],
  ['settings-bottom', 'online&clip=sync', async (p) => {
    await pick(p, 'Settings')
    await p.waitForTimeout(600)
    await p.getByText('Install updates automatically').scrollIntoViewIfNeeded()
  }],
  ['connect', 'searching', (p) => pick(p, 'Connect by IP')],
  ['doctor', 'online', async (p) => {
    await pick(p, 'Settings')
    await p.waitForTimeout(500)
    await pick(p, 'Connection check')
  }],
  ['add-device', 'online', async (p) => {
    await pick(p, 'Settings')
    await p.waitForTimeout(500)
    await pick(p, 'Pair another computer')
    // Settings fades out as this comes in: measure the screen, not the crossing
    await p.waitForTimeout(600)
  }],
  ['details', 'sending', (p) => pick(p, 'Details')],
  ['many-sending', 'many-sending'],
  ['many-mixed', 'many-mixed'],
  ['many-offers', 'many-offers'],
  // Review 04: what was left out or renamed, opened
  ['done-which', 'done-in', async (p) => {
    for (const s of await p.locator('summary').all()) await s.click()
  }],
  // Review 04: more files while transfers run (the send list over them), and
  // the strip that offers it
  ['send-more', 'many-sending', (p) => pick(p, 'Add files')],
  ['send-more-one', 'sending&devices=3'],
  // Mission 05 §3.2: the Shelf's private text, Reveal, remove and Clear
  ['shelf-private', 'online', async (p) => {
    await pick(p, 'Shelf')
    await p.waitForTimeout(400)
    // The box's input lies over its label here, so press the box itself
    await p.getByRole('checkbox', { name: 'Private' }).click({ force: true })
    await p.getByRole('textbox', { name: 'Text to keep' }).fill('hunter2 is the password')
  }],
  ['shelf-revealed', 'online&shelf=1', async (p) => {
    await pick(p, 'Shelf')
    await p.waitForTimeout(400)
    await p.getByRole('button', { name: /^Reveal/ }).first().click()
  }],
  ['shelf-remove-ask', 'online&shelf=1', async (p) => {
    await pick(p, 'Shelf')
    await p.waitForTimeout(400)
    await p.getByRole('button', { name: /^Remove / }).first().click()
  }],
  ['shelf-clear', 'online&shelf=1', async (p) => {
    await pick(p, 'Shelf')
    await p.waitForTimeout(400)
    await pick(p, 'Clear Shelf…')
  }],
  // Mission 05 §1.4, §1.5, §1.8
  ['stopped-out', 'stopped-out'],
  ['stopped-in', 'stopped-in'],
  ['waiting-there', 'waiting-there'],
  ['next', 'next'],
  ['nearby-three', 'nearby&devices=3'],
]

const launcher = engine === 'webkit' ? webkit : chromium
const channel = engine === 'msedge' || engine === 'chrome' ? engine : undefined
const browser = await launcher.launch({ channel })
console.log(`engine: ${engine} ${browser.version()}`)
if (shots) mkdirSync(shots, { recursive: true })

/** Runs in the page: every visible text run's line boxes, and what cuts them off */
function measure() {
  const hidden = (el) => {
    for (let e = el; e && e !== document.documentElement; e = e.parentElement) {
      if (e.getAttribute('aria-hidden') === 'true' || e.classList.contains('sr-only')) return true
      const cs = getComputedStyle(e)
      if (cs.display === 'none' || cs.visibility === 'hidden' || Number(cs.opacity) === 0) return true
      if (cs.position === 'absolute' && cs.clip !== 'auto' && e.offsetWidth <= 1) return true
      // A closed <details>: only its summary shows
      if (e.tagName === 'DETAILS' && !e.open && !el.closest('summary')) return true
    }
    return false
  }
  const label = (el) => {
    const t = (el.textContent ?? '').trim().replace(/\s+/g, ' ')
    return `<${el.tagName.toLowerCase()}${el.className && typeof el.className === 'string' ? ` .${el.className.split(' ')[0]}` : ''}> “${t.slice(0, 50)}”`
  }
  const runs = []
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    if (!n.textContent?.trim()) continue
    const el = n.parentElement
    if (!el || hidden(el)) continue
    const range = document.createRange()
    range.selectNodeContents(n)
    // Inside a scrolling area only the part in view counts (the rest is
    // scrolled away, not under anything)
    let view = null
    for (let e = el; e && e !== document.body; e = e.parentElement) {
      const cs = getComputedStyle(e)
      if (['auto', 'scroll'].includes(cs.overflowY) || ['auto', 'scroll'].includes(cs.overflowX)) {
        view = e.getBoundingClientRect()
        break
      }
    }
    const raw = [...range.getClientRects()].filter((r) => r.width > 0.5 && r.height > 0.5)
    // Areas here scroll up and down only: text running off a side is cut off
    const offSide = view && raw.some((r) => r.right > view.right + 1 || r.left < view.left - 1)
    const rects = raw
      .map((r) =>
        view
          ? new DOMRect(
              Math.max(r.left, view.left),
              Math.max(r.top, view.top),
              Math.min(r.right, view.right) - Math.max(r.left, view.left),
              Math.min(r.bottom, view.bottom) - Math.max(r.top, view.top),
            )
          : r,
      )
      .filter((r) => r.width > 0.5 && r.height > 0.5)
    if (rects.length || offSide) runs.push({ el, rects, label: label(el), offSide })
  }
  const problems = []
  // Overlaps: line boxes of different text runs
  for (let i = 0; i < runs.length; i++) {
    for (let j = i + 1; j < runs.length; j++) {
      for (const a of runs[i].rects) {
        for (const b of runs[j].rects) {
          const w = Math.min(a.right, b.right) - Math.max(a.left, b.left)
          const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top)
          if (w > 2 && h > 2) {
            problems.push(`overlap: ${runs[i].label} × ${runs[j].label}`)
            break
          }
        }
      }
    }
  }
  // Cut off: by a clipping (not scrolling) ancestor, the window, an ellipsis or a clamp
  const vw = document.documentElement.clientWidth
  const vh = document.documentElement.clientHeight
  for (const run of runs) {
    const cut = (why) => problems.push(`cut off (${why}): ${run.label}`)
    if (run.offSide) cut('past the side of a scrolling area')
    let scrolls = false
    for (let e = run.el; e && e !== document.body; e = e.parentElement) {
      const cs = getComputedStyle(e)
      const ox = cs.overflowX
      const oy = cs.overflowY
      if (oy === 'auto' || oy === 'scroll' || ox === 'auto' || ox === 'scroll') {
        scrolls = true
        break
      }
      if (cs.textOverflow === 'ellipsis' && e.scrollWidth > e.clientWidth + 1) cut('ellipsis')
      if (cs.webkitLineClamp && cs.webkitLineClamp !== 'none' && e.scrollHeight > e.clientHeight + 1) cut('line clamp')
      // A box that hides its overflow cuts text only if the box really
      // overflows and the text lies past its edge (glyphs may poke a pixel
      // or two out of a tight line height; that isn't cut).
      if (ox === 'hidden' || ox === 'clip' || oy === 'hidden' || oy === 'clip') {
        const overX = e.scrollWidth > e.clientWidth + 1
        const overY = e.scrollHeight > e.clientHeight + 1
        if (overX || overY) {
          const box = e.getBoundingClientRect()
          for (const r of run.rects) {
            const slack = Math.max(2, r.height * 0.25)
            const pastX = overX && (r.left < box.left - 1 || r.right > box.right + 1)
            const pastY = overY && (r.top < box.top - slack || r.bottom > box.bottom + slack)
            if (pastX || pastY) {
              cut(`by ${label(e)}`)
              break
            }
          }
        }
      }
    }
    if (!scrolls) {
      for (const r of run.rects) {
        const slack = Math.max(2, r.height * 0.25)
        if (r.right > vw + 1 || r.bottom > vh + slack || r.left < -1) {
          cut('by the window edge')
          break
        }
      }
    }
  }
  // A segmented control hugs its options (Mission 05 §1.7): a track stretched
  // past its last option leaves an empty, clickable-looking stretch
  for (const g of document.querySelectorAll('[role="radiogroup"]')) {
    const opts = g.querySelectorAll('[role="radio"]')
    if (!opts.length || g.closest('details:not([open])')) continue
    const track = g.getBoundingClientRect()
    if (!track.width) continue
    const last = Math.max(...[...opts].map((o) => o.getBoundingClientRect().right))
    const pad = parseFloat(getComputedStyle(g).paddingRight) || 0
    if (track.right - last - pad > 6) problems.push(`stretched control: ${g.getAttribute('aria-label') ?? 'a choice'} (${Math.round(track.right - last - pad)} px empty)`)
  }
  // A page with no text at all hasn't rendered (a reload mid-check, a crash):
  // it must never pass for want of anything to measure
  // (Mini mode is all icons: keys on screen count as rendered)
  if (!runs.length && !document.querySelector('button')) problems.push('nothing on screen: the window didn’t render')
  return { problems: [...new Set(problems)], signature: runs.map((r) => r.rects.map((x) => `${x.x | 0},${x.y | 0},${x.width | 0}`).join(';')).join('|') }
}

/** Until the text stops moving. An empty page isn't settled: it hasn't
    rendered yet (a cold dev server takes a few seconds for the first one). */
async function settled(page) {
  let last = null
  for (let i = 0; i < 40; i++) {
    const { signature } = await page.evaluate(measure)
    if (signature && signature === last) return
    last = signature
    await page.waitForTimeout(250)
  }
}

let failures = 0
let checked = 0
const jobs = []
for (const os of oses) for (const [name, query, act] of CASES) {
  if (only && name !== only) continue
  for (const devices of DEVICES) for (const theme of THEMES) jobs.push({ os, name, query, act, devices, theme })
}

async function run(job) {
  const { os, name, query, act, devices, theme } = job
  // Opened at the normal size, where every state can be reached (smaller
  // sizes include mini mode, which hides the panels)
  const page = await browser.newPage({ viewport: { width: 552, height: 780 }, colorScheme: theme })
  const errors = []
  page.on('pageerror', (e) => errors.push(e.message))
  // One more try when the dev server is slow to answer (it is when other work
  // runs alongside); a second timeout is a real failure
  // still=1: simulated transfers hold where they start (they'd finish mid-check otherwise)
  const url = `${base}?mock=${query}&os=${os}&devices=${devices}&still=1`
  await page.goto(url).catch(() => page.goto(url))
  // The window, rendered (not the empty page before the app has loaded)
  await page.waitForFunction(() => document.body.innerText.trim().length > 0, null, { timeout: 20_000 }).catch(() => {})
  await page.waitForTimeout(400)
  if (act) {
    await settled(page)
    // A step that can't be done is a problem for this state, not the end of the run
    await act(page).catch((e) => errors.push(`couldn't get to this state: ${String(e.message ?? e).split('\n')[0]}`))
  }
  for (const [w, h] of SIZES) {
    await page.setViewportSize({ width: w, height: h })
    await settled(page)
    // A page that throws (or reloads) mid-measure is a problem for this
    // layout, said with the error, not the end of the whole run
    const { problems } = await page.evaluate(measure).catch((e) => ({ problems: [`couldn't measure: ${String(e.message ?? e).split('\n')[0]}`] }))
    checked++
    const where = `${name} · ${os} · ${theme} · ${devices} device${devices > 1 ? 's' : ''} · ${w}×${h}`
    if (problems.length || errors.length) {
      failures++
      console.log(`✗ ${where}\n  ${[...errors.map((e) => `error: ${e}`), ...problems].join('\n  ')}`)
    }
    const pictured = (os === 'mac' && theme === 'light' && devices === 1) || (os === 'win' && theme === 'dark' && devices === 6)
    if (shots && w === 552 && pictured) {
      await page.screenshot({ path: `${shots}/m05-${name}-${os}-${theme}-${devices}dev.jpg`, type: 'jpeg', quality: 80 })
    }
  }
  await page.close()
}

// A few pages at once
const queue = [...jobs]
await Promise.all(
  Array.from({ length: 4 }, async () => {
    for (let job = queue.shift(); job; job = queue.shift()) await run(job)
  }),
)
await browser.close()
console.log(`\n${checked} layouts checked (${jobs.length} states × ${SIZES.length} sizes), ${failures} with problems`)
process.exit(failures ? 1 : 0)
