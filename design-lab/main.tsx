/* SeamDrop's design lab: every Project Send screen, in real SeamDrop windows.
   Nothing here ships in the app. */
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { IconContext } from '@phosphor-icons/react'
import '../src/styles/base.css'
import { Lab } from './Lab'

const iconDefaults = { weight: 'regular' as const, size: 16, 'aria-hidden': true }

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <IconContext value={iconDefaults}>
      <Lab />
    </IconContext>
  </StrictMode>,
)
