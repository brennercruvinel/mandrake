// Draws the status bar in ANSI at several widths, from the same layout code the
// mod runs, so a design change can be seen before it is loaded.
//
//   bun scripts/preview-statusbar.ts [light] [width ...]

import type { StatusBarSpan } from '../types'
import { toStatusOptions } from '../hooks/statusbar/format'
import { layout, paletteOf } from '../hooks/statusbar/layout'
import type { StatusModel } from '../hooks/statusbar/model'

const NOW = Date.parse('2026-10-21T04:00:00Z')
const MIN = 60_000
const HOUR = 60 * MIN

const base: StatusModel = {
  identity: { name: 'quimera', color: '#39C5BB' },
  model: 'Opus 5.5',
  context: { tokens: 37_000, budget: 800_000 },
  limits: [
    { kind: 'five_hour', label: '5h', percent: 5, resetsAt: NOW + 2 * HOUR + 14 * MIN },
    { kind: 'seven_day', label: '7d', percent: 45, resetsAt: NOW + 78 * HOUR },
  ],
  branch: 'movom',
  diff: { added: 5, removed: 224 },
  pullRequest: {
    number: 38,
    url: 'https://github.com/brennercruvinel/quimera/pull/38',
    state: 'Open',
  },
  checks: { passed: 4, failed: 0, pending: 0, total: 4 },
  minute: 0,
}

const scenarios: [string, StatusModel][] = [
  ['calm', base],
  [
    'pressure: context at 70% of the compact budget, the week on a pace past 100%',
    {
      ...base,
      context: { tokens: 560_000, budget: 800_000 },
      limits: [base.limits[0]!, { ...base.limits[1]!, percent: 65 }],
    },
  ],
  [
    'critical: context near compact, 5h at 93%, branch already merged',
    {
      ...base,
      context: { tokens: 720_000, budget: 800_000 },
      limits: [{ ...base.limits[0]!, percent: 93 }, base.limits[1]!],
      branch: 'feat/tiamati-dragon-statusbar-redesign',
      pullRequest: { ...base.pullRequest!, state: 'Merged' },
      checks: { passed: 3, failed: 1, pending: 0, total: 4 },
    },
  ],
]

const args = process.argv.slice(2)
const isLight = args.includes('light')
const widths = args.filter(a => /^\d+$/.test(a)).map(Number)
const palette = paletteOf(isLight)
const options = { icons: toStatusOptions({}).icons, palette, nowMs: NOW }

const ESC = '\x1b['
const rgb = (hex: string): string => {
  const n = Number.parseInt(hex.slice(1), 16)
  return `${ESC}38;2;${(n >> 16) & 255};${(n >> 8) & 255};${n & 255}m`
}
const bgRgb = (hex: string): string => rgb(hex).replace('38;2;', '48;2;')
const ansi = (spans: readonly StatusBarSpan[]): string =>
  spans
    .map(s => `${s.bg ? bgRgb(s.bg) : ''}${s.color ? rgb(s.color) : `${ESC}2m`}${s.text}${ESC}0m`)
    .join('')

const HINT = `${ESC}2m⏵⏵ bypass permissions on (shift+tab to cycle) · ← 1 agent${ESC}0m`

for (const [title, model] of scenarios) {
  console.log(`\n${ESC}1m${title}${ESC}0m`)
  for (const width of widths.length > 0 ? widths : [150, 120, 100, 80, 60, 44]) {
    console.log(`${ESC}2m${String(width).padStart(3)} ${'─'.repeat(width)}${ESC}0m`)
    for (const row of layout(model, width, options)) console.log(`    ${ansi(row)}`)
    console.log(`    ${HINT}`)
  }
}
