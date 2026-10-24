import type { ElementTable, RenderElement } from 'claude-code'

import { gradient, stopsOf, textOnAll } from './color'
import type { Config } from './config'

/** A status tag as a badge: one Text when solid, one Text per cell along a gradient. */
export function badge(tag: string, config: Config, t: Pick<ElementTable, 'Box' | 'Text'>): RenderElement {
  const { Box, Text } = t
  const { style, textColor } = config
  const label = style === 'badge' ? ` ${tag} ` : `[${tag}]`
  const cells = gradient(stopsOf(config.tags.get(tag) ?? ''), label.length)
  const text = textOnAll(cells, textColor)
  const isSolid = cells.every(c => c === cells[0])

  if (isSolid) {
    return style === 'badge' ? (
      <Text backgroundColor={cells[0]} color={text} bold>
        {label}
      </Text>
    ) : (
      <Text color={cells[0]} bold>
        {label}
      </Text>
    )
  }

  return (
    <Box flexDirection="row" flexShrink={0}>
      {[...label].map((ch, j) =>
        style === 'badge' ? (
          <Text backgroundColor={cells[j]} color={text} bold>
            {ch}
          </Text>
        ) : (
          <Text color={cells[j]} bold>
            {ch}
          </Text>
        ),
      )}
    </Box>
  )
}
