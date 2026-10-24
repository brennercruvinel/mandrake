// Markdown blocks parsed to a small tree the renderer draws. Pure: no `$`,
// no palette, so the tests read it directly.

import { type Inline, parseAttrs, parseInlines } from './inlines'

export type Align = 'left' | 'center' | 'right'

export type CalloutKind = 'note' | 'tip' | 'important' | 'warning' | 'caution'

/** `marker` is the bullet or number as written. */
export type ListItem = { task: boolean | null; marker: string; blocks: Block[] }

export type Block =
  | { kind: 'heading'; level: number; inlines: Inline[] }
  /** `pipeRaw` holds a paragraph that opens with `|` but is no table (yet), as written. */
  | { kind: 'para'; inlines: Inline[]; pipeRaw?: string }
  | { kind: 'list'; ordered: boolean; start: number; isLoose: boolean; items: ListItem[] }
  /** `lines` are the quoted lines with their `>` taken off. */
  | { kind: 'quote'; blocks: Block[]; lines: string[] }
  | { kind: 'callout'; type: CalloutKind; title: Inline[]; blocks: Block[] }
  | { kind: 'panel'; title?: string; color?: string; border?: string; blocks: Block[] }
  /** `raw` is the fence as written; an unclosed one runs to the end. */
  | { kind: 'code'; lang: string; text: string; raw: string; isClosed: boolean }
  | { kind: 'table'; raw: string; align: Align[]; header: Inline[][]; rows: Inline[][][] }
  | { kind: 'hr' }

// ---------------------------------------------------------------- blocks

const FENCE = /^ {0,3}(`{3,}|~{3,})\s*([^\s`]*)[^`]*$/
const HEADING = /^ {0,3}(#{1,6})(?:\s+(.*?))?\s*#*\s*$/
const HR = /^ {0,3}([-*_])(?:[ \t]*\1){2,}[ \t]*$/
const QUOTE = /^ {0,3}>/
const LIST = /^( *)([-*+]|\d{1,9}[.)])(?:([ \t]+)(.*))?$/
const TABLE_SEP = /^\s*\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)*\|?\s*$/
const PIPE = /^\s*\|/
const PANEL_OPEN = /^\s*<panel\b([^>]*)>\s*$/i
const PANEL_CLOSE = /^\s*<\/panel\s*>\s*$/i
const ALERT = /^\s*\[!(note|tip|important|warning|caution)\]\s*(.*)$/i

const isBlank = (line: string) => line.trim() === ''

const indentOf = (line: string) => {
  let n = 0
  for (const c of line) {
    if (c === ' ') n += 1
    else if (c === '\t') n += 4 - (n % 4)
    else break
  }
  return n
}

/** The line with `n` columns of leading indentation taken off, tabs counted to 4. */
function dedent(line: string, n: number): string {
  let col = 0
  let i = 0
  while (i < line.length && col < n) {
    const c = line[i]
    if (c === ' ') col += 1
    else if (c === '\t') col += 4 - (col % 4)
    else break
    i += 1
  }
  return (col > n ? ' '.repeat(col - n) : '') + line.slice(i)
}

function startsBlock(line: string): boolean {
  return (
    FENCE.test(line) ||
    HEADING.test(line) ||
    QUOTE.test(line) ||
    HR.test(line) ||
    PANEL_OPEN.test(line) ||
    LIST.test(line)
  )
}

function splitRow(line: string): string[] {
  let body = line.trim()
  if (body.startsWith('|')) body = body.slice(1)
  if (body.endsWith('|') && !body.endsWith('\\|')) body = body.slice(0, -1)
  const cells: string[] = []
  let cell = ''
  let inCode = false
  for (let i = 0; i < body.length; i += 1) {
    const c = body[i]!
    if (c === '\\' && body[i + 1] === '|') {
      cell += '|'
      i += 1
    } else if (c === '`') {
      inCode = !inCode
      cell += c
    } else if (c === '|' && !inCode) {
      cells.push(cell.trim())
      cell = ''
    } else {
      cell += c
    }
  }
  cells.push(cell.trim())
  return cells
}

function alignOf(cell: string): Align {
  const c = cell.trim()
  if (c.startsWith(':') && c.endsWith(':')) return 'center'
  if (c.endsWith(':')) return 'right'
  return 'left'
}

export function parseBlocks(input: string): Block[] {
  const lines = input.replace(/\r\n?/g, '\n').split('\n')
  return blocksOf(lines)
}

function blocksOf(lines: readonly string[]): Block[] {
  const blocks: Block[] = []
  let i = 0
  while (i < lines.length) {
    const line = lines[i]!
    if (isBlank(line)) {
      i += 1
      continue
    }

    const fence = FENCE.exec(line)
    if (fence) {
      const mark = fence[1]!
      const body: string[] = []
      const indent = indentOf(line)
      const start = i
      let isClosed = false
      i += 1
      while (i < lines.length) {
        const l = lines[i]!
        const close = /^ {0,3}(`{3,}|~{3,})\s*$/.exec(l)
        if (close && close[1]![0] === mark[0] && close[1]!.length >= mark.length) {
          i += 1
          isClosed = true
          break
        }
        body.push(dedent(l, indent))
        i += 1
      }
      const raw = lines.slice(start, i).join('\n')
      blocks.push({ kind: 'code', lang: fence[2] ?? '', text: body.join('\n'), raw, isClosed })
      continue
    }

    const panel = PANEL_OPEN.exec(line)
    if (panel) {
      const { named, bare } = parseAttrs(panel[1] ?? '')
      const body: string[] = []
      let depth = 1
      i += 1
      while (i < lines.length) {
        const l = lines[i]!
        if (PANEL_OPEN.test(l)) depth += 1
        if (PANEL_CLOSE.test(l)) {
          depth -= 1
          if (depth === 0) {
            i += 1
            break
          }
        }
        body.push(l)
        i += 1
      }
      blocks.push({
        kind: 'panel',
        title: named.title,
        color: named.color ?? bare.find(b => !BORDERS.has(b.toLowerCase())),
        border: named.border ?? named.style ?? bare.find(b => BORDERS.has(b.toLowerCase())),
        blocks: blocksOf(body),
      })
      continue
    }

    const heading = HEADING.exec(line)
    if (heading) {
      blocks.push({ kind: 'heading', level: heading[1]!.length, inlines: parseInlines(heading[2] ?? '') })
      i += 1
      continue
    }

    if (HR.test(line)) {
      blocks.push({ kind: 'hr' })
      i += 1
      continue
    }

    if (QUOTE.test(line)) {
      const body: string[] = []
      while (i < lines.length && QUOTE.test(lines[i]!)) {
        body.push(lines[i]!.replace(/^ {0,3}> ?/, ''))
        i += 1
      }
      const alert = ALERT.exec(body[0] ?? '')
      if (alert) {
        const type = alert[1]!.toLowerCase() as CalloutKind
        blocks.push({ kind: 'callout', type, title: parseInlines(alert[2] ?? ''), blocks: blocksOf(body.slice(1)) })
      } else {
        blocks.push({ kind: 'quote', blocks: blocksOf(body), lines: body })
      }
      continue
    }

    const next = lines[i + 1]
    if (line.includes('|') && next !== undefined && TABLE_SEP.test(next) && next.includes('-')) {
      const header = splitRow(line)
      const align = splitRow(next).map(alignOf)
      const raw = [line, next]
      const rows: Inline[][][] = []
      i += 2
      while (i < lines.length && !isBlank(lines[i]!) && lines[i]!.includes('|')) {
        raw.push(lines[i]!)
        const cells = splitRow(lines[i]!)
        rows.push(header.map((_, c) => parseInlines(cells[c] ?? '')))
        i += 1
      }
      blocks.push({ kind: 'table', raw: raw.join('\n'), align, header: header.map(parseInlines), rows })
      continue
    }

    const item = LIST.exec(line)
    if (item) {
      const [list, after] = listAt(lines, i)
      blocks.push(list)
      i = after
      continue
    }

    // A paragraph: lines up to a blank or the start of another block; a
    // setext underline turns it into a heading.
    const body: string[] = [line.trim()]
    const start = i
    i += 1
    let level = 0
    while (i < lines.length) {
      const l = lines[i]!
      if (isBlank(l)) break
      if (/^ {0,3}=+\s*$/.test(l)) {
        level = 1
        i += 1
        break
      }
      if (/^ {0,3}-+\s*$/.test(l)) {
        level = 2
        i += 1
        break
      }
      if (startsBlock(l)) break
      body.push(l.trim())
      i += 1
    }
    const inlines = parseInlines(body.join('\n'))
    if (level > 0) blocks.push({ kind: 'heading', level, inlines })
    else if (PIPE.test(line)) blocks.push({ kind: 'para', inlines, pipeRaw: lines.slice(start, i).join('\n') })
    else blocks.push({ kind: 'para', inlines })
  }
  return blocks
}

const BORDERS = new Set(['round', 'single', 'double', 'bold', 'classic', 'singledouble', 'doublesingle'])

/** The list starting at line `start`, and the index of the line after it. */
function listAt(lines: readonly string[], start: number): [Block, number] {
  const first = LIST.exec(lines[start]!)!
  const ordered = /\d/.test(first[2]!)
  const baseIndent = indentOf(first[1]!)
  const items: ListItem[] = []
  let isLoose = false
  let i = start

  while (i < lines.length) {
    const m = LIST.exec(lines[i]!)
    if (!m || indentOf(m[1]!) !== baseIndent || /\d/.test(m[2]!) !== ordered) break
    const gap = m[3] ?? ' '
    const width = indentOf(m[1]!) + m[2]!.length + (gap.length > 4 ? 1 : Math.max(1, indentOf(gap)))
    const body: string[] = [m[4] ?? '']
    i += 1
    let sawBlank = false
    while (i < lines.length) {
      const l = lines[i]!
      if (isBlank(l)) {
        sawBlank = true
        body.push('')
        i += 1
        continue
      }
      if (indentOf(l) >= width) {
        body.push(dedent(l, width))
        sawBlank = false
        i += 1
        continue
      }
      // A line less indented than the item's content: a sibling, the end of
      // the list, or a lazy continuation of the item's last paragraph.
      if (!sawBlank && !startsBlock(l)) {
        body.push(l.trim())
        i += 1
        continue
      }
      break
    }
    while (body.length > 0 && isBlank(body[body.length - 1]!)) body.pop()
    if (sawBlank && i < lines.length && LIST.test(lines[i]!)) isLoose = true

    let task: boolean | null = null
    const box = /^\[([ xX])\][ \t]+/.exec(body[0] ?? '')
    if (box) {
      task = box[1] !== ' '
      body[0] = body[0]!.slice(box[0].length)
    }
    items.push({ task, marker: m[2]!, blocks: blocksOf(body) })
  }

  const start0 = ordered ? Number(/\d+/.exec(first[2]!)![0]) : 1
  return [{ kind: 'list', ordered, start: start0, isLoose, items }, i]
}

