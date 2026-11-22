import type { CSSProperties } from 'react'
import { DesktopIcon, DeviceMobileIcon, DeviceTabletIcon, LaptopIcon } from '@phosphor-icons/react'
import type { Device } from '../../engine/types'
import { cssColor, defaultColor, defaultIcon, type Picture } from './picture'
import s from './DevicePicture.module.css'

const icons = { laptop: LaptopIcon, desktop: DesktopIcon, phone: DeviceMobileIcon, tablet: DeviceTabletIcon }

/** A device's picture, a disc sized by the font: its photo, its colour and
    icon, or the default (a colour from its id, its system's icon) */
export function DevicePicture({ device, picture }: { device: Pick<Device, 'id' | 'kind'>; picture?: Picture | null }) {
  if (picture?.kind === 'photo') {
    return (
      <span className={s.picture} data-picture="" data-photo="">
        <img src={`data:image/png;base64,${picture.png}`} alt="" draggable={false} />
      </span>
    )
  }
  const color = picture?.kind === 'mark' ? picture.color : defaultColor(device.id)
  const Icon = icons[picture?.kind === 'mark' ? picture.icon : defaultIcon(device.kind)]
  return (
    <span className={s.picture} data-picture="" style={{ '--pic': cssColor(color) } as CSSProperties}>
      <Icon weight="regular" aria-hidden />
    </span>
  )
}
