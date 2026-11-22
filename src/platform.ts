import type { Side } from './engine/types'
import tauriConf from '../src-tauri/tauri.conf.json'

/** Product name: single source is `productName` in src-tauri/tauri.conf.json */
export const APP_NAME: string = tauriConf.productName

/** Running inside the Tauri app (vs. the browser design canvas) */
export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** The OS this window runs on, which decides its chrome and wording */
function detectSide(): Side {
  if (typeof navigator === 'undefined' || typeof window === 'undefined') return 'mac'
  // Dev: ?os=win previews the Windows window in a browser (with ?mock=…)
  const forced = import.meta.env.DEV ? new URLSearchParams(window.location.search).get('os') : null
  if (forced === 'win' || forced === 'mac') return forced
  return /Windows/.test(navigator.userAgent) ? 'win' : 'mac'
}

export const hostSide: Side = detectSide()
