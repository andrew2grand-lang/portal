/* Shared words for the window: the doctor's fallback checks and how a
   delivery is named. */
import { APP_NAME } from '../platform'
import type { DoctorCheck, Payload } from './types'

/** "Site photos and 2 more", "report.pdf" */
export function payloadTitle(p: Payload): string {
  const [first, ...rest] = p.items
  if (!first) return 'Nothing'
  return rest.length ? `${first.name} and ${rest.length} more` : first.name
}

/** "Desk PC", "Desk PC and MacBook Air", "Desk PC, MacBook Air and Studio" */
export function names(list: string[]): string {
  if (list.length <= 1) return list[0] ?? ''
  return `${list.slice(0, -1).join(', ')} and ${list.at(-1)}`
}

/** What the doctor says when nothing has been found yet */
export const genericDoctor: DoctorCheck[] = [
  {
    id: 'peer-open',
    status: 'unknown',
    title: `${APP_NAME} is open on the other computer`,
    fix: `Open ${APP_NAME} there too. It keeps running from the menu bar or system tray.`,
  },
  {
    id: 'same-network',
    status: 'unknown',
    title: 'Both are on the same network',
    fix: 'Put both on the same home network: its Wi-Fi, or plugged into its router.',
  },
  {
    id: 'isolation',
    status: 'likely',
    title: 'Your network may keep devices apart',
    fix: 'Guest, school and office Wi-Fi often do. Use your home network, or Connect by IP.',
  },
]
