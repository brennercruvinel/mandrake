// Markdown inlines plus the markup tags, parsed to the nodes the renderer draws.

import { NAMED_COLORS } from './themes'

export type Style = {
  /** A raw color as written: a name, a role (`ok`), `#hex` or `rgb()`. */
  color?: string
  bg?: string
  bold?: boolean
  italic?: boolean
  underline?: boolean
  strike?: boolean
  dim?: boolean
  inverse?: boolean
  /** What markdown made it, so the palette colors it when no tag chose a color. */
  role?: 'bold' | 'italic' | 'strike' | 'highlight'
}

export type Inline =
  | { kind: 'text'; text: string }
  | { kind: 'code'; text: string }
  /** `image` keeps an image as written, for a layout that leaves images as text. */
  | { kind: 'link'; href: string; children: Inline[]; image?: string }
  | { kind: 'span'; style: Style; children: Inline[] }
  | { kind: 'grad'; stops: GradStops; children: Inline[] }
  | { kind: 'badge'; color?: string; children: Inline[] }
  | { kind: 'kbd'; text: string }
  | { kind: 'br' }

/**
 * A gradient's colors: stops as written (names, roles, hex or presets), the
 * rainbow, or a theme's own title gradient (`{}` for the theme in force).
 */
export type GradStops = readonly string[] | 'rainbow' | { theme?: string }

/** A tag's attributes: `name=value` pairs, and the bare words. */
export function parseAttrs(raw: string): { named: Record<string, string>; bare: string[] } {
  const named: Record<string, string> = {}
  const bare: string[] = []
  const re = /([a-zA-Z#][\w#-]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|\{\s*["']?([^}"']*)["']?\s*\}|([^\s"'>]+)))?/g
  for (const m of raw.matchAll(re)) {
    const name = m[1]!
    const value = m[2] ?? m[3] ?? m[4] ?? m[5]
    if (value === undefined) bare.push(name)
    else named[name.toLowerCase()] = value
  }
  return { named, bare }
}

type TagHead =
  | { kind: 'span'; style: Style }
  | { kind: 'grad'; stops: GradStops }
  | { kind: 'badge'; color?: string }

type TagSpec = (named: Record<string, string>, bare: string[]) => TagHead

const FLAGS: Record<string, keyof Style> = {
  bold: 'bold',
  b: 'bold',
  italic: 'italic',
  i: 'italic',
  underline: 'underline',
  u: 'underline',
  strike: 'strike',
  strikethrough: 'strike',
  s: 'strike',
  dim: 'dim',
  dimcolor: 'dim',
  inverse: 'inverse',
  inv: 'inverse',
}

const isTrue = (v: string | undefined) => v === undefined || !/^(false|0|no|off)$/i.test(v)

/** A `<t>` / `<Text>` tag's attributes as a Style, Ink's prop names and the markup shorthands alike. */
function styleOf(named: Record<string, string>, bare: string[]): Style {
  const style: Style = {}
  for (const word of bare) {
    const flag = FLAGS[word.toLowerCase()]
    if (flag !== undefined) (style as Record<string, unknown>)[flag] = true
    else if (style.color === undefined) style.color = word
    else if (style.bg === undefined) style.bg = word
  }
  for (const [key, value] of Object.entries(named)) {
    if (key === 'color' || key === 'fg' || key === 'foreground') style.color = value
    else if (key === 'bg' || key === 'background' || key === 'backgroundcolor') style.bg = value
    else {
      const flag = FLAGS[key]
      if (flag !== undefined) (style as Record<string, unknown>)[flag] = isTrue(value)
    }
  }
  return style
}

const span = (style: Style): TagHead => ({ kind: 'span', style })

const STYLE_TAGS: Record<string, TagSpec> = {
  t: (n, b) => span(styleOf(n, b)),
  text: (n, b) => span(styleOf(n, b)),
  style: (n, b) => span(styleOf(n, b)),
  c: (n, b) => span(styleOf(n, b)),
  color: (n, b) => span(styleOf(n, b)),
  fg: (n, b) => span(styleOf(n, b)),
  bg: (n, b) => span({ ...styleOf(n, []), bg: n.color ?? b[0] }),
  b: () => span({ bold: true }),
  bold: () => span({ bold: true }),
  strong: () => span({ bold: true }),
  i: () => span({ italic: true }),
  em: () => span({ italic: true }),
  italic: () => span({ italic: true }),
  u: () => span({ underline: true }),
  underline: () => span({ underline: true }),
  ins: () => span({ underline: true }),
  s: () => span({ strike: true }),
  strike: () => span({ strike: true }),
  del: () => span({ strike: true }),
  dim: () => span({ dim: true }),
  faint: () => span({ dim: true }),
  inv: () => span({ inverse: true }),
  inverse: () => span({ inverse: true }),
  hl: (n, b) => span({ role: 'highlight', bg: n.color ?? b[0] }),
  mark: (n, b) => span({ role: 'highlight', bg: n.color ?? b[0] }),
  highlight: (n, b) => span({ role: 'highlight', bg: n.color ?? b[0] }),
  ok: () => span({ color: 'ok' }),
  success: () => span({ color: 'ok' }),
  warn: () => span({ color: 'warn' }),
  warning: () => span({ color: 'warn' }),
  err: () => span({ color: 'err' }),
  error: () => span({ color: 'err' }),
  info: () => span({ color: 'info' }),
  note: () => span({ color: 'info' }),
  accent: () => span({ color: 'accent' }),
  muted: () => span({ color: 'muted' }),
  c1: () => span({ color: 'c1' }),
  c2: () => span({ color: 'c2' }),
  c3: () => span({ color: 'c3' }),
  c4: () => span({ color: 'c4' }),
  c5: () => span({ color: 'c5' }),
  rainbow: () => ({ kind: 'grad', stops: 'rainbow' }),
  grad: (n, b) => ({ kind: 'grad', stops: gradStops(n, b) }),
  gradient: (n, b) => ({ kind: 'grad', stops: gradStops(n, b) }),
  badge: (n, b) => ({ kind: 'badge', color: n.color ?? n.bg ?? b[0] }),
  pill: (n, b) => ({ kind: 'badge', color: n.color ?? n.bg ?? b[0] }),
}

/** A `<grad>` tag's stops; none, or `theme=<name>`, takes a theme's own gradient. */
function gradStops(named: Record<string, string>, bare: string[]): GradStops {
  if (named.theme !== undefined) return { theme: named.theme }
  const stops = [named.from, ...bare, ...(named.via ? [named.via] : []), named.to].filter((s): s is string => !!s)
  return stops.length > 0 ? stops : {}
}

function tagSpec(name: string): TagSpec | undefined {
  const lower = name.toLowerCase()
  const spec = STYLE_TAGS[lower]
  if (spec !== undefined) return spec
  // A known color names a span of itself: `<orange>`, `<#ff8800>`.
  if (/^#[0-9a-f]{3}([0-9a-f]{3})?$/i.test(name) || NAMED_COLORS[lower] !== undefined) {
    return (n, b) => span({ ...styleOf(n, b), color: name })
  }
  return undefined
}

/** Whether `<name …>` is one of the markup tags (`kbd` and `br` included). */
export function isTag(name: string): boolean {
  const lower = name.toLowerCase()
  return lower === 'kbd' || lower === 'key' || lower === 'br' || tagSpec(name) !== undefined
}

const OPEN_TAG = /^<([a-zA-Z#][\w#-]*)((?:\s+[^<>]*?)?)\s*(\/?)>/
const CLOSE_TAG = /^<\/([a-zA-Z#][\w#-]*)?\s*>/
const URL_RE = /^https?:\/\/[^\s<>"'`]+/
const PUNCT = /[!"#$%&'()*+,\-./:;<=>?@[\\\]^_`{|}~]/

/** The index past a code span opening at `i`, or -1 when it never closes. */
function codeSpanEnd(s: string, i: number): [number, number] {
  let n = 0
  while (s[i + n] === '`') n += 1
  let j = i + n
  while (j < s.length) {
    const k = s.indexOf('`'.repeat(n), j)
    if (k === -1) return [-1, n]
    let run = 0
    while (s[k + run] === '`') run += 1
    if (run === n) return [k, n]
    j = k + run
  }
  return [-1, n]
}

/** Where the close of the tag opened before `from` starts and ends; the string's end when it is never closed. */
function findClose(s: string, from: number, name: string): [number, number] {
  const stack: string[] = []
  let i = from
  while (i < s.length) {
    const c = s[i]
    if (c === '\\') {
      i += 2
      continue
    }
    if (c === '`') {
      const [end, n] = codeSpanEnd(s, i)
      i = end === -1 ? i + n : end + n
      continue
    }
    if (c === '<') {
      const rest = s.slice(i)
      const close = CLOSE_TAG.exec(rest)
      if (close) {
        const closing = close[1]?.toLowerCase()
        // Forgiving: any close tag of ours ends the innermost open tag, so a
        // mismatched `<magenta>x</cyan>` stops where it was meant to.
        const isOurs = closing === undefined || closing === name || isTag(closing)
        if (stack.length === 0 && isOurs) return [i, i + close[0].length]
        if (stack.length > 0 && isOurs) {
          const at = closing === undefined ? -1 : stack.lastIndexOf(closing)
          stack.length = at !== -1 ? at : stack.length - 1
        }
        i += close[0].length
        continue
      }
      const open = OPEN_TAG.exec(rest)
      if (open && isTag(open[1]!) && open[3] !== '/' && !/^(br|kbd|key)$/i.test(open[1]!)) {
        stack.push(open[1]!.toLowerCase())
        i += open[0].length
        continue
      }
    }
    i += 1
  }
  return [s.length, s.length]
}

/** The index of the delimiter run closing emphasis opened before `from`, or -1. */
function findEmphasisClose(s: string, from: number, delim: string): number {
  const ch = delim[0]!
  let i = from
  while (i < s.length) {
    const c = s[i]
    if (c === '\\') {
      i += 2
      continue
    }
    if (c === '`') {
      const [end, n] = codeSpanEnd(s, i)
      i = end === -1 ? i + n : end + n
      continue
    }
    if (c === ch) {
      let run = 0
      while (s[i + run] === ch) run += 1
      const before = s[i - 1] ?? ' '
      const after = s[i + run] ?? ' '
      const canClose = !/\s/.test(before) && (ch !== '_' || !/[\p{L}\p{N}]/u.test(after))
      if (canClose && run >= delim.length && (run === delim.length || run === 3 || delim.length === 3)) {
        // `***` closes `**` with a `*` left over and vice versa: take the matching part.
        return run > delim.length ? i + (run - delim.length) : i
      }
      i += run
      continue
    }
    i += 1
  }
  return -1
}

export function parseInlines(s: string): Inline[] {
  const out: Inline[] = []
  let buf = ''
  const flush = () => {
    if (buf !== '') {
      out.push({ kind: 'text', text: buf })
      buf = ''
    }
  }
  let i = 0
  while (i < s.length) {
    const c = s[i]!

    if (c === '\\' && i + 1 < s.length && PUNCT.test(s[i + 1]!)) {
      buf += s[i + 1]
      i += 2
      continue
    }

    if (c === '`') {
      const [end, n] = codeSpanEnd(s, i)
      if (end === -1) {
        buf += '`'.repeat(n)
        i += n
        continue
      }
      flush()
      let code = s.slice(i + n, end).replace(/\n/g, ' ')
      if (code.startsWith(' ') && code.endsWith(' ') && code.trim() !== '') code = code.slice(1, -1)
      out.push({ kind: 'code', text: code })
      i = end + n
      continue
    }

    if (c === '<') {
      const rest = s.slice(i)
      const auto = /^<(https?:\/\/[^\s<>]+)>/.exec(rest)
      if (auto) {
        flush()
        out.push({ kind: 'link', href: auto[1]!, children: [{ kind: 'text', text: auto[1]! }] })
        i += auto[0].length
        continue
      }
      const open = OPEN_TAG.exec(rest)
      if (open && isTag(open[1]!)) {
        const name = open[1]!.toLowerCase()
        const { named, bare } = parseAttrs(open[2] ?? '')
        const bodyStart = i + open[0].length
        if (name === 'br') {
          flush()
          out.push({ kind: 'br' })
          i = bodyStart
          continue
        }
        if (open[3] === '/') {
          i = bodyStart
          continue
        }
        const [closeStart, closeEnd] = findClose(s, bodyStart, name)
        const inner = s.slice(bodyStart, closeStart)
        flush()
        if (name === 'kbd' || name === 'key') {
          out.push({ kind: 'kbd', text: inner })
        } else {
          const spec = tagSpec(open[1]!)!
          out.push({ ...spec(named, bare), children: parseInlines(inner) })
        }
        i = closeEnd
        continue
      }
      // A stray close tag of ours draws as nothing.
      const close = CLOSE_TAG.exec(rest)
      if (close && (close[1] === undefined || isTag(close[1]))) {
        i += close[0].length
        continue
      }
    }

    if (c === '~' && s[i + 1] === '~' && s[i + 2] !== undefined && !/\s/.test(s[i + 2]!)) {
      const end = s.indexOf('~~', i + 2)
      if (end !== -1 && !/\s/.test(s[end - 1]!)) {
        flush()
        out.push({ kind: 'span', style: { strike: true, role: 'strike' }, children: parseInlines(s.slice(i + 2, end)) })
        i = end + 2
        continue
      }
    }

    if (c === '*' || c === '_') {
      let run = 0
      while (s[i + run] === c) run += 1
      const before = s[i - 1] ?? ' '
      const after = s[i + run] ?? ' '
      const canOpen = !/\s/.test(after) && (c !== '_' || !/[\p{L}\p{N}]/u.test(before))
      if (canOpen && run <= 3) {
        const delim = c.repeat(run)
        const end = findEmphasisClose(s, i + run, delim)
        if (end !== -1 && end > i + run) {
          flush()
          const children = parseInlines(s.slice(i + run, end))
          const style: Style =
            run === 1
              ? { italic: true, role: 'italic' }
              : run === 2
                ? { bold: true, role: 'bold' }
                : { bold: true, italic: true, role: 'bold' }
          out.push({ kind: 'span', style, children })
          i = end + run
          continue
        }
      }
      buf += c.repeat(run)
      i += run
      continue
    }

    if (c === '[' || (c === '!' && s[i + 1] === '[')) {
      const at = c === '!' ? i + 1 : i
      const link = linkAt(s, at)
      if (link) {
        flush()
        const children = parseInlines(link.text)
        if (c === '!') {
          children.unshift({ kind: 'text', text: '🖼 ' })
          out.push({ kind: 'link', href: link.href, children, image: s.slice(i, link.end) })
        } else {
          out.push({ kind: 'link', href: link.href, children })
        }
        i = link.end
        continue
      }
    }

    if ((c === 'h' || c === 'H') && !/[\p{L}\p{N}]/u.test(s[i - 1] ?? ' ')) {
      const url = URL_RE.exec(s.slice(i))
      if (url) {
        const href = url[0].replace(/[).,;:!?\]]+$/, '')
        flush()
        out.push({ kind: 'link', href, children: [{ kind: 'text', text: href }] })
        i += href.length
        continue
      }
    }

    buf += c
    i += 1
  }
  flush()
  return out
}

function linkAt(s: string, i: number): { text: string; href: string; end: number } | undefined {
  let depth = 0
  let j = i
  for (; j < s.length; j += 1) {
    const c = s[j]
    if (c === '\\') {
      j += 1
      continue
    }
    if (c === '[') depth += 1
    if (c === ']') {
      depth -= 1
      if (depth === 0) break
    }
  }
  if (depth !== 0 || s[j + 1] !== '(') return undefined
  let k = j + 2
  let parens = 1
  for (; k < s.length; k += 1) {
    if (s[k] === '(') parens += 1
    if (s[k] === ')') {
      parens -= 1
      if (parens === 0) break
    }
  }
  if (parens !== 0) return undefined
  const target = s.slice(j + 2, k).trim()
  const href = (/^<([^>]*)>/.exec(target)?.[1] ?? target.split(/\s+/)[0] ?? '').trim()
  return { text: s.slice(i + 1, j), href, end: k + 1 }
}

/** The text an inline tree reads as, tags and markup gone. */
export function plainOf(nodes: readonly Inline[]): string {
  return nodes
    .map(n => {
      switch (n.kind) {
        case 'text':
        case 'code':
        case 'kbd':
          return n.text
        case 'br':
          return '\n'
        default:
          return plainOf(n.children)
      }
    })
    .join('')
}
