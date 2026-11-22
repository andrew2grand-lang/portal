import { createContext } from 'react'

/** Set by the stage while a transfer shows with several devices paired: its
    pane ends with a way to send something else to the others */
export const SendMoreContext = createContext(false)
