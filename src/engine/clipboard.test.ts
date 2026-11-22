import { describe, expect, it } from 'vitest'
import { BROWSER_NOTE, PRIVATE_RULE, SYNC_RULE, syncLine } from './clipboard'

const c = (mine: boolean, theirs: boolean, asks = false) => ({ mine, theirs, asks, to: 'clipboard' as const })

describe('clipboard sync wording, per device', () => {
  it('says it runs only once both agreed', () => {
    expect(syncLine(c(true, true), 'Desk PC', false)).toMatch(/^On: /)
    expect(syncLine(c(true, false), 'Desk PC', false)).toBe('On here. Nothing syncs until Desk PC turns it on too.')
    expect(syncLine(c(false, true, true), 'Desk PC', false)).toMatch(/asks to sync/)
    expect(syncLine(c(false, false), 'Desk PC', false)).toBe('Off.')
  })
  it('says the rule once, not under every device (Review 04)', () => {
    expect(SYNC_RULE).toMatch(/both/i)
    expect(syncLine(c(false, false), 'Desk PC', false)).not.toMatch(/both/i)
  })
  it('says which browsers mark private windows, as measured (Review 04)', () => {
    expect(BROWSER_NOTE).toMatch(/private window of Edge, Chrome or Firefox/)
    expect(BROWSER_NOTE).toMatch(/may not/)
  })
  it('says a pause is local and stops both ways', () => {
    expect(syncLine(c(true, true), 'Desk PC', true)).toMatch(/paused here/)
  })
  it('names what is never sent without guessing', () => {
    expect(PRIVATE_RULE).toMatch(/marks as private/)
    expect(PRIVATE_RULE).not.toMatch(/detect/i)
  })
})
