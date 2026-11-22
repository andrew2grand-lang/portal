import { useState } from 'react'
import { DownloadSimpleIcon, EnvelopeSimpleIcon } from '@phosphor-icons/react'
import { Checkbox } from '../../components/Checkbox/Checkbox'
import { Key } from '../../components/Key/Key'
import { usePortal } from '../../engine/context'
import { LINKS } from '../../links'
import s from './Stage.module.css'

/** What the technical log holds, said before anyone shares it (Review 04) */
export const LOG_HOLDS =
  'The technical log lists what happened: file and folder names, computer names and network addresses. Never what’s in your files or your clipboard.'

/** Send feedback (Settings → About; Mission 04 §5). With the log ticked, the
    technical log is saved and shown in Finder / Explorer to attach. The email
    opens only once the address is confirmed (`src/links.ts`). */
export function FeedbackKeys() {
  const { dispatch } = usePortal()
  const [withLog, setWithLog] = useState(true)
  // Until email works, the button says what it does: it saves a file
  if (!LINKS.confirmed) {
    return (
      <div className={s.feedback}>
        <div className={s.deviceKeys}>
          <Key size="sm" icon={<DownloadSimpleIcon />} onClick={() => dispatch({ type: 'saveTechnicalLog' })}>
            Save technical log
          </Key>
        </div>
        <p className={s.aside}>{LOG_HOLDS} It’s saved in Downloads for you to send to support. Feedback by email isn’t set up yet.</p>
      </div>
    )
  }
  return (
    <div className={s.feedback}>
      <div className={s.deviceKeys}>
        <Key size="sm" icon={<EnvelopeSimpleIcon />} onClick={() => dispatch({ type: 'sendFeedback', withLog })}>
          Send feedback
        </Key>
        <Checkbox checked={withLog} onChange={setWithLog}>
          Include technical log
        </Checkbox>
      </div>
      {withLog && <p className={s.aside}>{LOG_HOLDS}</p>}
    </div>
  )
}
