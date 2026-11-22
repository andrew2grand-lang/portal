import { createContext, use } from 'react'
import type { Side } from '../engine/types'

/** Which machine a component is rendering inside. The OS follows the side. */
export const WindowContext = createContext<Side>('mac')

export function useSide(): Side {
  return use(WindowContext)
}
