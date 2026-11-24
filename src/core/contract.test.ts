/* The UI ↔ Rust contract (beta.2): every command, called the way the
   window calls it, records the exact JSON `invoke` would send. The Rust side
   (src-tauri/tests/contract.rs) puts each through the argument types the
   command declares, with Tauri's own rule for missing keys. A partial
   Settings patch that Rust can't read (beta.1: every patch without
   clearReceiveNote) fails there, not silently in the app.

   The examples are typed against the real functions, so a new command or a
   new Settings field without an example doesn't compile. `UPDATE_CONTRACT=1
   npx vitest run src/core/contract.test.ts` rewrites the fixture. */
import { readFileSync, writeFileSync } from 'node:fs'
import { describe, expect, it, vi } from 'vitest'

const calls: { cmd: string; args: unknown }[] = []
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: unknown) => {
    calls.push({ cmd, args: args ?? {} })
    return Promise.resolve(null)
  },
}))

const { core } = await import('./bridge')
type Core = typeof core
type SettingsPatch = Parameters<Core['saveSettings']>[0]

const ID = 'a'.repeat(32)
const TID = '0000000000a1b2c3'

/** One partial patch per field, as Settings, the tray and the menus send them */
const oneField: { [K in keyof Required<SettingsPatch>]: SettingsPatch[K][] } = {
  deviceName: ['Desk PC'],
  receiveDir: ['/Users/sam/Downloads'],
  skipJunk: [true, false],
  autoUpdate: [true, false],
  testBuilds: [true, false],
  clipboardSync: [true, false],
  sendTo: [[ID], []],
  appearance: ['system', 'light', 'dark'],
  folderPerDevice: [true, false],
  receiveMode: ['loose', 'app_folder', 'numbered'],
  keepDates: [true, false],
  clearReceiveNote: [true],
}
const patches: SettingsPatch[] = [
  {},
  ...Object.entries(oneField).flatMap(([k, vs]) => (vs as unknown[]).map((v) => ({ [k]: v }) as SettingsPatch)),
  // Several at once, as some call sites do
  { receiveMode: 'app_folder', folderPerDevice: true },
]

/** Every command, with the arguments the window passes */
const examples: { [K in keyof Core]: Parameters<Core[K]>[] } = {
  snapshot: [[]],
  logs: [[]],
  connect: [['192.168.1.20']],
  retry: [[]],
  pair: [[ID]],
  pairing: [
    [{ type: 'showCode' }],
    [{ type: 'ignore' }],
    [{ type: 'code', code: '048213' }],
    [{ type: 'cancel' }],
    [{ type: 'dismiss' }],
    [{ type: 'allowAgain' }],
  ],
  removePeer: [[ID]],
  renamePeer: [[ID, 'Desk PC']],
  setIncoming: [[ID, 'ask'], [ID, 'always'], [ID, 'never']],
  setConnected: [[ID, true], [ID, false]],
  saveSettings: patches.map((p) => [p]),
  send: [[[ID], ['/Users/sam/Desktop/Site photos', '/Users/sam/Desktop/plan.pdf']]],
  describe: [[['/Users/sam/Desktop/plan.pdf']]],
  accept: [[TID, false], [TID, true]],
  decline: [[TID]],
  cancel: [[TID]],
  continueTransfer: [[TID]],
  openFile: [['/Users/sam/Downloads/plan.pdf']],
  sendAgain: [[TID]],
  fixFirewall: [[]],
  dismiss: [[TID]],
  updateStatus: [[]],
  checkUpdates: [[]],
  restartToUpdate: [[]],
  updateLater: [[]],
  sendText: [[[ID], 'hello', false], [undefined, 'sk_test_123', true]],
  sendClipboard: [[[ID]], []],
  setClipSync: [[ID, true], [ID, false]],
  clipSyncOff: [[]],
  pauseClipSync: [[60], []],
  resumeClipSync: [[]],
  setClipTo: [[ID, 'clipboard'], [ID, 'shelf']],
  copyClip: [[1]],
  copyTechnicalLog: [[]],
  openReceived: [[]],
  saveTechnicalLog: [[]],
  clearClips: [[]],
  removeClip: [[1]],
  shelfAdd: [['Wi-Fi password', 'correct horse', true], ['', 'text', false]],
  shelfEdit: [[ID, 'New name', null], [ID, null, 'new text']],
  shelfRemove: [[[ID]], [null]],
  shelfUndo: [[]],
  shelfCopy: [[ID]],
  shelfReveal: [[ID]],
  setShelfShare: [[ID, true], [ID, false]],
  setPicture: [
    [{ kind: 'mark', color: 0x4f6bed, icon: 'laptop' }],
    [{ kind: 'photo', png: 'iVBORw0KGgo=' }],
    [null],
  ],
  shelfPutMarked: [[3]],
  shelfPutReceived: [[1]],
  saveClipImage: [[1, 'Clipboard image 2026-11-24 10.00.00']],
  dismissClipNotice: [[]],
  unqueueNext: [[1]],
  continueNext: [[ID]],
  shortcut: [[]],
  setShortcut: [['CommandOrControl+Alt+Shift+C']],
}

const FIXTURE = 'src-tauri/tests/fixtures/ui-invokes.json'

describe('the UI ↔ Rust contract', () => {
  it('records the exact JSON of every command the window sends', async () => {
    for (const [name, list] of Object.entries(examples) as [keyof Core, unknown[][]][]) {
      for (const args of list) await (core[name] as (...a: unknown[]) => Promise<unknown>)(...args)
    }
    const recorded = JSON.stringify(calls, null, 1) + '\n'
    if (process.env.UPDATE_CONTRACT) writeFileSync(FIXTURE, recorded)
    const saved = readFileSync(FIXTURE, 'utf8')
    expect(saved, `${FIXTURE} is out of date: UPDATE_CONTRACT=1 npx vitest run src/core/contract.test.ts`).toBe(recorded)
  })

  it('covers every command and every Settings field', () => {
    expect(Object.keys(examples).sort()).toEqual(Object.keys(core).sort())
    for (const k of Object.keys(oneField)) expect(patches.some((p) => Object.keys(p).length === 1 && k in p), k).toBe(true)
  })
})
