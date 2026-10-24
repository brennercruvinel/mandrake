import type { Color } from 'claude-code'

export type TextMode = 'auto' | 'white' | 'black'

const WHITE = '#FFFFFF'
const BLACK = '#000000'
const HEX = /^#[0-9a-fA-F]{6}$/

function channel(hex: string, at: number): number {
  const c = parseInt(hex.slice(at, at + 2), 16) / 255
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

// WCAG 2 relative luminance; undefined for theme keys and names, whose value only the surface knows.
export function luminance(color: Color): number | undefined {
  if (!HEX.test(color)) return undefined
  return 0.2126 * channel(color, 1) + 0.7152 * channel(color, 3) + 0.0722 * channel(color, 5)
}

export function contrast(a: number, b: number): number {
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05)
}

export function textOn(background: Color | undefined, mode: TextMode): Color {
  if (mode === 'white') return WHITE
  if (mode === 'black') return BLACK

  const l = background === undefined ? undefined : luminance(background)
  if (l === undefined) return 'inverseText'
  return contrast(1, l) >= contrast(0, l) ? WHITE : BLACK
}

// A color written as stops joined by '>' ("#4130DF>#9B1FE8>#C2185B") paints one stop per cell, interpolated.
export function stopsOf(color: Color): string[] {
  return color.split('>').map(s => s.trim()).filter(Boolean)
}

export function gradient(stops: readonly string[], cells: number): Color[] {
  if (stops.length < 2 || !stops.every(s => HEX.test(s))) return Array(cells).fill(stops[0] ?? '')

  const rgb = (hex: string) => [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16))
  const hex = (c: number[]) => '#' + c.map(v => Math.round(v).toString(16).padStart(2, '0')).join('').toUpperCase()

  return Array.from({ length: cells }, (_, i) => {
    const t = cells === 1 ? 0 : (i / (cells - 1)) * (stops.length - 1)
    const k = Math.min(Math.floor(t), stops.length - 2)
    const a = rgb(stops[k] as string)
    const b = rgb(stops[k + 1] as string)
    return hex(a.map((v, j) => v + ((b[j] as number) - v) * (t - k)))
  })
}

// One text color for a whole gradient: the one whose worst cell has the higher contrast.
export function textOnAll(backgrounds: readonly Color[], mode: TextMode): Color {
  if (mode !== 'auto' || backgrounds.length === 1) return textOn(backgrounds[0], mode)

  const ls = backgrounds.map(luminance)
  if (ls.some(l => l === undefined)) return 'inverseText'
  const worst = (text: number) => Math.min(...ls.map(l => contrast(text, l as number)))
  return worst(1) >= worst(0) ? WHITE : BLACK
}
