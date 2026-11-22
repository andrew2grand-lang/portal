import { useCallback, useEffect, useRef, useState } from 'react'
import type { Report, ScanProgress, TargetOs } from './report'
import type { RealScan } from './screens/types'

const idle: RealScan = {
  status: 'idle',
  path: '',
  target: 'windows',
  online: false,
  steps: {},
  last: null,
  report: null,
  error: null,
  startedAt: 0,
  finishedAt: null,
}

/** Runs the real scanner through the lab's dev-server bridge (server-sent events) */
export function useRealScan() {
  const [real, setReal] = useState<RealScan>(idle)
  const source = useRef<EventSource | null>(null)

  const start = useCallback((path: string, target: TargetOs, online: boolean) => {
    source.current?.close()
    setReal({ ...idle, status: 'running', path, target, online, startedAt: performance.now() })
    const q = new URLSearchParams({ path, target, online: online ? '1' : '0' })
    const es = new EventSource(`/api/scan?${q}`)
    source.current = es
    const finish = (patch: Partial<RealScan>) => {
      es.close()
      setReal((r) => ({ ...r, ...patch, finishedAt: performance.now() }))
    }
    es.addEventListener('progress', (e) => {
      const p = JSON.parse((e as MessageEvent).data) as ScanProgress
      setReal((r) => ({ ...r, steps: { ...r.steps, [p.step]: p }, last: p }))
    })
    es.addEventListener('report', (e) => finish({ status: 'done', report: JSON.parse((e as MessageEvent).data) as Report }))
    es.addEventListener('failed', (e) => finish({ status: 'failed', error: JSON.parse((e as MessageEvent).data).message }))
    es.onerror = () => {
      if (es.readyState === EventSource.CLOSED || source.current !== es) return
      finish({ status: 'failed', error: 'The lab’s scan bridge isn’t answering. Is npm run lab still running?' })
    }
  }, [])

  useEffect(() => () => source.current?.close(), [])
  return { real, start }
}

export interface Folder {
  path: string
  parent: string | null
  home: string
  entries: { name: string; path: string }[]
}

export async function browse(path: string): Promise<Folder | null> {
  try {
    const res = await fetch(`/api/browse?${new URLSearchParams({ path })}`)
    return res.ok ? ((await res.json()) as Folder) : null
  } catch {
    return null
  }
}
