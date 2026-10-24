// Inline nodes drawn with the surface's own elements: Text runs for every
// style, color literals on their own color, links as the theme's layout says.

import type { ElementTable, RenderChildren } from 'claude-code'

import { swatched } from '../swatches/swatch'
import { type GradStops, type Inline, plainOf } from './inlines'
import { CONFIG, paint } from './native'
import {
  GRADIENTS,
  RAINBOW,
  ROLE_COLORS,
  THEMES,
  type Palette,
  paletteFor,
  resolveColor,
  shadeFor,
  spread,
  toneColor,
} from './themes'

type Table = Pick<ElementTable, 'Box' | 'Text' | 'Link' | 'Code' | 'Markdown'>

/** The text styles a span passes on to the spans inside it. */
type Marks = { bold?: boolean; italic?: boolean; underline?: boolean; strikethrough?: boolean }

export type Ctx = {
  t: Table
  p: Palette
  isLight: boolean
  /** Columns the block draws into. */
  width: number
  /** How deep in lists, for the bullet glyph and color. */
  listDepth: number
  /** Inside a blockquote: paragraphs draw italic. */
  isQuoted: boolean
  marks?: Marks
}

/** A Text string's bound is 10000; runs are cut well below it. */
const CHUNK = 4000

export const isNative = (ctx: Ctx) => ctx.p.layout === 'native'

/** Text as a Text may hold it: no escapes or control characters, tabs as spaces. */
export function clean(text: string): string {
  return text
    .replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, '')
    .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g, '')
    .replace(/\t/g, '    ')
    .replace(/[\x00-\x09\x0b-\x1f\x7f]/g, '')
}

function chunks(text: string): string[] {
  const out: string[] = []
  for (let i = 0; i < text.length; i += CHUNK) out.push(text.slice(i, i + CHUNK))
  return out.length > 0 ? out : ['']
}

/** Plain text with its color literals drawn as swatches. */
export function textRun(text: string, ctx: Ctx): RenderChildren[] {
  return swatched(clean(text), ctx.t, chunks)
}

/** Props with the unset ones left out, as a tree's props must be. */
export function defined<T extends Record<string, unknown>>(props: T): T {
  const out: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(props)) if (v !== undefined && v !== false) out[k] = v
  return out as T
}

/** Terminal cells a string takes: wide East Asian and emoji count two, marks none. */
export function cellWidth(text: string): number {
  let n = 0
  for (const ch of text) {
    const c = ch.codePointAt(0)!
    if ((c >= 0x300 && c <= 0x36f) || (c >= 0x200b && c <= 0x200f) || c === 0xfe0f || c === 0x20e3) continue
    if (
      (c >= 0x1100 && c <= 0x115f) ||
      (c >= 0x2e80 && c <= 0xa4cf) ||
      (c >= 0xac00 && c <= 0xd7a3) ||
      (c >= 0xf900 && c <= 0xfaff) ||
      (c >= 0xfe30 && c <= 0xfe4f) ||
      (c >= 0xff00 && c <= 0xff60) ||
      (c >= 0xffe0 && c <= 0xffe6) ||
      (c >= 0x1f300 && c <= 0x1f64f) ||
      (c >= 0x1f900 && c <= 0x1f9ff) ||
      (c >= 0x20000 && c <= 0x3fffd)
    ) {
      n += 2
    } else {
      n += 1
    }
  }
  return n
}

/** A fixed color as the theme draws it: toned to the theme, then shaded for the background. */
function themed(hex: string, ctx: Ctx): string {
  return shadeFor(toneColor(hex, ctx.p.tone), ctx.isLight)
}

/** A raw color as drawn here: a role from the palette, anything else themed. */
export function colorOf(raw: string | undefined, ctx: Ctx): string | undefined {
  if (raw === undefined) return undefined
  if (ROLE_COLORS[raw.trim().toLowerCase()] !== undefined) return resolveColor(raw, ctx.p)
  const hex = resolveColor(raw, ctx.p)
  return hex === undefined ? undefined : themed(hex, ctx)
}

/** Dark or light text, whichever reads on `bg`. */
function contrast(bg: string): string {
  const n = parseInt(bg.slice(1), 16)
  const lum = 0.299 * ((n >> 16) & 255) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255)
  return lum > 140 ? '#1c1c1c' : '#ffffff'
}

/** `text` colored along `stops`, in at most `segments` runs. */
export function graded(text: string, stops: readonly string[], ctx: Ctx, segments = 48): RenderChildren[] {
  const { Text } = ctx.t
  const chars = Array.from(text)
  if (chars.length === 0) return []
  const size = Math.max(1, Math.ceil(chars.length / segments))
  const runs: string[] = []
  for (let i = 0; i < chars.length; i += size) runs.push(chars.slice(i, i + size).join(''))
  const colors = spread(stops, runs.length)
  return runs.map((run, k) => <Text color={colors[k]!}>{run}</Text>)
}

/** A gradient's colors as drawn: a theme's title stops, or each stop themed; the theme's own when none resolve. */
function stopsOf(stops: GradStops, ctx: Ctx): readonly string[] {
  if (stops === 'rainbow') return RAINBOW.map(c => themed(c, ctx))
  if (!Array.isArray(stops)) {
    const name = (stops as { theme?: string }).theme?.toLowerCase()
    return name !== undefined && THEMES[name] !== undefined ? paletteFor(name, ctx.isLight).title : ctx.p.title
  }
  const out: string[] = []
  for (const stop of stops as readonly string[]) {
    const preset = GRADIENTS[stop.toLowerCase()]
    if (preset !== undefined) out.push(...preset.map(c => themed(c, ctx)))
    else {
      const color = colorOf(stop, ctx)
      if (color !== undefined) out.push(color)
    }
  }
  return out.length > 0 ? out : ctx.p.title
}

/** The href a Link accepts (`https:` or local `http:`, printable ASCII, no `@`), or undefined. */
function safeHref(href: string): string | undefined {
  try {
    const url = new URL(href)
    const isLocal = url.protocol === 'http:' && (url.hostname === 'localhost' || url.hostname === '127.0.0.1')
    if (url.protocol !== 'https:' && !isLocal) return undefined
    const spelled = url.href
    if (spelled.length > 2048 || !/^[\x21-\x7e]+$/.test(spelled) || spelled.includes('@')) return undefined
    return spelled
  } catch {
    return undefined
  }
}

export function inlines(nodes: readonly Inline[], ctx: Ctx, hasColor = false): RenderChildren[] {
  return nodes.map(n => inline(n, ctx, hasColor))
}

/** The native theme's link: the text, then the address dim unless the text already is it; images as written. */
function nativeLink(n: Extract<Inline, { kind: 'link' }>, ctx: Ctx, hasColor: boolean): RenderChildren {
  const { Text } = ctx.t
  if (n.image !== undefined) return chunks(clean(n.image))
  const label = inlines(n.children, ctx, hasColor)
  if (n.href === '' || plainOf(n.children) === n.href) return label
  return [...label, <Text {...paint(CONFIG.styles.linkUrl)}>{` (${clean(n.href)})`}</Text>]
}

function inline(n: Inline, ctx: Ctx, hasColor: boolean): RenderChildren {
  const { Text, Link } = ctx.t
  const { p } = ctx
  switch (n.kind) {
    case 'text':
      return textRun(n.text, ctx)
    case 'br':
      return '\n'
    case 'code':
      return <Text {...defined({ color: p.code, backgroundColor: p.codeBg })}>{textRun(n.text, ctx)}</Text>
    case 'kbd': {
      const bg = ctx.isLight ? '#e0e0e0' : '#4a4a5a'
      return (
        <Text backgroundColor={bg} color={contrast(bg)} bold>
          {` ${clean(n.text)} `}
        </Text>
      )
    }
    case 'link': {
      if (isNative(ctx)) return nativeLink(n, ctx, hasColor)
      const label = n.children.length > 0 ? inlines(n.children, ctx, true) : [clean(n.href)]
      const styled = (
        <Text color={p.link} underline>
          {label}
        </Text>
      )
      const href = safeHref(n.href)
      if (href !== undefined) return <Link href={href}>{styled}</Link>
      const shown = plainOf(n.children)
      if (n.href === '' || shown === n.href) return styled
      return (
        <Text>
          {styled}
          <Text color={p.muted}>{` (${clean(n.href)})`}</Text>
        </Text>
      )
    }
    case 'span': {
      const s = n.style
      let color = colorOf(s.color, ctx)
      let bg = colorOf(s.bg, ctx)
      if (s.role === 'highlight') {
        bg = bg ?? p.highlight
        color = color ?? contrast(bg)
      }
      if (color === undefined && !hasColor) {
        if (s.role === 'bold') color = p.bold
        else if (s.role === 'italic') color = p.italic
        else if (s.role === 'strike' && !isNative(ctx)) color = p.muted
      }
      const marks: Marks = {
        bold: s.bold ?? ctx.marks?.bold,
        italic: s.italic ?? ctx.marks?.italic,
        underline: s.underline ?? ctx.marks?.underline,
        strikethrough: s.strike ?? ctx.marks?.strikethrough,
      }
      const props = defined({
        color,
        backgroundColor: bg,
        ...marks,
        dimColor: s.dim,
        inverse: s.inverse,
      })
      return <Text {...props}>{inlines(n.children, { ...ctx, marks }, hasColor || color !== undefined)}</Text>
    }
    case 'grad':
      return <Text>{graded(clean(plainOf(n.children)), stopsOf(n.stops, ctx), ctx)}</Text>
    case 'badge': {
      const bg = colorOf(n.color, ctx) ?? p.accent
      return (
        <Text backgroundColor={bg} color={contrast(bg)} bold>
          {' '}
          {inlines(n.children, ctx, true)}{' '}
        </Text>
      )
    }
  }
}
