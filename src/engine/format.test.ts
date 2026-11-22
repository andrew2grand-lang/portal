import { describe, expect, it } from 'vitest'
import { doneTiming, etaWords, formatSpeed } from './format'

describe('speed and time left (the core decides; these only word them)', () => {
  it('speeds keep their 2 significant figures', () => {
    expect(formatSpeed(4_500_000)).toBe('4.5 MB/s')
    expect(formatSpeed(45_000_000)).toBe('45 MB/s')
    expect(formatSpeed(120_000_000)).toBe('120 MB/s')
    expect(formatSpeed(980_000)).toBe('980 KB/s')
    expect(formatSpeed(1_100_000_000)).toBe('1.1 GB/s')
  })
  it('time left in plain words', () => {
    expect(etaWords({ kind: 'working_out' })).toBe('Working out time left…')
    expect(etaWords({ kind: 'minutes', n: 6 })).toBe('About 6 minutes')
    expect(etaWords({ kind: 'minutes_and_a_half', n: 2 })).toBe('About 2½ minutes')
    expect(etaWords({ kind: 'minute_and_a_half' })).toBe('About a minute and a half')
    expect(etaWords({ kind: 'checking' })).toBe('Checking what already arrived…')
    expect(etaWords({ kind: 'hidden' })).toBe('')
    expect(etaWords({ kind: 'seconds', n: 40 })).toBe('About 40 seconds')
    expect(etaWords({ kind: 'few_seconds' })).toBe('A few seconds')
  })
})

describe('the finished line (Review 04: the speed counts only what moved this time)', () => {
  it('says size, time and the average speed', () => {
    expect(doneTiming(633_000_000, 0, 7.4, 'sent')).toBe('633 MB in 7.4 s (86 MB/s)')
  })
  it('leaves bytes that were already there out of the speed', () => {
    expect(doneTiming(633_000_000, 280_000_000, 3, 'sent')).toBe('633 MB: 353 MB sent in 3.0 s (118 MB/s)')
    expect(doneTiming(633_000_000, 280_000_000, 3, 'arrived')).toBe('633 MB: 353 MB arrived in 3.0 s (118 MB/s)')
  })
  it('without a time, only the size', () => {
    expect(doneTiming(633_000_000, 0, undefined, 'sent')).toBe('633 MB')
  })
})
