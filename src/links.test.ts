import { describe, expect, it } from 'vitest'
import { LINKS, feedbackMail, websiteLink } from './links'

describe('links (Mission 04 §5)', () => {
  it('keeps the website and the feedback address in one place, not confirmed yet', () => {
    expect(LINKS.website).toBe('https://seamdrop.com')
    expect(LINKS.feedback).toBe('feedback@seamdrop.com')
    expect(LINKS.confirmed).toBe(false)
  })
  it('opens nothing and mails nothing until confirmed', () => {
    expect(websiteLink(false)).toBeNull()
    expect(feedbackMail({ version: '0.4.0', os: 'macOS 15.1 (24B83)', withLog: true }, false)).toBeNull()
  })
  it('once confirmed: a short mail with the version and the OS', () => {
    expect(websiteLink(true)).toBe('https://seamdrop.com')
    const url = feedbackMail({ version: '0.4.0', os: 'Windows 11 24H2 (build 26100)', withLog: false }, true)!
    expect(url.startsWith('mailto:feedback@seamdrop.com?')).toBe(true)
    const q = new URLSearchParams(url.split('?')[1])
    expect(q.get('subject')).toBe('SeamDrop feedback')
    expect(q.get('body')).toContain('SeamDrop 0.4.0 · Windows 11 24H2 (build 26100)')
    expect(q.get('body')!.length).toBeLessThan(300)
    const withLog = feedbackMail({ version: '0.4.0', os: 'macOS', withLog: true }, true)!
    expect(new URLSearchParams(withLog.split('?')[1]).get('body')).toContain('technical log')
  })
})
