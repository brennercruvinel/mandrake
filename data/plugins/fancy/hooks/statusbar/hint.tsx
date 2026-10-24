import type { ElementTable, RenderElement } from 'claude-code'

import type { StatusBarSpan } from '../../types'

/** The status rows above the engine's hint, each span dim or in its color, on its background, linked when it has an href. */
export function hintRows(
  engineHint: RenderElement,
  rows: readonly (readonly StatusBarSpan[])[],
  t: Pick<ElementTable, 'Box' | 'Link' | 'Text'>,
): RenderElement {
  const { Box, Link, Text } = t
  return (
    <Box flexDirection="column">
      {rows.map(row => (
        <Text wrap="truncate-end">
          {row.map(span => {
            const text = span.href === undefined ? span.text : <Link href={span.href} label={span.text} />

            const background = span.bg === undefined ? {} : { backgroundColor: span.bg }

            return span.color === undefined ? (
              <Text dimColor {...background}>
                {text}
              </Text>
            ) : (
              <Text color={span.color} {...background}>
                {text}
              </Text>
            )
          })}
        </Text>
      ))}
      {engineHint}
    </Box>
  )
}
