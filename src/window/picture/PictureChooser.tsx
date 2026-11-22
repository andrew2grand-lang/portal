import { useRef, useState } from 'react'
import { CameraIcon, DesktopIcon, DeviceMobileIcon, DeviceTabletIcon, LaptopIcon } from '@phosphor-icons/react'
import { Key } from '../../components/Key/Key'
import { usePortal } from '../../engine/context'
import { DevicePicture } from './DevicePicture'
import { cssColor, defaultColor, defaultIcon, PICTURE_COLORS, PICTURE_ICONS, photoToPng, type PictureIcon } from './picture'
import s from './DevicePicture.module.css'

const iconOf = { laptop: LaptopIcon, desktop: DesktopIcon, phone: DeviceMobileIcon, tablet: DeviceTabletIcon }
const iconName: Record<PictureIcon, string> = { laptop: 'Laptop', desktop: 'Desktop', phone: 'Phone', tablet: 'Tablet' }

/** Settings → This computer → Picture (Mission 05 §2.3): a photo, or a
    colour and an icon, or the default. Paired computers see it. */
export function PictureChooser() {
  const { world, dispatch } = usePortal()
  const me = world.me
  const pic = me.picture
  const file = useRef<HTMLInputElement>(null)
  const [error, setError] = useState<string>()
  // The colour and icon showing now, so a change keeps the other half
  const color = pic?.kind === 'mark' ? pic.color : defaultColor(me.id)
  const icon = pic?.kind === 'mark' ? pic.icon : defaultIcon(me.kind)
  const set = (next: Parameters<typeof DevicePicture>[0]['picture']) => {
    setError(undefined)
    dispatch({ type: 'setPicture', picture: next ?? null })
  }
  const choose = async (f: File | undefined) => {
    if (!f) return
    try {
      set({ kind: 'photo', png: await photoToPng(f) })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'That photo can’t be used. Try another.')
    }
  }
  return (
    <div className={s.chooser}>
      <p className={s.chooserLabel}>Picture</p>
      <div className={s.chooserRow}>
        <span className={s.chooserNow}>
          <DevicePicture device={me} picture={pic} />
        </span>
        <div className={s.chooserParts}>
          <div className={s.swatches} role="radiogroup" aria-label="Colour">
            {PICTURE_COLORS.map((c) => (
              <button
                key={c}
                type="button"
                role="radio"
                aria-checked={pic?.kind !== 'photo' && c === color}
                aria-label={`Colour ${cssColor(c)}`}
                className={s.swatch}
                style={{ background: cssColor(c) }}
                onClick={() => set({ kind: 'mark', color: c, icon })}
              />
            ))}
          </div>
          <div className={s.swatches} role="radiogroup" aria-label="Icon">
            {PICTURE_ICONS.map((i) => {
              const Icon = iconOf[i]
              return (
                <button
                  key={i}
                  type="button"
                  role="radio"
                  aria-checked={pic?.kind !== 'photo' && i === icon}
                  aria-label={iconName[i]}
                  title={iconName[i]}
                  className={s.iconChoice}
                  onClick={() => set({ kind: 'mark', color, icon: i })}
                >
                  <Icon weight="regular" aria-hidden />
                </button>
              )
            })}
          </div>
          <div className={s.chooserKeys}>
            <Key size="sm" icon={<CameraIcon />} onClick={() => file.current?.click()}>
              Choose photo…
            </Key>
            {pic && (
              <Key size="sm" onClick={() => set(null)}>
                Default
              </Key>
            )}
          </div>
        </div>
      </div>
      {error && (
        <p className={s.chooserError} role="alert">
          {error}
        </p>
      )}
      <input
        ref={file}
        type="file"
        accept="image/png,image/jpeg,image/heic,image/webp"
        hidden
        onChange={(e) => {
          void choose(e.target.files?.[0])
          e.target.value = ''
        }}
      />
    </div>
  )
}
