import { useEffect, useMemo, useRef, useState } from 'react'
import { MagnifyingGlassIcon } from '@phosphor-icons/react'
import { TextField } from '../../components/TextField/TextField'
import { usePortal } from '../../engine/context'
import { APP_NAME } from '../../platform'
import { useSide } from '../WindowContext'
import s from './Stage.module.css'
import { Head, Pane } from './parts'

/** One Help section: a title and short plain paragraphs */
interface Topic {
  title: string
  paras: string[]
}

/** Every explanation in one place (Mission 05 §6): the screens keep only a
    title and, where it prevents a mistake, one short line. */
function topics(side: 'mac' | 'win'): Topic[] {
  const paste = side === 'mac' ? '⌘V' : 'Ctrl+V'
  const reveal = side === 'mac' ? 'Show in Finder' : 'Show in File Explorer'
  return [
    {
      title: 'Pairing',
      paras: [
        `Open ${APP_NAME} on both computers, on the same network. Each computer running it shows up in the list with its name, system and address.`,
        'Press Pair beside the one you want. The other computer asks first: press Show code there, then type its six digits on the computer where you pressed Pair. The code works once and expires after two minutes.',
        'Show a code only when you are pairing that computer with your own. Three wrong codes lock pairing on that computer until you allow it again in Settings.',
        'Pair once: from then on the two know each other and connect by themselves whenever both are open.',
      ],
    },
    {
      title: 'Sending files',
      paras: [
        'Drop files or folders on the window, or use Add files and Add folder. Tick the computers they go to, then press Send.',
        'The other computer asks first, unless you chose Always accept for this one there. Every file is checked when it arrives, to make sure it is complete and unchanged.',
        'A computer that is busy with a transfer gets your files next: they wait, and start when that transfer ends.',
        'If the connection drops, the transfer pauses and continues by itself when both computers can see each other again. If you quit the app during a transfer, it asks next time whether to continue or cancel.',
      ],
    },
    {
      title: 'Clipboard and sync',
      paras: [
        `Send clipboard sends what you copied to the ticked computers, ready to paste with ${paste} there. Files you copied go as a transfer instead.`,
        'Automatic sync sends every copy by itself. Both computers have to turn it on, in Clipboard; turning it off on either one stops it. Pause stops it on this computer for a while.',
        'Recently received keeps the last five things that arrived, until you quit. Images can be saved as a file from there.',
        'If a copy is not synced, the window says why, once: marked private, too big, or it could not be read.',
        'Copies made while the other computer is away are not sent later; the next copy after it is back is.',
        `What arrives marked private, and what you copy from a private Shelf item, is kept out of clipboard history${side === 'win' ? ' (Win+V) and the cloud clipboard' : ' and clipboard managers that respect the private mark'}.`,
      ],
    },
    {
      title: 'The Shelf',
      paras: [
        'The Shelf is text you keep, to copy here or on your other computers: addresses, passwords, snippets you paste often. Nothing lands on it by itself.',
        'Add text with Add to Shelf, or press Put on Shelf on something that arrived, or on a private copy that wasn’t synced. Copy puts an item back on the clipboard.',
        'To share it with a paired computer, press Share with its name on the Shelf (or in Settings), then press Share on that computer when it asks. Each computer shows its own items and those of the computers it shares with directly; items are not passed further along.',
        'A computer that is away gets what changed when it is back.',
        'Removing an item, or Clear Shelf, removes it on every computer it is shared with. Undo brings it back for a few seconds.',
      ],
    },
    {
      title: 'Privacy',
      paras: [
        `${APP_NAME} works only on your network, between your own computers. Files and the clipboard go directly between them, encrypted, never through a server.`,
        'Apps can mark what they copy as private, as password managers do, and some browsers do in private windows: Edge, Chrome and Firefox, and Tor Browser always. Those copies are never synced. Other browsers may not mark them, so copy from them with care.',
        `${APP_NAME} never looks at what you copied to guess whether it is private; only those marks count. It reads a marked copy only when you press Put on Shelf on its notice, and only if it is still the same copy.`,
        side === 'mac'
          ? 'The Shelf is kept encrypted on disk. On a Mac its key is a file next to it that only your account can read: this stops backups, sync tools and someone copying the files, but not another app running as you.'
          : 'The Shelf is kept encrypted on disk, with a key Windows protects for your account: it can’t be opened from another account, another computer or a backup.',
        'Private Shelf items show as dots until you press Reveal, and never appear in notifications, Details or the technical log.',
      ],
    },
    {
      title: 'Where files go',
      paras: [
        'Received files go to your Downloads folder by default. In Settings you can choose: straight into a folder, into a SeamDrop folder (optionally one for each computer), or a new numbered folder for each delivery.',
        `Files and folders are dated when they arrive, so they are the newest in the folder; Settings can keep the sender’s dates instead. ${reveal} on the finished screen opens the folder with the new files selected.`,
      ],
    },
    {
      title: 'Troubleshooting',
      paras: [
        'If the other computer doesn’t show up: check that both are on the same network and that the app is open on both. Guest, school and office Wi-Fi often keep devices apart.',
        'Connect by IP connects straight to the other computer’s address; each computer shows its own address in Settings.',
        side === 'mac'
          ? 'On a Mac, the app needs Local Network access: System Settings → Privacy & Security → Local Network.'
          : 'On Windows, the firewall must allow the app on private networks. The connection check (Settings → Connection) can fix a rule that blocks it.',
        'Connection check in Settings lists what works and what doesn’t, in plain words.',
        'Get test builds (Settings → About) installs early versions before everyone else. Turning it off keeps the version you have until a regular update is newer.',
      ],
    },
    {
      title: 'Supported systems',
      paras: [
        'macOS 11 Big Sur and later, on Apple silicon and Intel (one app for both).',
        'Windows 10 and 11 on 64-bit Intel and AMD processors. Windows 11 on ARM runs it through its built-in emulation; Windows 10 on ARM can’t run it.',
        'Older systems aren’t supported: they no longer get security updates.',
      ],
    },
  ]
}

/** Help: short, searchable sections in plain words */
export function HelpView() {
  const side = useSide()
  const { world, dispatch } = usePortal()
  const [q, setQ] = useState('')
  const sections = useRef<HTMLDivElement>(null)
  // Opened from "How the Shelf works": that section first
  const topic = world.helpTopic
  useEffect(() => {
    if (!topic) return
    sections.current?.querySelector(`[data-topic="${CSS.escape(topic)}"]`)?.scrollIntoView({ block: 'start' })
  }, [topic])
  const all = useMemo(() => topics(side), [side])
  const words = q.trim().toLowerCase().split(/\s+/).filter(Boolean)
  const shown = all.filter((t) => !words.length || words.every((w) => `${t.title} ${t.paras.join(' ')}`.toLowerCase().includes(w)))
  return (
    <Pane onClose={() => dispatch({ type: 'openPanel', panel: null })}>
      <Head title="Help" />
      <TextField label={<MagnifyingGlassIcon aria-label="Search Help" />} value={q} onChange={setQ} placeholder="Search Help" />
      {shown.length === 0 && <p className={s.aside}>Nothing in Help matches that.</p>}
      <div ref={sections}>
        {shown.map((t) => (
          <section key={t.title} className={s.helpTopic} aria-label={t.title} data-topic={t.title}>
            <h3 className={s.helpTitle}>{t.title}</h3>
            {t.paras.map((p) => (
              <p key={p} className={s.helpPara}>
                {p}
              </p>
            ))}
          </section>
        ))}
      </div>
    </Pane>
  )
}
