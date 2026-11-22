// CI minutes this mission used (Mission 04 §0: report minutes used).
// Sums every job's wall time since a date, by runner OS, and what GitHub
// bills for a private repo (Linux ×1, Windows ×2, macOS ×10, each job
// rounded up to a whole minute).
//
// Usage: node scripts/ci-minutes.mjs [since=2026-11-14] (needs `gh` logged in)
import { execFileSync } from 'node:child_process'

const since = process.argv[2] ?? '2026-11-14'
const gh = (path) => JSON.parse(execFileSync('gh', ['api', '--paginate', '--slurp', path], { encoding: 'utf8', maxBuffer: 64 << 20 }))
const repo = execFileSync('gh', ['repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner'], { encoding: 'utf8' }).trim()

const runs = gh(`repos/${repo}/actions/runs?created=>=${since}&per_page=100`).flatMap((p) => p.workflow_runs)
const totals = { linux: { jobs: 0, minutes: 0 }, windows: { jobs: 0, minutes: 0 }, macos: { jobs: 0, minutes: 0 } }
const byWorkflow = {}
for (const run of runs) {
  const jobs = gh(`repos/${repo}/actions/runs/${run.id}/jobs?per_page=100`).flatMap((p) => p.jobs)
  for (const job of jobs) {
    if (!job.started_at || !job.completed_at) continue
    const labels = (job.labels ?? []).join(' ')
    const os = /macos/i.test(labels) ? 'macos' : /windows/i.test(labels) ? 'windows' : 'linux'
    const minutes = Math.ceil((new Date(job.completed_at) - new Date(job.started_at)) / 60_000)
    totals[os].jobs++
    totals[os].minutes += minutes
    const key = `${run.name} (${run.head_branch})`
    byWorkflow[key] = (byWorkflow[key] ?? 0) + minutes * { linux: 1, windows: 2, macos: 10 }[os]
  }
}
const billed = totals.linux.minutes + totals.windows.minutes * 2 + totals.macos.minutes * 10
console.log(`CI since ${since}: ${runs.length} runs`)
for (const [os, t] of Object.entries(totals)) console.log(`  ${os.padEnd(8)} ${String(t.jobs).padStart(4)} jobs  ${String(t.minutes).padStart(5)} min`)
console.log(`  billed (Linux ×1, Windows ×2, macOS ×10): ${billed} min`)
console.log('By workflow (billed):')
for (const [k, v] of Object.entries(byWorkflow).sort((a, b) => b[1] - a[1])) console.log(`  ${String(v).padStart(5)}  ${k}`)
