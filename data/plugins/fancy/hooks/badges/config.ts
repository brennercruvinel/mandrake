import type { Color, PluginOptions } from 'claude-code'

import type { TextMode } from './color'

export type Style = 'badge' | 'text'

export type Config = {
  tags: ReadonlyMap<string, Color>
  style: Style
  textColor: TextMode
}

export const DEFAULT_TAGS =
  'DONE:#13651A,FAIL:#A10E01,NEXT:#0D3CD9,WAIT:#FF782D,' +
  'GOAL:#D0F4DE,DRFT:#F2F2F2,WARN:#FFC18F,HYPO:#7BDFF2,IDEA:#FFF170,POCS:#80FFDB,REFS:#3A0CA3,AUTO:#4130DF>#9B1FE8>#C2185B,LEAD:#FF2BFF'

const NAME = /^[A-Za-z0-9_-]+$/

export function parseTags(spec: string): Map<string, Color> {
  const tags = new Map<string, Color>()

  for (const pair of spec.split(',')) {
    const at = pair.indexOf(':')
    const name = pair.slice(0, at).trim()
    const color = pair.slice(at + 1).trim()
    if (at > 0 && color && NAME.test(name)) tags.set(name, color)
  }

  return tags
}

export function readConfig(options: PluginOptions): Config {
  const custom = typeof options.tags === 'string' ? parseTags(options.tags) : new Map<string, Color>()

  return {
    tags: custom.size > 0 ? custom : parseTags(DEFAULT_TAGS),
    style: options.style === 'text' ? 'text' : 'badge',
    textColor: options.text_color === 'white' || options.text_color === 'black' ? options.text_color : 'auto',
  }
}
