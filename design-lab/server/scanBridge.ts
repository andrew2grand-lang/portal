/* The lab's bridge to the real scanner: a dev-server plugin (never shipped).
     GET /api/scan?path=…&target=windows|macos[&online=1]
       Server-sent events: `progress` (one per `portal-scan --progress` line),
       then `report` (the JSON report) or `failed` ({ message }).
     GET /api/browse?path=…
       The sub-folders of a folder, for the lab's folder picker. Names only.
   Requests from other sites are refused, so only the lab page can use it. */
import { spawn } from 'node:child_process'
import { existsSync, readdirSync, statSync } from 'node:fs'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { homedir } from 'node:os'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import type { Plugin } from 'vite'

const repo = resolve(import.meta.dirname, '../..')
const exe = process.platform === 'win32' ? '.exe' : ''

function scannerPath(): string | null {
  for (const profile of ['release', 'debug']) {
    const p = join(repo, 'target', profile, `portal-scan${exe}`)
    if (existsSync(p)) return p
  }
  return null
}

/** `~/x` and absolute paths only; anything else isn't a folder we can name */
export function folderFrom(raw: string | null): string | null {
  // Network paths (\\server\share, //server/share) would reach out to another machine
  if (!raw || /^[\\/]{2}/.test(raw)) return null
  const path = raw.startsWith('~') ? join(homedir(), raw.slice(1)) : raw
  if (!isAbsolute(path)) return null
  try {
    return statSync(path).isDirectory() ? resolve(path) : null
  } catch {
    return null
  }
}

/** Browsers send Sec-Fetch-Site (and Origin): anything but our own page is turned away */
export function sameOrigin(req: Pick<IncomingMessage, 'headers'>): boolean {
  const site = req.headers['sec-fetch-site']
  if (site !== undefined && site !== 'same-origin' && site !== 'none') return false
  const origin = req.headers.origin
  if (origin === undefined) return true
  try {
    return new URL(origin).host === req.headers.host
  } catch {
    return false
  }
}

function scan(url: URL, req: IncomingMessage, res: ServerResponse) {
  res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-store', connection: 'keep-alive' })
  const send = (event: string, data: unknown) => res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
  const fail = (message: string) => {
    send('failed', { message })
    res.end()
  }

  const folder = folderFrom(url.searchParams.get('path'))
  if (!folder) return fail("That isn't a folder on this computer. Type a full path, like ~/Projects/app.")
  const scanner = scannerPath()
  if (!scanner) return fail('The scanner isn’t built yet. Run: cargo build --release -p portal-scan')
  const target = url.searchParams.get('target') === 'macos' ? 'macos' : 'windows'
  const args = [folder, '--json', '--progress', '--target', target]
  if (url.searchParams.get('online') === '1') args.push('--online')

  const child = spawn(scanner, args, { cwd: homedir(), stdio: ['ignore', 'pipe', 'pipe'] })
  let out = ''
  let errText = ''
  let pending = ''
  child.stdout.setEncoding('utf8').on('data', (chunk: string) => (out += chunk))
  child.stderr.setEncoding('utf8').on('data', (chunk: string) => {
    pending += chunk
    const lines = pending.split('\n')
    pending = lines.pop() ?? ''
    for (const line of lines) {
      try {
        send('progress', JSON.parse(line))
      } catch {
        errText += line + '\n'
      }
    }
  })
  child.on('error', (e) => fail(`The scanner couldn't start: ${e.message}`))
  child.on('close', (code) => {
    if (res.writableEnded) return
    if (code !== 0) return fail(errText.trim() || `The scanner stopped (exit ${code}).`)
    try {
      send('report', JSON.parse(out))
      res.end()
    } catch {
      fail('The scanner’s report couldn’t be read.')
    }
  })
  req.on('close', () => child.kill())
}

function browse(url: URL, res: ServerResponse) {
  const folder = folderFrom(url.searchParams.get('path') ?? '~') ?? homedir()
  let entries: { name: string; path: string }[] = []
  try {
    entries = readdirSync(folder, { withFileTypes: true })
      .filter((d) => d.isDirectory() && !d.name.startsWith('.'))
      .map((d) => ({ name: d.name, path: join(folder, d.name) }))
      .sort((a, b) => a.name.localeCompare(b.name))
      .slice(0, 400)
  } catch {
    /* unreadable: show it empty */
  }
  const parent = dirname(folder)
  res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' })
  res.end(JSON.stringify({ path: folder, parent: parent === folder ? null : parent, home: homedir(), entries }))
}

export function scanBridge(): Plugin {
  return {
    name: 'portal-scan-bridge',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (!req.url?.startsWith('/api/')) return next()
        if (!sameOrigin(req) || req.method !== 'GET') {
          res.writeHead(403).end()
          return
        }
        const url = new URL(req.url, 'http://localhost')
        if (url.pathname === '/api/scan') return scan(url, req, res)
        if (url.pathname === '/api/browse') return browse(url, res)
        res.writeHead(404).end()
      })
    },
  }
}
