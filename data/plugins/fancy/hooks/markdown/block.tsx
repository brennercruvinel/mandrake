// Blocks drawn with the surface's own elements: Boxes for layout, the
// engine's Code for fences, box lines for tables. The `native` layout
// (the native theme) draws its own blocks in native-block.tsx.

import type { RenderElement } from 'claude-code'

import { find } from '../swatches/colors'
import { colorLine } from '../swatches/swatch'
import type { Block, CalloutKind } from './blocks'
import { type Inline, plainOf } from './inlines'
import { type Ctx, cellWidth, clean, colorOf, graded, inlines, isNative } from './inline'
import { nativeBlock } from './native-block'
import { stripTags } from './stream'
import type { Palette } from './themes'

const BULLETS = ['•', '◦', '▸', '▪']
const BORDERS: Record<string, string> = {
  round: 'round',
  single: 'single',
  double: 'double',
  bold: 'bold',
  classic: 'classic',
  singledouble: 'singleDouble',
  doublesingle: 'doubleSingle',
}

const CALLOUTS: Record<CalloutKind, { glyph: string; label: string; color: (p: Palette) => string }> = {
  note: { glyph: '●', label: 'Note', color: p => p.info },
  tip: { glyph: '★', label: 'Tip', color: p => p.ok },
  important: { glyph: '◆', label: 'Important', color: p => p.accent },
  warning: { glyph: '▲', label: 'Warning', color: p => p.warn },
  caution: { glyph: '✖', label: 'Caution', color: p => p.err },
}

/** Blocks one under another: a blank row between them unless `isTight`, the native theme's gap in the native layout. */
export function blocks(list: readonly Block[], ctx: Ctx, isTight = false): RenderElement[] {
  const { Box } = ctx.t
  if (isNative(ctx)) return list.map(b => block(b, ctx))
  return list.map((b, i) => (
    <Box flexDirection="column" marginTop={i > 0 && !isTight ? 1 : 0}>
      {block(b, ctx)}
    </Box>
  ))
}

/** The blocks of one markdown run, in the column the layout lays them out in. */
export function column(list: readonly Block[], ctx: Ctx): RenderElement {
  const { Box } = ctx.t
  return isNative(ctx) ? (
    <Box flexDirection="column" flexGrow={1} flexShrink={1} gap={1}>
      {blocks(list, ctx)}
    </Box>
  ) : (
    <Box flexDirection="column" flexGrow={1} flexShrink={1}>
      {blocks(list, ctx)}
    </Box>
  )
}

/** A fence holding color literals: each line drawn as text with its swatches. */
function swatchedCode(b: Extract<Block, { kind: 'code' }>, ctx: Ctx): RenderElement | undefined {
  const lines = b.text.split('\n')
  if (!lines.some(l => find(l).length > 0)) return undefined
  const { Box, Text } = ctx.t
  return (
    <Box flexDirection="column">
      {b.lang !== '' && !isNative(ctx) && (
        <Text color={ctx.p.muted} italic>
          {clean(b.lang)}
        </Text>
      )}
      {lines.map(l => colorLine({ kind: 'line', text: clean(l), matches: find(l), code: true }, ctx.t))}
    </Box>
  )
}

export function block(b: Block, ctx: Ctx): RenderElement {
  const { Box, Text, Code, Markdown } = ctx.t
  const { p } = ctx
  if (b.kind === 'code') {
    const colored = b.isClosed ? swatchedCode(b, ctx) : undefined
    if (colored !== undefined) return colored
  }
  if (isNative(ctx)) {
    const drawn = nativeBlock(b, ctx, blocks)
    if (drawn !== undefined) return drawn
  }
  switch (b.kind) {
    case 'heading': {
      if (b.level === 1) {
        const title = clean(plainOf(b.inlines))
        const rule = '━'.repeat(Math.max(3, Math.min(cellWidth(title), ctx.width - 1)))
        return (
          <Box flexDirection="column">
            <Text bold>{graded(title, p.title, ctx)}</Text>
            <Text>{graded(rule, p.title, ctx)}</Text>
          </Box>
        )
      }
      if (b.level === 2) {
        return (
          <Text bold color={p.h2}>
            {'▍ '}
            {inlines(b.inlines, ctx, true)}
          </Text>
        )
      }
      if (b.level === 3) {
        return (
          <Text bold color={p.h3}>
            {inlines(b.inlines, ctx, true)}
          </Text>
        )
      }
      return (
        <Text bold italic color={p.h4}>
          {inlines(b.inlines, ctx, true)}
        </Text>
      )
    }

    case 'para':
      return ctx.isQuoted ? <Text italic>{inlines(b.inlines, ctx)}</Text> : <Text>{inlines(b.inlines, ctx)}</Text>

    case 'list': {
      const last = b.start + b.items.length - 1
      const markWidth = b.ordered ? String(last).length + 1 : 1
      const bulletColor = p.bullets[ctx.listDepth % p.bullets.length]!
      return (
        <Box flexDirection="column">
          {b.items.map((item, k) => {
            const mark = b.ordered ? `${b.start + k}.`.padStart(markWidth) : BULLETS[ctx.listDepth % BULLETS.length]!
            const marker =
              item.task === null ? (
                <Text color={b.ordered ? p.number : bulletColor} bold={b.ordered}>
                  {mark}
                </Text>
              ) : item.task ? (
                <Text color={p.ok} bold>
                  {'✔'}
                </Text>
              ) : (
                <Text color={p.muted}>{'○'}</Text>
              )
            const inner: Ctx = { ...ctx, width: ctx.width - markWidth - 1, listDepth: ctx.listDepth + 1 }
            return (
              <Box flexDirection="row" marginTop={b.isLoose && k > 0 ? 1 : 0}>
                <Box width={markWidth + 1} flexShrink={0}>
                  {marker}
                </Box>
                <Box flexDirection="column" flexGrow={1} flexShrink={1}>
                  {blocks(item.blocks, inner, !b.isLoose)}
                </Box>
              </Box>
            )
          })}
        </Box>
      )
    }

    case 'quote':
      return (
        <Box borderStyle="round" borderColor={p.quote} paddingX={1} flexDirection="column" alignSelf="flex-start">
          {blocks(b.blocks, { ...ctx, width: ctx.width - 4, isQuoted: true })}
        </Box>
      )

    case 'callout': {
      const meta = CALLOUTS[b.type]
      const color = meta.color(p)
      return (
        <Box borderStyle="round" borderColor={color} paddingX={1} flexDirection="column" alignSelf="flex-start">
          <Text bold color={color}>
            {`${meta.glyph} ${meta.label}`}
            {b.title.length > 0 ? ': ' : ''}
            {inlines(b.title, ctx, true)}
          </Text>
          {blocks(b.blocks, { ...ctx, width: ctx.width - 4 })}
        </Box>
      )
    }

    case 'panel': {
      const color = colorOf(b.color, ctx) ?? p.accent
      const border = BORDERS[(b.border ?? 'round').toLowerCase()] ?? 'round'
      return (
        <Box borderStyle={border} borderColor={color} paddingX={1} flexDirection="column" alignSelf="flex-start">
          {b.title !== undefined && (
            <Text bold color={color}>
              {clean(b.title)}
            </Text>
          )}
          {blocks(b.blocks, { ...ctx, width: ctx.width - 4 })}
        </Box>
      )
    }

    case 'code': {
      const source = b.text.replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, '').replace(/[\x00-\x08\x0b-\x1f\x7f]/g, '')
      const lang = b.lang.replace(/[^\w+#.-]/g, '')
      const parts: string[] = []
      let rest = source
      while (rest.length > 9000) {
        const cut = rest.lastIndexOf('\n', 9000)
        const at = cut > 0 ? cut : 9000
        parts.push(rest.slice(0, at))
        rest = rest.slice(at + (cut > 0 ? 1 : 0))
      }
      parts.push(rest)
      return (
        <Box flexDirection="column">
          {lang !== '' && (
            <Text color={p.muted} italic>
              {lang}
            </Text>
          )}
          {parts.map(part =>
            part.trim() === '' ? <Text> </Text> : lang !== '' ? <Code source={part} language={lang} /> : <Code source={part} />,
          )}
        </Box>
      )
    }

    case 'table':
      return table(b, ctx) ?? <Markdown text={stripTags(clean(b.raw)).slice(0, 9500)} />

    case 'hr':
      return <Text>{graded('─'.repeat(Math.max(3, ctx.width - 1)), p.rule, ctx)}</Text>
  }
}

/** A table drawn in box lines with styled cells, or undefined when it would not fit. */
function table(b: Extract<Block, { kind: 'table' }>, ctx: Ctx): RenderElement | undefined {
  const { Box, Text } = ctx.t
  const { p } = ctx
  const all = [b.header, ...b.rows]
  const plain = all.map(row => row.map(cell => clean(plainOf(cell))))
  if (plain.some(row => row.some(cell => cell.includes('\n')))) return undefined
  const widths = b.header.map((_, c) => Math.max(1, ...plain.map(row => cellWidth(row[c] ?? ''))))
  const total = widths.reduce((sum, w) => sum + w + 3, 1)
  if (total > ctx.width - 1) return undefined

  const line = (left: string, mid: string, right: string) => (
    <Text color={p.muted}>{left + widths.map(w => '─'.repeat(w + 2)).join(mid) + right}</Text>
  )
  const row = (cells: readonly Inline[][], texts: readonly string[], isHeader: boolean) => (
    <Text wrap="truncate-end">
      <Text color={p.muted}>│</Text>
      {widths.map((w, c) => {
        const room = w - cellWidth(texts[c] ?? '')
        const align = b.align[c] ?? 'left'
        const left = align === 'right' ? room : align === 'center' ? Math.floor(room / 2) : 0
        const cell = cells[c] ?? []
        return (
          <Text>
            {' '.repeat(left + 1)}
            {isHeader ? (
              <Text bold color={p.h2}>
                {inlines(cell, ctx, true)}
              </Text>
            ) : (
              <Text>{inlines(cell, ctx)}</Text>
            )}
            {' '.repeat(room - left + 1)}
            <Text color={p.muted}>│</Text>
          </Text>
        )
      })}
    </Text>
  )

  return (
    <Box flexDirection="column">
      {line('╭', '┬', '╮')}
      {row(b.header, plain[0]!, true)}
      {line('├', '┼', '┤')}
      {b.rows.map((cells, r) => row(cells, plain[r + 1]!, false))}
      {line('╰', '┴', '╯')}
    </Box>
  )
}
