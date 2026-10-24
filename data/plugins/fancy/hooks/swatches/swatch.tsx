// Color literals drawn on their own color, in the spirit of nvim-colorizer.

import type { ElementTable, RenderChildren, RenderElement } from 'claude-code'

import { contrast, find, hex } from './colors'
import type { Segment } from './segment'

type Table = Pick<ElementTable, 'Box' | 'Text'>

/** `text` with each literal on its own color; `plain` draws the runs between them. */
export function swatched(
  text: string,
  t: Pick<Table, 'Text'>,
  plain: (run: string) => RenderChildren = run => run,
): RenderChildren[] {
  const matches = find(text)
  if (matches.length === 0) return [plain(text)]
  const { Text } = t
  const parts: RenderChildren[] = []
  let at = 0
  for (const m of matches) {
    if (m.start > at) parts.push(plain(text.slice(at, m.start)))
    parts.push(
      <Text backgroundColor={hex(m.color)} color={contrast(m.color)}>
        {m.text}
      </Text>,
    )
    at = m.end
  }
  if (at < text.length) parts.push(plain(text.slice(at)))
  return parts
}

/** A line holding color literals, drawn as plain text; a fence's line is indented. */
export function colorLine(s: Extract<Segment, { kind: 'line' }>, t: Table): RenderElement {
  const { Box, Text } = t
  const parts = s.text === '' ? [] : swatched(s.text, t)
  return (
    <Box paddingLeft={s.code ? 2 : 0}>
      <Text>{parts.length ? parts : ' '}</Text>
    </Box>
  )
}
