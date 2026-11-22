import { useEffect, useState } from 'react'

/** Mini mode (Mission 05 §5): below 360 px wide or 480 px high */
export const MINI_QUERY = '(max-width: 359.98px), (max-height: 479.98px)'

export function useMini(): boolean {
  const [mini, setMini] = useState(() => typeof window !== 'undefined' && window.matchMedia(MINI_QUERY).matches)
  useEffect(() => {
    const m = window.matchMedia(MINI_QUERY)
    const on = () => setMini(m.matches)
    on()
    m.addEventListener('change', on)
    return () => m.removeEventListener('change', on)
  }, [])
  return mini
}
