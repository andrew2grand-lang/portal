/* The window's settings live in three files: tauri.conf.json and the macOS
   and Windows ones, whose window list replaces the base one whole. beta.2
   found two things set only in the base: "create": false (the setup hook
   builds the window, so it was made twice and the app quit) and the 300 × 360
   minimum (mini mode was out of reach). These must agree everywhere. */
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const read = (f: string) => JSON.parse(readFileSync(`src-tauri/${f}`, 'utf8')) as { app?: { windows?: Record<string, unknown>[] } }
const main = (f: string) => read(f).app?.windows?.find((w) => w.label === 'main')

describe('the main window, on every system', () => {
  const base = main('tauri.conf.json')
  for (const f of ['tauri.macos.conf.json', 'tauri.windows.conf.json']) {
    it(`${f} keeps what the base sets`, () => {
      const w = main(f)
      expect(w, `${f} has no main window`).toBeDefined()
      expect(w?.create).toBe(false)
      expect([w?.minWidth, w?.minHeight]).toEqual([base?.minWidth, base?.minHeight])
      expect([w?.width, w?.height]).toEqual([base?.width, base?.height])
    })
  }
  it('is built by the setup hook (create: false), at least 300 × 360', () => {
    expect(base?.create).toBe(false)
    expect([base?.minWidth, base?.minHeight]).toEqual([300, 360])
  })
})
