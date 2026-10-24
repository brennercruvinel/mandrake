// The rows a reply is drawn in: the plugin's own rows behind its gutter, and
// the rows the plugins beneath drew, which bring their own.

import type { ElementTable, RenderElement, RenderSurface } from 'claude-code'

import { segment } from '../swatches/segment'
import { colorLine } from '../swatches/swatch'
import { type Ctx, inlines } from '../markdown/inline'
import { parseInlines } from '../markdown/parse'
import { CONFIG } from '../markdown/native'
import { badge } from '../badges/badge'
import type { Config } from '../badges/config'
import type { Part } from './segments'

type Table = Pick<ElementTable, 'Box' | 'Text' | 'Markdown'>

export type Row = { element: RenderElement; gap: boolean; isOwn: boolean }

/** Whether any line outside the cut fences holds a color literal. */
export function hasColors(parts: readonly Part[]): boolean {
  const colored = (text: string) => segment(text).some(s => s.kind === 'line')
  return parts.some(p => (p.kind === 'md' && colored(p.text)) || (p.kind === 'tag' && colored(p.rest)))
}

/** A status tag line: the badge, then the rest of the line through the engine in force. */
export function tagRow(part: Extract<Part, { kind: 'tag' }>, tags: Config, t: Table, inner: Ctx | undefined): RenderElement {
  const { Box, Text, Markdown } = t
  const [line] = part.rest ? segment(part.rest) : []
  const rest = !part.rest ? null : inner ? (
    <Text>{inlines(parseInlines(part.rest), inner)}</Text>
  ) : line?.kind === 'line' ? (
    colorLine(line, t)
  ) : (
    <Markdown text={part.rest} />
  )
  return (
    <Box flexDirection="row" flexGrow={1} flexShrink={1}>
      {badge(part.tag, tags, t)}
      {rest ? (
        <Box marginLeft={1} flexShrink={1}>
          {rest}
        </Box>
      ) : null}
    </Box>
  )
}

/** The rows one under another; on the terminal the reply opens with a bullet in a two-column gutter. */
export function assemble(
  rows: readonly Row[],
  t: Table,
  { surface, isFirstOfReply, ctx }: { surface: RenderSurface; isFirstOfReply: boolean; ctx: Ctx | undefined },
): RenderElement {
  const { Box, Text } = t
  const isRich = ctx !== undefined && ctx.p.layout !== 'native'
  const dot = isRich ? '⏺' : CONFIG.glyphs.reply
  return (
    <Box flexDirection="column">
      {rows.map((r, i) => {
        const marginTop = i > 0 && r.gap ? 1 : 0
        if (!r.isOwn || surface !== 'terminal') return <Box marginTop={marginTop}>{r.element}</Box>
        const glyph = i === 0 && isFirstOfReply ? dot : ' '
        return (
          <Box flexDirection="row" marginTop={marginTop}>
            <Box width={2} flexShrink={0}>
              {isRich && ctx ? <Text color={ctx.p.dot}>{glyph}</Text> : <Text>{glyph}</Text>}
            </Box>
            {r.element}
          </Box>
        )
      })}
    </Box>
  )
}
