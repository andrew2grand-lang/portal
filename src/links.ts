/* Where SeamDrop lives on the web, and where feedback goes: one place
   (Mission 04 §5). We don't own seamdrop.com: somebody else registered it.
   Nothing opens the site or starts an email to it until `confirmed` is true
   (The owner switches it; see "Needs the owner" in docs/PROGRESS.md). */
import { APP_NAME } from './platform'

export const LINKS = {
  website: 'https://seamdrop.com',
  feedback: 'feedback@seamdrop.com',
  confirmed: false,
} as const

/** The site to open, or null while it isn't confirmed */
export function websiteLink(confirmed: boolean = LINKS.confirmed): string | null {
  return confirmed ? LINKS.website : null
}

/** A short feedback email (the version and the OS; a mailto link can't
    attach the log, and long bodies get cut off), or null while the address
    isn't confirmed */
export function feedbackMail(
  about: { version: string; os: string; withLog: boolean },
  confirmed: boolean = LINKS.confirmed,
): string | null {
  if (!confirmed) return null
  const body = [
    `${APP_NAME} ${about.version} · ${about.os}`,
    '',
    about.withLog ? 'The technical log is attached (SeamDrop saved it in Downloads).' : '',
    '',
    'What happened:',
  ].join('\n')
  const q = new URLSearchParams({ subject: `${APP_NAME} feedback`, body })
  return `mailto:${LINKS.feedback}?${q.toString().replaceAll('+', '%20')}`
}
