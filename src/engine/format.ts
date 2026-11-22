import type { CoreEta } from '../core/bridge'
import type { Link } from './types'

export function formatBytes(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(2)} GB`
  if (bytes >= 1e8) return `${Math.round(bytes / 1e6)} MB`
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`
  if (bytes >= 1e3) return `${Math.round(bytes / 1e3)} KB`
  return `${bytes} B`
}

export function formatRate(bytesPerSecond: number): string {
  return `${Math.max(0, Math.round(bytesPerSecond / 1e6))} MB/s`
}

/** A speed the core already rounded to 2 significant figures: "4.5 MB/s", "120 MB/s", "980 KB/s" */
export function formatSpeed(bytesPerSecond: number): string {
  const units: [number, string][] = [
    [1e9, 'GB/s'],
    [1e6, 'MB/s'],
    [1e3, 'KB/s'],
  ]
  for (const [size, unit] of units) {
    if (bytesPerSecond >= size) {
      const v = bytesPerSecond / size
      return `${v >= 10 ? Math.round(v) : Number(v.toPrecision(2))} ${unit}`
    }
  }
  return `${Math.round(bytesPerSecond)} B/s`
}

/** The core's time left, in words */
export function etaWords(eta: CoreEta): string {
  switch (eta.kind) {
    case 'working_out':
      return 'Working out time left…'
    case 'checking':
      return 'Checking what already arrived…'
    case 'hidden':
      return ''
    case 'minutes':
      return `About ${eta.n} minutes`
    case 'minutes_and_a_half':
      return `About ${eta.n}½ minutes`
    case 'minute_and_a_half':
      return 'About a minute and a half'
    case 'about_a_minute':
      return 'About a minute'
    case 'seconds':
      return `About ${eta.n} seconds`
    case 'few_seconds':
      return 'A few seconds'
  }
}

export function formatEta(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return 'Finishing'
  if (seconds < 1.5) return 'About a second left'
  if (seconds < 60) return `About ${Math.round(seconds)} s left`
  return `About ${Math.round(seconds / 60)} min left`
}

export function formatSeconds(seconds: number): string {
  return seconds < 10 ? `${seconds.toFixed(1)} s` : `${Math.round(seconds)} s`
}

/** "633 MB in 7.4 s (86 MB/s)"; after a pause, only what moved this time
    counts: "633 MB: 353 MB sent in 3.0 s (118 MB/s)" */
export function doneTiming(total: number, kept: number, seconds: number | undefined, verb: 'sent' | 'arrived'): string {
  if (!seconds) return formatBytes(total)
  const moved = Math.max(0, total - kept)
  const took = `in ${formatSeconds(seconds)} (${formatRate(moved / Math.max(seconds, 0.001))})`
  return kept > 0 ? `${formatBytes(total)}: ${formatBytes(moved)} ${verb} ${took}` : `${formatBytes(total)} ${took}`
}

export function clockTime(date = new Date()): string {
  return date.toLocaleTimeString('en-GB', { hour12: false })
}

export function plural(n: number, one: string, many = `${one}s`) {
  return `${n} ${n === 1 ? one : many}`
}


/** The link pill's words. A direct wired link is just Ethernet to people. */
export const linkLabel: Record<Link, string> = {
  wifi: 'Wi-Fi',
  cable: 'Ethernet',
  ethernet: 'Ethernet',
  other: 'Network',
}

/** "…over Wi-Fi" */
export const overLink: Record<Link, string> = {
  wifi: 'over Wi-Fi',
  cable: 'over Ethernet',
  ethernet: 'over Ethernet',
  other: 'over the network',
}
