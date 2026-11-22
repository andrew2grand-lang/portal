// Runs a script against the app's dev server (started here, stopped after).
// Usage: node scripts/with-dev-server.mjs scripts/glow-trace.mjs [args…]
// The script gets PORTAL_URL. Used by CI (no curl, works on every OS).
import { spawn } from 'node:child_process'
import { createServer } from 'vite'

const [script, ...args] = process.argv.slice(2)
if (!script) {
  console.error('usage: node scripts/with-dev-server.mjs <script> [args…]')
  process.exit(2)
}
const server = await createServer({ server: { port: 5180, strictPort: false }, logLevel: 'warn' })
await server.listen()
const url = server.resolvedUrls.local[0]
console.log(`dev server at ${url}`)
const child = spawn(process.execPath, [script, ...args], { stdio: 'inherit', env: { ...process.env, PORTAL_URL: url } })
const code = await new Promise((resolve) => child.on('exit', (c) => resolve(c ?? 1)))
await server.close()
process.exit(code)
