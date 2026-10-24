// A reply cut into what each stage of the pipeline owns: fences another
// plugin draws, status tag lines, and the markdown runs between them.

import { type Segment, split } from '../badges/parse'

export type Part = Segment | { kind: 'foreign'; text: string; gap: boolean }

/** Fence languages the mermaid plugin draws: handed down the chain exactly as written. */
export const FOREIGN = new Set(['mermaid', 'zenuml'])

const OPEN = /^ {0,3}(`{3,}|~{3,})\s*([^`\s]*)/
const CLOSE = /^ {0,3}(`{3,}|~{3,})\s*$/

const closes = (line: string, mark: string) => {
  const close = CLOSE.exec(line)?.[1]
  return close !== undefined && close[0] === mark[0] && close.length >= mark.length
}

/** The reply's parts in order; a foreign fence still open runs to the end, as it streams. */
export function partsOf(text: string, marker: RegExp): Part[] {
  const out: Part[] = []
  const lines = text.split('\n')
  let run: string[] = []

  const flush = () => {
    if (run.length > 0) out.push(...split(run.join('\n'), marker))
    run = []
  }

  let i = 0
  while (i < lines.length) {
    const line = lines[i]!
    const open = OPEN.exec(line)
    if (!open) {
      run.push(line)
      i += 1
      continue
    }
    let end = i + 1
    while (end < lines.length && !closes(lines[end]!, open[1]!)) end += 1
    end = Math.min(end + 1, lines.length)
    if (FOREIGN.has(open[2]!.toLowerCase())) {
      const gap = run.length > 0 && !run[run.length - 1]!.trim()
      flush()
      out.push({ kind: 'foreign', text: lines.slice(i, end).join('\n'), gap })
    } else {
      run.push(...lines.slice(i, end))
    }
    i = end
  }

  flush()
  return out
}
