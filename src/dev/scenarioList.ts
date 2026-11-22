/* Every state of the real window the mock core can show (src/dev/mockCore.ts).
   Shared by the scenario index (/), the screenshots and the overlap test. */
export const scenarioList = [
  'searching',
  'searching-long',
  'nearby',
  'revoked',
  /** Pairing by typed code (Mission 04 §2): asked here; showing the code;
      a wrong code typed there; typing it here; "Paired with"; locked */
  'pairing-incoming',
  'pairing-show',
  'pairing-wrong-there',
  'pairing-type',
  'pairing-wrong-here',
  'pairing-paired',
  'pairing-locked',
  'online',
  'online-ethernet',
  /** Paired, not connected yet: the core is still trying */
  'connecting',
  'offline',
  'disconnected',
  'incompatible',
  /** The other computer runs a newer version: update this one */
  'incompatible-self',
  'offer-in',
  /** Accept it and it's done 40 ms later: the arrival moment */
  'instant',
  'offer-out',
  'sending',
  /** Sending, with more waiting to go next (Mission 05 §1.5) */
  'next',
  'receiving',
  'stalled',
  /** The connection dropped mid-transfer: both sides wait (Mission 03) */
  'paused-out',
  'paused-in',
  /** The app was quit partway (Mission 05 §1.4): asks here, waits there */
  'stopped-out',
  'stopped-in',
  'waiting-there',
  /** Paused, the other computer is back: checking what arrived */
  'resuming',
  'done-in',
  'done-out',
  'failed',
  /** Cancel pressed here: the sender's and the receiver's screens */
  'cancelled',
  'cancelled-in',
  'blocked-peer',
  'declined',
  'refused',
  /** Several devices at once (Mission 04 §3) */
  'many-sending',
  'many-mixed',
  'many-offers',
  'add-device',
] as const

export type Scenario = (typeof scenarioList)[number]
