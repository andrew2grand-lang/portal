import { useEffect } from 'react'
import type { Theme } from './types'

/** Applies the theme to the document and the browser/window chrome colour */
export function useThemeEffect(theme: Theme) {
  useEffect(() => {
    document.documentElement.dataset.theme = theme
    const canvas = getComputedStyle(document.documentElement).getPropertyValue('--canvas').trim()
    document.querySelector('meta[name="theme-color"]')?.setAttribute('content', canvas)
  }, [theme])
}
