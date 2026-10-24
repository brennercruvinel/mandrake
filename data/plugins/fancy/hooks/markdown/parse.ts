// The parsing API: markdown blocks, the markup tags, and the streaming round trip.

import { type Block, parseBlocks } from './blocks'

export { type Align, type Block, type CalloutKind, type ListItem, parseBlocks } from './blocks'
export { type GradStops, type Inline, type Style, isTag, parseInlines, plainOf } from './inlines'
export { restoreLines, stripStreamed, stripTags } from './stream'

const parsed = new Map<string, Block[]>()

/** The text's blocks, kept for the redraws a scroll or resize asks for. */
export function parseCached(text: string): Block[] {
  const hit = parsed.get(text)
  if (hit !== undefined) return hit
  const result = parseBlocks(text)
  if (parsed.size > 300) parsed.delete(parsed.keys().next().value!)
  parsed.set(text, result)
  return result
}
