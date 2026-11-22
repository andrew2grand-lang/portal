import { createContext, use } from 'react'
import type { Dispatch } from 'react'
import { useReducedMotion } from 'motion/react'
import type { MotionValue } from 'motion/react'
import type { Action, Peer, World } from './types'

/** One transfer's live numbers: they change without re-rendering React */
export interface Live {
  /** 0..1 of the bytes that arrived */
  progress: MotionValue<number>
  /** The speed to show, bytes per second (0: not known yet). The core
      already made it calm (Mission 04 §1.2) */
  rate: MotionValue<number>
  /** Time left, in words the core decided */
  eta: MotionValue<string>
}

/** The one contract every view renders from: the real core (CoreProvider),
    or the design lab's still world. */
export interface Portal {
  world: World
  dispatch: Dispatch<Action>
  /** A transfer's live numbers, by its id */
  live: (id: string) => Live
}

export const PortalContext = createContext<Portal | null>(null)

export function usePortal(): Portal {
  const portal = use(PortalContext)
  if (!portal) throw new Error('usePortal must be used inside a Portal provider')
  return portal
}

export function useLive(id: string): Live {
  return usePortal().live(id)
}

/** A device the window knows, by id (a stand-in if it's gone, never a crash) */
export function usePeer(id: string | undefined): Peer {
  const { world } = usePortal()
  return world.peers.find((p) => p.id === id) ?? unknownPeer(id ?? '')
}

export function unknownPeer(id: string): Peer {
  return {
    id,
    name: 'your other computer',
    os: 'Computer',
    short: 'computer',
    self: 'this computer',
    kind: 'other',
    paired: false,
    status: 'nearby',
    incoming: 'ask',
  }
}

/** System preference, or the dev panel's "force reduced motion" */
export function useReducedMotionPref(): boolean {
  const system = useReducedMotion()
  const { world } = usePortal()
  return !!system || world.dev.reducedMotion
}
