// Combines screenshots into one image for quick review.
// Usage: node scripts/contact-sheet.mjs out.png 3 a.png b.png ...
import { chromium } from 'playwright'
import { readFileSync } from 'node:fs'

const [out, cols, ...files] = process.argv.slice(2)
const imgs = files
  .map((f) => `<figure><img src="data:image/png;base64,${readFileSync(f).toString('base64')}"><figcaption>${f.split('/').pop()}</figcaption></figure>`)
  .join('')
const html = `<html><body style="margin:0;background:#888;font:11px system-ui">
<div style="display:grid;grid-template-columns:repeat(${cols},552px);gap:6px;padding:6px">${imgs}</div>
<style>figure{margin:0}img{width:552px;display:block}figcaption{padding:2px 0;color:#fff}</style></body></html>`
const browser = await chromium.launch({ channel: 'chrome' })
const page = await browser.newPage({ viewport: { width: 6 + cols * 558, height: 400 } })
await page.setContent(html)
await page.waitForTimeout(200)
await page.screenshot({ path: out, fullPage: true })
await browser.close()
