import { describe, expect, it } from 'vitest'
import { defaultColor, PICTURE_COLORS } from './picture'

describe('device pictures', () => {
  it('gives the same device the same colour, and similar ids different ones', () => {
    const ids = Array.from({ length: 24 }, (_, i) => `00000000000000000000000000000${String(i).padStart(3, '0')}`)
    expect(defaultColor(ids[0])).toBe(defaultColor(ids[0]))
    expect(new Set(ids.map(defaultColor)).size).toBeGreaterThanOrEqual(PICTURE_COLORS.length - 2)
    // Ids that differ in every character the same way (review: six devices, one colour)
    const repeated = ['1', '2', '3', '4', '5', '6', 'a', 'b'].map((c) => c.repeat(32))
    expect(new Set(repeated.map(defaultColor)).size).toBeGreaterThanOrEqual(4)
  })
})
