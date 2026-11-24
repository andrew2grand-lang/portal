import { describe, expect, it } from 'vitest'
import type { UpdateInfo } from './types'
import { noticeFor, restartLabel, updateLine } from './updates'

const base: UpdateInfo = { state: 'idle', current: '0.3.0', dismissed: false, requested: false, auto: true, testBuilds: false }
const u = (over: Partial<UpdateInfo>): UpdateInfo => ({ ...base, ...over })

describe('the update notice', () => {
  it('offers a ready update calmly, with what is new', () => {
    const n = noticeFor(u({ state: 'ready', version: '0.4.0', notes: 'Faster resume.' }))
    expect(n).toEqual({ title: 'SeamDrop 0.4.0 is ready', sub: 'What’s new: Faster resume.', keys: 'offer' })
  })
  it('shows only the first paragraph of long notes (the notice never clamps text)', () => {
    const n = noticeFor(u({ state: 'ready', version: '0.4.0', notes: 'Faster resume.\n\n- Fixed a crash\n- Better icons' }))
    expect(n?.sub).toBe('What’s new: Faster resume.')
  })
  it('stays away once Later was pressed for that version', () => {
    expect(noticeFor(u({ state: 'ready', version: '0.4.0', dismissed: true }))).toBeNull()
  })
  it('says it waits for a running transfer instead of restarting', () => {
    const n = noticeFor(u({ state: 'waiting', version: '0.4.0', peer: 'Desk PC' }))
    expect(n?.title).toBe('SeamDrop restarts to update when the transfer finishes')
    expect(n?.sub).toContain('Desk PC')
  })
  // Review 04 (developer): a paused 40 GB transfer isn't lost to a restart, and the notice says so
  it('says a paused transfer continues after the restart', () => {
    const offer = noticeFor(u({ state: 'ready', version: '0.4.0' }), ['Desk PC'])
    expect(offer?.sub).toBe('The paused transfer with Desk PC continues after the restart.')
    const now = noticeFor(u({ state: 'installing', version: '0.4.0' }), ['Desk PC', 'Old Laptop'])
    expect(now?.sub).toBe('SeamDrop restarts in a moment. The paused transfers with Desk PC and Old Laptop continue after it.')
  })
  it('keeps background checks and downloads quiet', () => {
    for (const state of ['idle', 'checking', 'upToDate'] as const) expect(noticeFor(u({ state }))).toBeNull()
    expect(noticeFor(u({ state: 'downloading', version: '0.4.0', percent: 40 }))).toBeNull()
    expect(noticeFor(u({ state: 'failed', message: 'Couldn’t reach the update server.' }))).toBeNull()
  })
  it('shows a download and a failure the person asked for', () => {
    expect(noticeFor(u({ state: 'downloading', version: '0.4.0', percent: 40, requested: true }))?.title).toBe(
      'Downloading SeamDrop 0.4.0… 40%',
    )
    const failed = noticeFor(u({ state: 'failed', message: 'The update didn’t pass its signature check.', requested: true }))
    // A next step, not just Later: try again, or OK to leave it
    expect(failed).toEqual({
      title: 'The update didn’t install',
      sub: 'The update didn’t pass its signature check.',
      keys: 'failed',
    })
  })
})

describe('Settings → About', () => {
  it('says where updates stand in one sentence', () => {
    expect(updateLine(u({}))).toBe('Checks for updates by itself every few hours.')
    expect(updateLine(u({ state: 'checking' }))).toBe('Checking for updates…')
    expect(updateLine(u({ state: 'ready', version: '0.4.0' }))).toBe('SeamDrop 0.4.0 is ready. Restart SeamDrop to install it.')
    expect(updateLine(u({ state: 'failed', message: 'Couldn’t reach the update server.' }))).toBe(
      'Couldn’t reach the update server.',
    )
    expect(updateLine(u({ state: 'upToDate', checkedAt: new Date(2026, 10, 2, 9, 5).getTime() }))).toBe(
      'Up to date. Checked at 09:05.',
    )
  })
})

describe('the restart key', () => {
  it('names the app, so nobody thinks the whole computer restarts', () => {
    expect(restartLabel(u({ state: 'ready', version: '0.4.0' }))).toBe('Restart SeamDrop to update')
    expect(restartLabel(u({ state: 'available', version: '0.4.0' }))).toBe('Update and restart SeamDrop')
  })
})

describe('About when nothing is published yet (beta.2)', () => {
  it('says "No updates yet", never that the information couldn’t be read', () => {
    expect(updateLine(u({ state: 'nothingYet' }))).toBe('No updates yet.')
    expect(noticeFor(u({ state: 'nothingYet' }))).toBeNull()
  })
  it('with test builds on and nothing newer: on the latest test build', () => {
    expect(updateLine(u({ state: 'upToDate', testBuilds: true }))).toBe('You’re on the latest test build.')
    expect(updateLine(u({ state: 'upToDate', testBuilds: false }))).toBe('Up to date.')
  })
  it('a newer beta is offered as any update is', () => {
    expect(updateLine(u({ state: 'available', version: '0.5.0-beta.2', testBuilds: true }))).toContain('0.5.0-beta.2 is available')
  })
})
