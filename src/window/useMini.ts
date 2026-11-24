import { useEffect, useState } from 'react'

/** Mini mode (Mission 05 §5): below 360 px wide or 480 px high */
export const MINI_QUERY = '(max-width: 359.98px), (max-height: 479.98px)'

export function useMini(): boolean {
  const [mini, setMini] = useState(() => typeof window !== 'undefined' && window.matchMedia(MINI_QUERY).matches)
  useEffect(() => {
    const m = window.matchMedia(MINI_QUERY)
    const on = () => setMini(window.matchMedia(MINI_QUERY).matches)
    on()
    // The query's own change event, and every resize too: WebKit doesn't
    // always send the change event when the window is resized (seen in CI)
    m.addEventListener('change', on)
    window.addEventListener('resize', on)
    return () => {
      m.removeEventListener('change', on)
      window.removeEventListener('resize', on)
    }
  }, [])
  return mini
}
