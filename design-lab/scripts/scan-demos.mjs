// Builds the lab's demo reports with the real scanner: each project in
// design-lab/projects is materialised into a temp folder (big files are
// sparse) and scanned. Run: npm run lab:demos
import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const repo = resolve(import.meta.dirname, '../..')
const exe = process.platform === 'win32' ? '.exe' : ''

/** Where each demo project lives on the sending computer, and where it goes */
const demos = [
  { name: 'local-chat', target: 'windows', source: 'macos', path: '~/Projects/local-chat' },
  { name: 'shop-web', target: 'windows', source: 'macos', path: '~/Code/shop-web' },
  { name: 'ledger-tool', target: 'macos', source: 'windows', path: 'C:\\Users\\sam\\Projects\\ledger-tool' },
  { name: 'Sketchbook', target: 'windows', source: 'macos', path: '~/Developer/Sketchbook' },
  { name: 'InvoiceDesk', target: 'macos', source: 'windows', path: 'C:\\Users\\sam\\source\\repos\\InvoiceDesk' },
  { name: 'Handbook', target: 'windows', source: 'macos', path: '~/Documents/Handbook' },
  { name: 'Empty folder', target: 'windows', source: 'macos', path: '~/Desktop/Empty folder' },
]

execFileSync('cargo', ['build', '-q', '--release', '-p', 'portal-scan', '--bin', 'portal-scan', '--example', 'materialize'], {
  cwd: repo,
  stdio: 'inherit',
})
const scanner = join(repo, 'target/release/portal-scan' + exe)
const materialize = join(repo, 'target/release/examples/materialize' + exe)

for (const demo of demos) {
  const tmp = mkdtempSync(join(tmpdir(), 'portal-demo-'))
  const dir = join(tmp, demo.name)
  try {
    execFileSync(materialize, [join(repo, 'design-lab/projects', demo.name), dir])
    const report = JSON.parse(execFileSync(scanner, [dir, '--json', '--target', demo.target], { encoding: 'utf8' }))
    // The scan ran here; the demo says where the project lives in the story
    report.project.path = demo.path
    report.source = demo.source
    report.scannedAt = '2026-10-28T09:41:00Z'
    writeFileSync(join(repo, 'design-lab/data', `${demo.name}.json`), JSON.stringify(report, null, 2) + '\n')
    const c = report.counts
    console.log(`${demo.name}: ${report.headline}  [${c.blocker}/${c.willNeedAFix}/${c.needsYou}/${c.couldntCheck}/${c.info}]`)
  } finally {
    rmSync(tmp, { recursive: true, force: true })
  }
}
