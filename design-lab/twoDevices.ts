/* The design lab designs Project Send between two computers, a Mac and a PC
   side by side (Mission 02). The app is keyed by device id since Mission 04;
   this keeps the lab's two-window view on top of the same components. */
import { createContext, use } from 'react'
import type { MotionValue } from 'motion/react'
import type { Device, LogLine, Peer, Side, Theme, World } from '../src/engine/types'

export const other = (side: Side): Side => (side === 'mac' ? 'win' : 'mac')

export const labDevices: Record<Side, Device> = {
  mac: { id: 'lab-mac', name: 'MacBook Pro', os: 'macOS', short: 'Mac', self: 'this Mac', kind: 'mac' },
  win: { id: 'lab-win', name: 'Desk PC', os: 'Windows 11', short: 'PC', self: 'this PC', kind: 'pc' },
}

/** The lab's two computers, by window */
export function useDevices(): Record<Side, Device> {
  return labDevices
}

export interface LabSettings {
  theme: Theme
  reducedMotion: boolean
  logs: Record<Side, LogLine[]>
  progress: MotionValue<number>
  rate: MotionValue<number>
  eta: MotionValue<string>
}

export const LabContext = createContext<LabSettings | null>(null)

export function useLab(): LabSettings {
  const lab = use(LabContext)
  if (!lab) throw new Error('useLab must be used inside LabProvider')
  return lab
}

/** A still world as seen from one window: the other computer paired and connected on Wi-Fi */
export function worldFor(side: Side, lab: LabSettings): World {
  const there = labDevices[other(side)]
  const peer: Peer = { ...there, paired: true, status: 'connected', link: 'wifi', incoming: 'ask' }
  return {
    ready: true,
    me: labDevices[side],
    peers: [peer],
    target: [peer.id],
    focus: peer,
    searchStartedAt: 0,
    panel: null,
    pairing: null,
    settings: {
      deviceName: labDevices[side].name,
      receiveDir: '',
      skipJunk: true,
      folderPerDevice: false, receiveMode: 'loose', keepDates: false,
      pairingLocked: false,
      appVersion: '0.4.0',
      osVersion: '',
      openAtLogin: false,
      addresses: [],
    },
    doctor: [],
    connect: { busy: false },
    dragOver: false,
    currentFile: {},
    queue: [],
    ignored: [],
  next: [],
  shelf: [],
    mode: 'project',
    sessions: [],
    sendErrors: [],
    logs: lab.logs[side],
    dev: { open: false, theme: lab.theme, reducedMotion: lab.reducedMotion, projectTab: true },
    appearance: 'system',
  }
}
