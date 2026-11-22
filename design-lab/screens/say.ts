import { useDevices } from '../twoDevices'
import { other } from '../twoDevices'
import { useSide } from '../../src/window/WindowContext'

/** Fills {here} ("this PC") and {there} ("MacBook Pro") for this window */
export function useSay() {
  const devices = useDevices()
  const side = useSide()
  return (text: string) => text.replaceAll('{here}', devices[side].self).replaceAll('{there}', devices[other(side)].name)
}
