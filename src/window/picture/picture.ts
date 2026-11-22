/* Device pictures (Mission 05 §2.3): a photo, or a colour and an icon, so
   devices are easy to tell apart. The default is a colour from the device id
   with its system's icon. Photos are cropped to a circle and re-encoded as
   PNG here (at most 256 × 256 and 64 KB); a fresh encode carries no EXIF or
   location data. The core checks every picture again before keeping or
   sending it. */

export type PictureIcon = 'laptop' | 'desktop' | 'phone' | 'tablet'

export type Picture = { kind: 'photo'; png: string } | { kind: 'mark'; color: number; icon: PictureIcon }

/** The colours to choose from (and the defaults): calm, distinct, readable
    with a white icon on top in both themes */
export const PICTURE_COLORS = [0x4f6bed, 0x7c5cd6, 0xd6508f, 0xe07a2f, 0x2f9e75, 0x2b8fc7, 0xb88a1a, 0x8a6a58] as const

export const PICTURE_ICONS: PictureIcon[] = ['laptop', 'desktop', 'phone', 'tablet']

/** The icon a device gets by default: a Mac is a laptop, anything else a desktop */
export const defaultIcon = (kind: 'mac' | 'pc' | 'other'): PictureIcon => (kind === 'mac' ? 'laptop' : 'desktop')

/** The same colour for the same device on every computer */
export function defaultColor(id: string): number {
  let h = 2166136261
  for (let i = 0; i < id.length; i++) h = Math.imul(h ^ id.charCodeAt(i), 16777619)
  // FNV's low bits barely move between similar ids: mix the high ones down
  h ^= h >>> 16
  h = Math.imul(h, 0x45d9f3b)
  h ^= h >>> 16
  return PICTURE_COLORS[(h >>> 0) % PICTURE_COLORS.length]
}

export const cssColor = (c: number) => `#${(c & 0xffffff).toString(16).padStart(6, '0')}`

export const MAX_SIDE = 256
export const MAX_BYTES = 64 * 1024

/** A chosen photo as the core takes it: the centre square, cropped to a
    circle, PNG, base64 (no data: prefix). Smaller sides until it fits. */
export async function photoToPng(file: Blob): Promise<string> {
  // Honours the photo's own rotation, then forgets everything but pixels
  const bitmap = await createImageBitmap(file)
  const side = Math.min(bitmap.width, bitmap.height)
  const sx = (bitmap.width - side) / 2
  const sy = (bitmap.height - side) / 2
  for (const size of [256, 224, 192, 160, 128, 112, 96, 80, 64]) {
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx) break
    ctx.beginPath()
    ctx.arc(size / 2, size / 2, size / 2, 0, Math.PI * 2)
    ctx.clip()
    ctx.imageSmoothingQuality = 'high'
    ctx.drawImage(bitmap, sx, sy, side, side, 0, 0, size, size)
    const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, 'image/png'))
    if (blob && blob.size <= MAX_BYTES) {
      bitmap.close()
      return toBase64(new Uint8Array(await blob.arrayBuffer()))
    }
  }
  bitmap.close()
  throw new Error('That photo can’t be made small enough. Try another.')
}

function toBase64(bytes: Uint8Array): string {
  let s = ''
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  return btoa(s)
}
