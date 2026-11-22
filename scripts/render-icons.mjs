// Renders src-tauri/icons/*.svg to PNG with the installed Chrome.
// Usage: node scripts/render-icons.mjs  (then: npx tauri icon src-tauri/icons/app-icon.png -o src-tauri/icons)
import { chromium } from 'playwright'
import { readFileSync } from 'node:fs'

const jobs = [
  ['src-tauri/icons/app-icon.svg', 'src-tauri/icons/app-icon.png', 1024],
  // Menu bar (template images: macOS tints them) and Windows tray
  ['src-tauri/icons/tray/mac.svg', 'src-tauri/icons/tray/mac.png', 44],
  ['src-tauri/icons/tray/mac-alert.svg', 'src-tauri/icons/tray/mac-alert.png', 44],
  ['src-tauri/icons/tray/win.svg', 'src-tauri/icons/tray/win.png', 32],
  ['src-tauri/icons/tray/win-alert.svg', 'src-tauri/icons/tray/win-alert.png', 32],
]

const browser = await chromium.launch({ channel: 'chrome' })
for (const [svgPath, outPath, size] of jobs) {
  const svg = readFileSync(svgPath, 'utf8').replace(/width="\d+" height="\d+"/, `width="${size}" height="${size}"`)
  const page = await browser.newPage({ viewport: { width: size, height: size } })
  await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`)
  await page.screenshot({ path: outPath, omitBackground: true, clip: { x: 0, y: 0, width: size, height: size } })
  await page.close()
  console.log('rendered', outPath)
}
await browser.close()
