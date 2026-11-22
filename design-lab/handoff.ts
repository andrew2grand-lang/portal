import { APP_NAME } from '../src/platform'
import { formatBytes } from '../src/engine/format'
import type { Report } from './report'
import { groupFindings, osName, place, stateName, stateOrder } from './report'

/** The hand-off note: the scanner's own (native apps, summary-only code), or
    one written from the findings, so anyone can pick the move up by hand. */
export function handoffMarkdown(r: Report): string {
  if (r.handoff) return r.handoff
  const target = osName[r.target]
  const lines = [
    `# Moving ${r.project.name} to ${target}`,
    '',
    `${r.headline} ${r.tier.reason}.`,
    '',
    `Written by ${APP_NAME}’s scanner, which read the project without running anything.`,
    '',
  ]
  for (const state of stateOrder) {
    const groups = groupFindings(r.findings, state)
    if (!groups.length || state === 'info') continue
    lines.push(`## ${stateName[state].group}`, '')
    for (const g of groups) {
      lines.push(`- **${g.first.title}.** ${g.first.why}`)
      if (state !== 'couldntCheck') lines.push(`  Fix: ${g.first.fixPattern}`)
      for (const p of g.places.slice(0, 6)) {
        const at = place(p)
        if (at) lines.push(`  - \`${at}\`${p.evidence ? `: \`${p.evidence}\`` : ''}`)
      }
    }
    lines.push('')
  }
  if (r.leftOut.items.length) {
    lines.push(`## Not included (${formatBytes(r.leftOut.bytes)})`, '')
    for (const i of r.leftOut.items) lines.push(`- \`${i.path}\` ${formatBytes(i.bytes)}${i.rebuildable ? ' (rebuild it there)' : ''}`)
    lines.push('')
  }
  if (r.secrets.length) {
    lines.push('## Secrets to add by hand', '')
    for (const sf of r.secrets) lines.push(`- \`${sf.path}\`: ${sf.keys.map((k) => `\`${k}\``).join(', ')}`)
    lines.push('')
  }
  return lines.join('\n')
}

export function download(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: 'text/markdown' }))
  const a = document.createElement('a')
  a.href = url
  a.download = name
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}
