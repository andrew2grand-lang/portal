import type { ScanProgress } from '../report'
import type { ResultState } from '../scenarios'

/** The scanning screen's steps, in the mission's words */
export function scanLabels(target: string): Record<ScanProgress['step'], string> {
  return {
    reading: 'Reading files',
    ecosystems: 'Finding ecosystems',
    code: `Checking for ${target}-only code`,
    packages: 'Checking packages',
    report: 'Writing the report',
  }
}

export const resultName: Record<ResultState, string> = {
  proved: 'Proved',
  failed: 'Failed',
  couldntCheck: 'Couldn’t check',
  needsYou: 'Needs you',
}
