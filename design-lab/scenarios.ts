/* Demo scenarios. Each report is real scanner output for a demo project in
   design-lab/projects (npm run lab:demos). Conversion, verification and AI
   data is designed demo data: that pipeline doesn't exist yet. */
import { APP_NAME } from '../src/platform'
import type { Side } from '../src/engine/types'
import type { Report } from './report'
import handbook from './data/Handbook.json'
import invoiceDesk from './data/InvoiceDesk.json'
import ledgerTool from './data/ledger-tool.json'
import localChat from './data/local-chat.json'
import shopWeb from './data/shop-web.json'
import sketchbook from './data/Sketchbook.json'

export type ResultState = 'proved' | 'failed' | 'couldntCheck' | 'needsYou'

export interface Result {
  state: ResultState
  title: string
  detail: string
}

export interface DiffLine {
  kind: 'ctx' | 'add' | 'del'
  text: string
}

export interface Change {
  id: string
  file: string
  summary: string
  /** pattern: a known fix; ai: written by the repair agent */
  by: 'pattern' | 'ai'
  added?: boolean
  lines: DiffLine[]
}

export type StepId = 'snapshot' | 'pack' | 'plan' | 'apply' | 'verify' | 'report' | 'repair'

export interface Check {
  label: string
  state: 'proved' | 'failed' | 'running' | 'pending'
  detail?: string
}

export interface Conversion {
  /** A fix no known pattern covers: the AI writes it, with your OK */
  forTheAi?: string
  /** What each step did, once done */
  done: Partial<Record<StepId, string>>
  /** What each step is doing, while it runs */
  doing: Partial<Record<StepId, string>>
  changes: Change[]
  checks: Check[]
  repair: {
    attempt: number
    of: number
    problem: string
    trying: string
    earlier: { attempt: number; tried: string; result: string }[]
    log: string[]
    spentHere: number
  }
  consent: {
    command: string
    from: string
    why: string
    aiFiles: { path: string; bytes: number }[]
    aiProblem: string
    neverSent: string[]
  }
  final: { headline: string; sub: string; results: Result[]; plan: string }
}

export interface Rebuild {
  summary: string
  blockers: { title: string; reason: string }[]
  keeps: string[]
  steps: { title: string; detail: string }[]
}

export interface Scenario {
  id: string
  label: string
  /** The sending computer; the other one receives */
  from: Side
  report: Report
  /** Files the scanning screen passes through */
  files: string[]
  convert: Conversion | null
  rebuild: Rebuild | null
}

/* Words like {here} ("this PC") and {there} ("MacBook Pro") name the
   computers from the window's point of view; see say() in screens/parts. */

/** Usage this month, shared by every Pro screen */
export const usage = { spent: 3.4, cap: 20, gift: 5, renews: '1 November' }

export const scenarios: Scenario[] = [
  {
    id: 'local-chat',
    label: 'Python local-AI app · Mac → Windows',
    from: 'mac',
    report: localChat as Report,
    files: [
      'pyproject.toml',
      'uv.lock',
      'app/main.py',
      'app/chat.py',
      'app/ingest.py',
      'scripts/build_index.py',
      'tests/test_ingest.py',
      'run.sh',
      'README.md',
    ],
    convert: {
      done: {
        snapshot: 'Python 3.12, 8 packages, a FastAPI app that talks to Ollama',
        pack: '14 files, 61 KB. The 1.8 GB of .venv and data/index stays on {there}.',
        plan: '6 fixes, all from known patterns',
        apply: '6 changes in 5 files',
        verify: '4 of 5 checks proved, 1 couldn’t be checked',
        report: 'Done',
        repair: '1 test fixed on the second try',
      },
      doing: {
        snapshot: 'Reading how local-chat runs on {there}',
        pack: 'Packing the project without .venv and the index',
        plan: 'Matching 6 findings to fix patterns',
        apply: 'Changing how files are opened in app/main.py',
        verify: 'Running the tests on {here} (9 of 14)',
        report: 'Writing what was proved',
        repair: 'Trying a fix for 1 failing test',
      },
      changes: [
        {
          id: 'enc-main',
          file: 'app/main.py',
          summary: 'Open the page as UTF-8',
          by: 'pattern',
          lines: [
            { kind: 'ctx', text: '@app.get("/", response_class=HTMLResponse)' },
            { kind: 'ctx', text: 'def home():' },
            { kind: 'del', text: '    with open(PAGE) as f:' },
            { kind: 'add', text: '    with open(PAGE, encoding="utf-8") as f:' },
            { kind: 'ctx', text: '        return f.read()' },
          ],
        },
        {
          id: 'path-main',
          file: 'app/main.py',
          summary: 'Build the index path with pathlib',
          by: 'pattern',
          lines: [
            { kind: 'del', text: 'DATA_DIR = os.getenv("LOCAL_CHAT_DATA", str(Path(__file__).parent.parent / "data"))' },
            { kind: 'del', text: 'INDEX_DIR = DATA_DIR + "/index"' },
            { kind: 'add', text: 'DATA_DIR = Path(os.getenv("LOCAL_CHAT_DATA", Path(__file__).parent.parent / "data"))' },
            { kind: 'add', text: 'INDEX_DIR = str(DATA_DIR / "index")' },
          ],
        },
        {
          id: 'enc-ingest',
          file: 'app/ingest.py',
          summary: 'Open notes as UTF-8',
          by: 'pattern',
          lines: [
            { kind: 'ctx', text: 'def read_notes(path: str) -> str:' },
            { kind: 'del', text: '    with open(path) as f:' },
            { kind: 'add', text: '    with open(path, encoding="utf-8") as f:' },
            { kind: 'ctx', text: '        return f.read()' },
          ],
        },
        {
          id: 'guard-index',
          file: 'scripts/build_index.py',
          summary: 'Start the workers under a __main__ guard',
          by: 'pattern',
          lines: [
            { kind: 'del', text: 'with ProcessPoolExecutor() as pool:' },
            { kind: 'add', text: 'def main():' },
            { kind: 'add', text: '    with ProcessPoolExecutor() as pool:' },
            { kind: 'ctx', text: '        for pdf, parts in zip(pdfs, pool.map(passages, map(str, pdfs))):' },
            { kind: 'add', text: '' },
            { kind: 'add', text: 'if __name__ == "__main__":' },
            { kind: 'add', text: '    main()' },
          ],
        },
        {
          id: 'run-ps1',
          file: 'run.ps1',
          summary: 'A Windows version of run.sh',
          by: 'pattern',
          added: true,
          lines: [
            { kind: 'add', text: '# Starts the web app on port 8000 (Ollama must already be running).' },
            { kind: 'add', text: '$env:OLLAMA_HOST = "127.0.0.1:11434"' },
            { kind: 'add', text: 'uvicorn app.main:app --reload --port 8000' },
          ],
        },
        {
          id: 'readme',
          file: 'README.md',
          summary: 'Windows steps in Run',
          by: 'pattern',
          lines: [
            { kind: 'ctx', text: '```sh' },
            { kind: 'del', text: 'python3 -m venv .venv && source .venv/bin/activate' },
            { kind: 'add', text: 'py -m venv .venv; .venv\\Scripts\\Activate.ps1' },
            { kind: 'ctx', text: 'pip install -e .' },
            { kind: 'del', text: './run.sh' },
            { kind: 'add', text: '.\\run.ps1' },
          ],
        },
        {
          id: 'test-ids',
          file: 'app/ingest.py',
          summary: 'Name passages with / on every OS',
          by: 'ai',
          lines: [
            { kind: 'ctx', text: '    return [text[i : i + size] for i in range(0, len(text), size)]' },
            { kind: 'add', text: '' },
            { kind: 'add', text: '' },
            { kind: 'add', text: 'def passage_id(path: Path, i: int) -> str:' },
            { kind: 'add', text: '    """The same name on every OS: docs/a.pdf-0"""' },
            { kind: 'add', text: '    return f"{path.as_posix()}-{i}"' },
          ],
        },
      ],
      checks: [
        { label: 'Install Python 3.12 and 8 packages', state: 'proved', detail: 'uv sync, 41 s' },
        { label: 'Run the tests', state: 'proved', detail: '14 of 14 passed, same as on {there}' },
        { label: 'Start the app', state: 'proved', detail: 'http://127.0.0.1:8000 answered in 1.9 s' },
        { label: 'Build the index from 2 sample PDFs', state: 'proved', detail: '4 workers started and stopped cleanly' },
        { label: 'Ask the model a question', state: 'pending', detail: 'Needs Ollama on {here}' },
      ],
      repair: {
        attempt: 2,
        of: 3,
        problem: 'tests/test_ingest.py: 1 test failed on Windows. Expected "docs/a.pdf-0", got "docs\\a.pdf-0".',
        trying: 'Build passage names with as_posix() so they read the same on both computers',
        earlier: [
          {
            attempt: 1,
            tried: 'Compare the names as paths in the test',
            result: 'The test passed, but the index on {there} uses / names: the old index would not match. Undone.',
          },
        ],
        log: [
          'read tests/test_ingest.py, app/ingest.py',
          'cause: str(Path) uses \\ on Windows',
          'edit app/ingest.py: passage_id() with as_posix()',
          'run: pytest tests/test_ingest.py',
        ],
        spentHere: 0.18,
      },
      consent: {
        command: 'uvicorn app.main:app --port 8000',
        from: 'run.sh, line 4',
        why: `${APP_NAME} starts the app on {here} to check it answers, then stops it.`,
        aiFiles: [
          { path: 'tests/test_ingest.py', bytes: 412 },
          { path: 'app/ingest.py', bytes: 1_230 },
          { path: 'app/main.py', bytes: 1_604 },
          { path: 'pyproject.toml', bytes: 598 },
        ],
        aiProblem: '1 test fails on Windows: passage names use \\ instead of /.',
        neverSent: ['.env', 'certificates and key files', '.venv and data/index', 'anything else in .gitignore'],
      },
      final: {
        headline: 'Runs on Windows. 2 things need you.',
        sub: 'local-chat is in C:\\Users\\Sam\\Projects\\local-chat. 7 changes, each one undoable.',
        results: [
          { state: 'proved', title: 'Installs on Windows', detail: 'Python 3.12 and all 8 packages, from uv.lock.' },
          { state: 'proved', title: 'Tests pass', detail: '14 of 14, the same as on {there}.' },
          { state: 'proved', title: 'The app starts', detail: 'http://127.0.0.1:8000 answered with the chat page.' },
          { state: 'proved', title: 'The index builds', detail: '4 worker processes started and finished on 2 sample PDFs.' },
          {
            state: 'needsYou',
            title: 'Install Ollama on {here}',
            detail: 'Then run ollama pull llama3.2. The model and its server aren’t part of the project.',
          },
          {
            state: 'needsYou',
            title: 'Add your keys to .env',
            detail: `OPENAI_API_KEY and HF_TOKEN. ${APP_NAME} never copies secrets; .env.example lists what’s needed.`,
          },
          {
            state: 'couldntCheck',
            title: 'Answers from the model',
            detail: 'Needs Ollama running on {here}, so this wasn’t tried. Nothing is claimed.',
          },
        ],
        plan: '# local-chat on Windows\n\n1. Install Ollama, then `ollama pull llama3.2`.\n2. Copy `.env.example` to `.env` and add `OPENAI_API_KEY`, `HF_TOKEN`.\n3. Start with `.\\run.ps1` and open http://localhost:8000.\n',
      },
    },
    rebuild: null,
  },
  {
    id: 'shop-web',
    label: 'Node web app · Mac → Windows',
    from: 'mac',
    report: shopWeb as Report,
    files: ['package.json', 'package-lock.json', 'server/index.js', 'src/main.tsx', 'src/App.tsx', 'vite.config.ts', 'scripts/deploy.sh'],
    convert: {
      done: {
        snapshot: 'Node 22, 8 packages, a React app served by Express',
        pack: '12 files, 18 KB. node_modules (322 MB) is rebuilt on {here}.',
        plan: '5 fixes, all from known patterns',
        apply: '5 changes in 3 files',
        verify: '4 of 5 checks proved, 1 couldn’t be checked',
        report: 'Done',
        repair: 'Not needed: every check passed first time',
      },
      doing: {
        snapshot: 'Reading how shop-web runs on {there}',
        pack: 'Packing the project without node_modules',
        plan: 'Matching 5 findings to fix patterns',
        apply: 'Rewriting the build script in package.json',
        verify: 'Running npm run build on {here}',
        report: 'Writing what was proved',
        repair: 'Not needed',
      },
      changes: [
        {
          id: 'scripts',
          file: 'package.json',
          summary: 'npm scripts that work in cmd.exe and PowerShell',
          by: 'pattern',
          lines: [
            { kind: 'del', text: '"dev": "NODE_ENV=development vite",' },
            { kind: 'add', text: '"dev": "cross-env NODE_ENV=development vite",' },
            { kind: 'del', text: '"build": "rm -rf dist && vite build && cp -r public/fonts dist/fonts",' },
            { kind: 'add', text: '"build": "rimraf dist && vite build && cpy \\"public/fonts/**\\" dist/fonts",' },
            { kind: 'del', text: '"lint": "eslint \'src/**/*.{ts,tsx}\'",' },
            { kind: 'add', text: '"lint": "eslint \\"src/**/*.{ts,tsx}\\"",' },
          ],
        },
        {
          id: 'devdeps',
          file: 'package.json',
          summary: 'Add cross-env, rimraf and cpy-cli',
          by: 'pattern',
          lines: [
            { kind: 'ctx', text: '"devDependencies": {' },
            { kind: 'add', text: '  "cpy-cli": "5.0.0",' },
            { kind: 'add', text: '  "cross-env": "7.0.3",' },
            { kind: 'add', text: '  "rimraf": "6.0.1",' },
            { kind: 'ctx', text: '  "eslint": "9.17.0",' },
          ],
        },
        {
          id: 'deploy',
          file: 'scripts/deploy.ps1',
          summary: 'A Windows version of deploy.sh',
          by: 'pattern',
          added: true,
          lines: [
            { kind: 'add', text: '$ErrorActionPreference = "Stop"' },
            { kind: 'add', text: 'npm run build' },
            { kind: 'add', text: 'scp -r dist/* "${env:DEPLOY_HOST}:/srv/shop/"' },
          ],
        },
        {
          id: 'url',
          file: 'server/index.js',
          summary: 'Say when CATALOG_URL is missing',
          by: 'pattern',
          lines: [
            { kind: 'add', text: "const catalog = process.env.CATALOG_URL ?? 'http://localhost:4000'" },
            { kind: 'del', text: '  const upstream = await fetch(`${process.env.CATALOG_URL}/items`)' },
            { kind: 'add', text: '  const upstream = await fetch(new URL(\'/items\', catalog))' },
          ],
        },
      ],
      checks: [
        { label: 'Install packages', state: 'proved', detail: 'npm ci, 312 packages, 24 s' },
        { label: 'Build', state: 'proved', detail: 'npm run build made dist/ (1.1 MB)' },
        { label: 'Run the tests', state: 'proved', detail: '22 of 22 passed, same as on {there}' },
        { label: 'Start the server', state: 'proved', detail: 'http://localhost:3000 answered' },
        { label: 'Deploy', state: 'pending', detail: 'Needs DEPLOY_HOST and a server' },
      ],
      repair: {
        attempt: 1,
        of: 3,
        problem: 'None: every check passed.',
        trying: '',
        earlier: [],
        log: [],
        spentHere: 0,
      },
      consent: {
        command: 'npm run start',
        from: 'package.json, "start"',
        why: `${APP_NAME} starts the server on {here} to check it answers, then stops it.`,
        aiFiles: [],
        aiProblem: '',
        neverSent: ['.env', 'node_modules', 'dist'],
      },
      final: {
        headline: 'Runs on Windows.',
        sub: 'shop-web is in C:\\Users\\Sam\\Code\\shop-web. 4 changes, each one undoable.',
        results: [
          { state: 'proved', title: 'Installs on Windows', detail: '312 packages from package-lock.json.' },
          { state: 'proved', title: 'Builds', detail: 'npm run build, with the new cross-platform scripts.' },
          { state: 'proved', title: 'Tests pass', detail: '22 of 22, the same as on {there}.' },
          { state: 'proved', title: 'The server starts', detail: 'http://localhost:3000 served the storefront.' },
          {
            state: 'couldntCheck',
            title: 'Deploying',
            detail: 'deploy.ps1 needs DEPLOY_HOST and access to your server, so it wasn’t run.',
          },
        ],
        plan: '# shop-web on Windows\n\n1. `npm ci`, then `npm run dev`.\n2. To deploy, set `DEPLOY_HOST` and run `scripts\\deploy.ps1`.\n',
      },
    },
    rebuild: null,
  },
  {
    id: 'ledger-tool',
    label: 'Windows Python tool · PC → Mac',
    from: 'win',
    report: ledgerTool as Report,
    files: ['requirements.txt', 'ledger.py', 'start.bat', 'tools/fetch_rates.sh', 'tools/fetch_rates.ps1', 'README.md'],
    convert: {
      forTheAi: 'Find the output folder without the registry',
      done: {
        snapshot: 'Python, 2 packages, a command-line tool that reads the registry',
        pack: '8 files, 9 KB',
        plan: '5 fixes from known patterns, 1 for the AI (winreg)',
        apply: '6 changes in 4 files',
        verify: '3 checks proved, 1 failed, 1 couldn’t be checked',
        report: 'Done',
        repair: 'winreg replaced on the second try',
      },
      doing: {
        snapshot: 'Reading how ledger-tool runs on {there}',
        pack: 'Packing the project',
        plan: 'Matching 6 findings to fix patterns',
        apply: 'Converting line endings in tools/fetch_rates.sh',
        verify: 'Running ledger.py export on {here} with sample files',
        report: 'Writing what was proved',
        repair: 'Replacing the registry lookup',
      },
      changes: [
        {
          id: 'winreg',
          file: 'ledger.py',
          summary: 'Read the output folder from settings, not the registry',
          by: 'ai',
          lines: [
            { kind: 'del', text: 'import winreg' },
            { kind: 'add', text: 'import json' },
            { kind: 'add', text: 'from pathlib import Path' },
            { kind: 'ctx', text: '' },
            { kind: 'del', text: '    key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\\Microsoft\\Office\\16.0\\Excel\\Options")' },
            { kind: 'del', text: '    value, _ = winreg.QueryValueEx(key, "DefaultPath")' },
            { kind: 'del', text: '    return value' },
            { kind: 'add', text: '    settings = json.loads(Path("settings.json").read_text(encoding="utf-8"))' },
            { kind: 'add', text: '    return settings["output_folder"]' },
          ],
        },
        {
          id: 'exports',
          file: 'ledger.py',
          summary: 'Find bank exports in the home folder',
          by: 'pattern',
          lines: [
            { kind: 'del', text: 'EXPORTS = "C:\\\\Users\\\\sam\\\\Documents\\\\Bank exports"' },
            { kind: 'add', text: 'EXPORTS = Path.home() / "Documents" / "Bank exports"' },
            { kind: 'del', text: '    with open(EXPORTS + "\\\\" + month + ".csv", newline="", encoding="utf-8") as f:' },
            { kind: 'add', text: '    with open(EXPORTS / f"{month}.csv", newline="", encoding="utf-8") as f:' },
          ],
        },
        {
          id: 'startfile',
          file: 'ledger.py',
          summary: 'Open the workbook with the Mac’s open command',
          by: 'pattern',
          lines: [
            { kind: 'del', text: '    os.startfile(target)' },
            { kind: 'add', text: '    subprocess.run(["open", target], check=False)' },
          ],
        },
        {
          id: 'settings',
          file: 'settings.json',
          summary: 'Where ledgers are saved',
          by: 'ai',
          added: true,
          lines: [
            { kind: 'add', text: '{' },
            { kind: 'add', text: '  "output_folder": "~/Documents/Ledgers"' },
            { kind: 'add', text: '}' },
          ],
        },
        {
          id: 'crlf',
          file: 'tools/fetch_rates.sh',
          summary: 'Mac line endings',
          by: 'pattern',
          lines: [
            { kind: 'del', text: '#!/bin/sh␍' },
            { kind: 'add', text: '#!/bin/sh' },
          ],
        },
        {
          id: 'start',
          file: 'start.command',
          summary: 'A Mac version of start.bat',
          by: 'pattern',
          added: true,
          lines: [
            { kind: 'add', text: '#!/bin/sh' },
            { kind: 'add', text: 'cd "$(dirname "$0")"' },
            { kind: 'add', text: 'python3 ledger.py export --month "$1"' },
          ],
        },
      ],
      checks: [
        { label: 'Install 2 packages', state: 'proved', detail: 'openpyxl, rich' },
        { label: 'Export a month from sample files', state: 'proved', detail: 'Made Ledger 2026-09.xlsx (3 sample rows)' },
        { label: 'Open the workbook', state: 'proved', detail: 'open handed it to Numbers' },
        { label: 'Download exchange rates', state: 'failed', detail: 'example.org answered 404, on Windows too' },
        { label: 'Your real bank exports', state: 'pending', detail: 'Not on {here}' },
      ],
      repair: {
        attempt: 2,
        of: 3,
        problem: 'ledger.py imports winreg, which doesn’t exist on a Mac, to find where Excel saves files.',
        trying: 'Read the folder from a settings.json next to ledger.py, with ~/Documents/Ledgers to start',
        earlier: [
          {
            attempt: 1,
            tried: 'Read Excel’s default folder from its Mac preferences',
            result: 'Excel isn’t installed on {here}, so the check couldn’t pass. Undone.',
          },
        ],
        log: ['read ledger.py', 'winreg used once: output_folder()', 'edit ledger.py, add settings.json', 'run: python3 ledger.py export --month 2026-09'],
        spentHere: 0.11,
      },
      consent: {
        command: 'python3 ledger.py export --month 2026-09',
        from: 'README.md, line 9',
        why: `${APP_NAME} runs the export on {here} with 3 sample rows, to check it makes the workbook.`,
        aiFiles: [
          { path: 'ledger.py', bytes: 1_380 },
          { path: 'requirements.txt', bytes: 30 },
          { path: 'README.md', bytes: 190 },
        ],
        aiProblem: 'ledger.py reads Excel’s save folder from the Windows registry.',
        neverSent: ['your bank exports', 'anything outside the project'],
      },
      final: {
        headline: 'Runs on the Mac. 1 thing needs you.',
        sub: 'ledger-tool is in ~/Projects/ledger-tool. 6 changes, each one undoable.',
        results: [
          { state: 'proved', title: 'Installs on the Mac', detail: 'openpyxl and rich, from requirements.txt.' },
          { state: 'proved', title: 'Exports a month', detail: 'Made a workbook from 3 sample rows.' },
          { state: 'proved', title: 'Opens the workbook', detail: 'With the Mac’s open command, in Numbers.' },
          {
            state: 'failed',
            title: 'Downloading exchange rates',
            detail: 'example.org answered 404. It fails on Windows too, so the move didn’t cause it.',
          },
          {
            state: 'needsYou',
            title: 'Choose where ledgers are saved',
            detail: 'The registry setting doesn’t exist on a Mac. It’s ~/Documents/Ledgers in settings.json for now.',
          },
          {
            state: 'couldntCheck',
            title: 'Your real bank exports',
            detail: 'They’re on {there}, not in the project. Sample files were used instead.',
          },
        ],
        plan: '# ledger-tool on the Mac\n\n1. Check `output_folder` in `settings.json`.\n2. Copy your bank exports to `~/Documents/Bank exports`.\n3. Run `./start.command 2026-10`.\n',
      },
    },
    rebuild: null,
  },
  {
    id: 'sketchbook',
    label: 'Xcode app · Mac → Windows',
    from: 'mac',
    report: sketchbook as Report,
    files: ['Sketchbook.xcodeproj/project.pbxproj', 'Sketchbook/SketchbookApp.swift', 'Sketchbook/CanvasView.swift', 'Sketchbook/SketchDocument.swift', 'Sketchbook/Export.swift'],
    convert: null,
    rebuild: {
      summary:
        'Sketchbook is a SwiftUI app that draws with PencilKit. Both exist only on Apple systems, so there is no version of this code that runs on Windows. The ideas carry over; the interface has to be built again.',
      blockers: [
        { title: 'SwiftUI interface', reason: '4 files. SwiftUI only runs on Apple systems.' },
        { title: 'PencilKit drawing', reason: 'The canvas, tools and saved drawings use PencilKit, which Windows doesn’t have.' },
        { title: 'AppKit export', reason: 'Export.swift makes PNGs with AppKit (NSBitmapImageRep).' },
        { title: 'Xcode project', reason: 'Sketchbook.xcodeproj builds with Xcode, which only runs on a Mac.' },
      ],
      keeps: [
        'What the app does: documents, a canvas, pen tools, PNG export',
        'The test (ExportTests.swift) as a description of what export must do',
        'Your drawings, if exported as PNG first: .sketch files are PencilKit data',
      ],
      steps: [
        { title: 'Choose the Windows toolkit', detail: 'WinUI 3 with Windows Ink is closest to PencilKit. Avalonia or Flutter if you want one codebase for both.' },
        { title: 'Rebuild the document model', detail: 'Strokes, colours and widths in a format both versions can read (JSON or SVG).' },
        { title: 'Rebuild the canvas', detail: 'InkCanvas with pen, marker and eraser, pressure from the pen.' },
        { title: 'Rebuild export', detail: 'Render the strokes to PNG; port ExportTests as the first test.' },
        { title: 'Move existing drawings', detail: 'Export them as PNG on the Mac, or write a one-time converter.' },
      ],
    },
  },
  {
    id: 'invoicedesk',
    label: 'WPF app · PC → Mac',
    from: 'win',
    report: invoiceDesk as Report,
    files: ['InvoiceDesk.sln', 'InvoiceDesk/InvoiceDesk.csproj', 'InvoiceDesk/MainWindow.xaml', 'InvoiceDesk/MainWindow.xaml.cs', 'InvoiceDesk/Services/Printing.cs'],
    convert: null,
    rebuild: {
      summary:
        'InvoiceDesk is a WPF app: its windows are XAML that only Windows can draw, and it targets net8.0-windows. The invoice logic is plain C# and can come along; the interface has to be built again.',
      blockers: [
        { title: 'WPF interface', reason: '2 windows in XAML. WPF only runs on Windows.' },
        { title: 'Registry settings', reason: 'MainWindow reads the company name from the Windows registry.' },
        { title: 'Windows printing', reason: 'Services/Printing.cs uses WPF’s PrintDialog.' },
        { title: 'Windows-only target', reason: 'net8.0-windows builds only on Windows.' },
      ],
      keeps: [
        'Services/Totals.cs: plain C#, runs on .NET 8 on a Mac as it is',
        'The tests in InvoiceDesk.Tests',
        'QuestPDF, which works on macOS, for invoices as PDFs',
      ],
      steps: [
        { title: 'Choose the toolkit', detail: 'Avalonia is closest to WPF (XAML and the same patterns). .NET MAUI if you want the Mac’s own look.' },
        { title: 'Retarget to net8.0', detail: 'Move Services/ into a class library both versions share.' },
        { title: 'Move settings out of the registry', detail: 'A settings.json in the user’s app-data folder works on both.' },
        { title: 'Rebuild the 2 windows', detail: 'The invoice list and the invoice form, in Avalonia XAML.' },
        { title: 'Replace printing', detail: 'Make PDFs with QuestPDF and let the Mac print them.' },
      ],
    },
  },
  {
    id: 'handbook',
    label: 'Folder of documents · Mac → Windows',
    from: 'mac',
    report: handbook as Report,
    files: ['README.md', '01-welcome.md', '02-meetings.md', '03-expenses.md', 'forms/expense-policy.pdf', 'images/office-map.png', 'site/index.html'],
    convert: null,
    rebuild: null,
  },
]

export function scenarioById(id: string | null): Scenario {
  return scenarios.find((s) => s.id === id) ?? scenarios[0]
}
