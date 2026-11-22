import type { ReactNode } from 'react'
import { MinusIcon, SquareIcon, XIcon } from '@phosphor-icons/react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { BrandMark } from '../components/BrandMark/BrandMark'

import type { Side } from '../engine/types'
import { APP_NAME } from '../platform'
import s from './AppWindow.module.css'
import { usePortal } from '../engine/context'

interface AppWindowProps {
  side: Side
  /** Sits in the shell, around the window edge */
  glow?: ReactNode
  /** A real OS window (Tauri): the OS draws the frame, the HTML chrome turns live */
  native?: boolean
  /** A small control in the title bar (settings) */
  accessory?: ReactNode
  children: ReactNode
}

/** One app window with OS-native chrome. Same brand, each OS's conventions. */
export function AppWindow({ side, glow, native = false, accessory, children }: AppWindowProps) {
  const { world } = usePortal()
  return (
    <section
      className={s.shell}
      data-os={side}
      data-native={native ? '' : undefined}
      aria-label={native ? APP_NAME : `${APP_NAME} on ${world.me.name}`}
    >
      {glow}
      <div className={s.window}>
        {side === 'mac' ? (
          <MacTitlebar native={native} accessory={accessory} />
        ) : (
          <WinTitlebar native={native} accessory={accessory} />
        )}
        <div className={s.body}>{children}</div>
      </div>
    </section>
  )
}

/* macOS: traffic lights top-left, centred title. In the app the OS draws the
   real traffic lights over this bar (overlay title bar), so only the title stays. */
function MacTitlebar({ native, accessory }: { native: boolean; accessory?: ReactNode }) {
  return (
    <header className={s.macBar} data-tauri-drag-region={native ? '' : undefined}>
      {!native && (
        <span className={s.traffic} aria-hidden>
          <span data-light="close" />
          <span data-light="minimise" />
          <span data-light="zoom" />
        </span>
      )}
      <span className={s.macTitle} data-tauri-drag-region={native ? '' : undefined}>
        {APP_NAME}
      </span>
      {accessory && <span className={s.macAccessory}>{accessory}</span>}
    </header>
  )
}

/* Windows: app icon + left-aligned title, caption buttons top-right. In the
   app the window is undecorated and these buttons drive it. */
function WinTitlebar({ native, accessory }: { native: boolean; accessory?: ReactNode }) {
  const drag = native ? '' : undefined
  return (
    <header className={s.winBar} data-tauri-drag-region={drag}>
      <span className={s.winIcon} data-tauri-drag-region={drag}>
        <BrandMark size={16} />
      </span>
      <span className={s.winTitle} data-tauri-drag-region={drag}>
        {APP_NAME}
      </span>
      {accessory && <span className={s.winAccessory}>{accessory}</span>}
      {native ? (
        <span className={s.captions}>
          <button type="button" aria-label="Minimise" onClick={() => void getCurrentWindow().minimize()}>
            <MinusIcon weight="light" />
          </button>
          <button type="button" aria-label="Maximise" onClick={() => void getCurrentWindow().toggleMaximize()}>
            <SquareIcon weight="light" />
          </button>
          <button type="button" aria-label="Close" data-close onClick={() => void getCurrentWindow().close()}>
            <XIcon weight="light" />
          </button>
        </span>
      ) : (
        <span className={s.captions} aria-hidden>
          <span>
            <MinusIcon weight="light" />
          </span>
          <span>
            <SquareIcon weight="light" />
          </span>
          <span data-close>
            <XIcon weight="light" />
          </span>
        </span>
      )}
    </header>
  )
}
