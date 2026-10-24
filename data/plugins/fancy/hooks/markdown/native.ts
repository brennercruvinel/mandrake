// The native theme: flat colored markdown, with fences and tables left to the
// engine's own Markdown. Drawn by the shared engine under the `native` theme.

import type { Palette } from './themes'

type Style = { color?: string; bold?: boolean; italic?: boolean; dimColor?: boolean; strikethrough?: boolean }

// Style colors name a key of `colors`, or are a raw color.
export const CONFIG = {
  colors: {
    purple: '#9d7cd8',
    orange: '#f5a742',
    amber: '#e5c07b',
    green: '#7fd88f',
  } as Record<string, string>,
  styles: {
    heading: { bold: true, color: 'purple' },
    bold: { bold: true, color: 'orange' },
    italic: { italic: true, color: 'amber' },
    code: { color: 'green' },
    strike: { strikethrough: true },
    linkUrl: { dimColor: true },
    listMarker: { color: 'orange' },
    quoteBar: { dimColor: true },
    quote: { italic: true },
  } as Record<string, Style>,
  glyphs: { bullet: '-', quoteBar: '│', reply: '●' },
  indent: 2,
  debugTiming: false,
}

/** Past this a fence or table is drawn as plain text, not handed to Markdown. */
export const NATIVE_LIMIT = 10000

export const paint = (style: Style | undefined): Style => {
  if (style === undefined) return {}
  return style.color ? { ...style, color: CONFIG.colors[style.color] ?? style.color } : style
}

const color = (style: string) => paint(CONFIG.styles[style]).color ?? '#d0d0d0'

export const NATIVE: Palette = {
  title: [color('heading')],
  h2: color('heading'),
  h3: color('heading'),
  h4: color('heading'),
  bold: color('bold'),
  italic: color('italic'),
  code: color('code'),
  link: '#61afef',
  bullets: [color('listMarker')],
  number: color('listMarker'),
  quote: '#8a8a8a',
  rule: ['#5c6370'],
  muted: '#8a8a8a',
  ok: CONFIG.colors.green!,
  warn: CONFIG.colors.amber!,
  err: '#e06c75',
  info: '#61afef',
  accent: CONFIG.colors.purple!,
  highlight: CONFIG.colors.amber!,
  dot: '#d0d0d0',
  series: [CONFIG.colors.purple!, CONFIG.colors.orange!, CONFIG.colors.green!, CONFIG.colors.amber!, '#61afef'],
  tone: { saturation: 1 },
  layout: 'native',
}
