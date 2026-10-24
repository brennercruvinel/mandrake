// The native theme's blocks: flat colored headings, written list markers with nested
// lists indented, a dim bar before each quoted line, and fences and tables
// handed to the engine's Markdown. Callouts, panels and rules draw as in the vivid layout.

import type { RenderElement } from 'claude-code'

import type { Block } from './blocks'
import { type Ctx, clean, inlines } from './inline'
import { parseInlines } from './inlines'
import { CONFIG, NATIVE_LIMIT, paint } from './native'
import { stripTags } from './stream'

type Blocks = (list: readonly Block[], ctx: Ctx) => RenderElement[]

/** The block in the native theme, or undefined for one it draws as the vivid layout does. */
export function nativeBlock(b: Block, ctx: Ctx, blocks: Blocks): RenderElement | undefined {
  const { Box, Text, Markdown } = ctx.t
  const { p } = ctx
  switch (b.kind) {
    case 'heading':
      return (
        <Text bold color={p.h2}>
          {inlines(b.inlines, ctx, true)}
        </Text>
      )

    case 'para':
      return b.pipeRaw !== undefined ? <Text>{clean(b.pipeRaw)}</Text> : undefined

    case 'list': {
      const lists = (item: Block[]) => item.filter(x => x.kind === 'list')
      return (
        <Box flexDirection="column">
          {b.items.map(item => {
            const mark = item.task === null ? (b.ordered ? item.marker : CONFIG.glyphs.bullet) : item.task ? '✔' : '○'
            const nested = lists(item.blocks)
            const inner: Ctx = { ...ctx, listDepth: ctx.listDepth + 1 }
            return (
              <Box flexDirection="row">
                <Box flexShrink={0}>
                  <Text {...paint({ ...CONFIG.styles.listMarker, color: p.number })}>{`${mark} `}</Text>
                </Box>
                <Box flexDirection="column" flexGrow={1} flexShrink={1}>
                  {blocks(
                    item.blocks.filter(x => x.kind !== 'list'),
                    inner,
                  )}
                  {nested.length > 0 ? (
                    <Box flexDirection="column" paddingLeft={CONFIG.indent}>
                      {blocks(nested, inner)}
                    </Box>
                  ) : null}
                </Box>
              </Box>
            )
          })}
        </Box>
      )
    }

    case 'quote':
      return (
        <Box flexDirection="column">
          {b.lines.map(line => (
            <Box flexDirection="row">
              <Box flexShrink={0}>
                <Text {...paint(CONFIG.styles.quoteBar)}>{`${CONFIG.glyphs.quoteBar} `}</Text>
              </Box>
              <Box flexGrow={1} flexShrink={1}>
                <Text {...paint(CONFIG.styles.quote)}>{inlines(parseInlines(line || ' '), ctx)}</Text>
              </Box>
            </Box>
          ))}
        </Box>
      )

    case 'code':
      if (!b.isClosed) return <Text>{clean(b.raw)}</Text>
      return b.raw.length > NATIVE_LIMIT ? <Text>{clean(b.raw)}</Text> : <Markdown text={b.raw} />

    case 'table': {
      const raw = stripTags(b.raw)
      return raw.length > NATIVE_LIMIT ? <Text>{clean(raw)}</Text> : <Markdown text={raw} />
    }

    default:
      return undefined
  }
}
