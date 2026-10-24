import { describe, expect, test } from 'claude-code/testing'

import type { StatusBarSpan, StatusModel } from '../types'
import { type Segment, cellWidth, fit, fitState } from '../hooks/statusbar/fit'
import { toStatusOptions } from '../hooks/statusbar/format'
import { bar, layout, paletteOf } from '../hooks/statusbar/layout'
import {
  contextLevel,
  formatIn,
  formatTokens,
  limitLevel,
  parseCompactAt,
  parseIdentityColors,
  prettyModel,
  projectedPercent,
  toContext,
  toLimits,
} from '../hooks/statusbar/model'

const MIN = 60_000
const HOUR = 60 * MIN
const DAY = 24 * HOUR
const NOW = Date.parse('2026-10-21T04:00:00Z')
const DARK = paletteOf(false)
const OPTIONS = { icons: toStatusOptions({}).icons, palette: DARK, nowMs: NOW }

const MODEL: StatusModel = {
  identity: { name: 'quimera', color: '#39C5BB' },
  model: 'Opus 5.5',
  context: { tokens: 37_000, budget: 800_000 },
  limits: [
    { kind: 'five_hour', label: '5h', percent: 5, resetsAt: NOW + 2 * HOUR + 14 * MIN },
    { kind: 'seven_day', label: '7d', percent: 45, resetsAt: NOW + 78 * HOUR },
  ],
  branch: 'movom',
  diff: { added: 5, removed: 224 },
  pullRequest: { number: 38, url: 'https://github.com/acme/app/pull/38', state: 'Open' },
  checks: { passed: 4, failed: 0, pending: 0, total: 4 },
  minute: 0,
}

const text = (spans: readonly StatusBarSpan[]): string => spans.map(span => span.text).join('')
const rows = (model: StatusModel, columns: number): string[] => layout(model, columns, OPTIONS).map(text)
const spanWith = (model: StatusModel, columns: number, part: string): StatusBarSpan | undefined =>
  layout(model, columns, OPTIONS)
    .flat()
    .find(span => span.text.includes(part))

describe('model', () => {
  test('spells model ids the way /model names them', () => {
    expect(prettyModel('claude-opus-5-5[1m]')).toBe('Opus 5.5')
    expect(prettyModel('claude-sonnet-4-5-20250929')).toBe('Sonnet 4.5')
    expect(prettyModel('claude-haiku-4')).toBe('Haiku 4')
    expect(prettyModel('opus')).toBe('Opus')
    expect(prettyModel('Opus 5.5')).toBe('Opus 5.5')
    expect(prettyModel('')).toBeUndefined()
  })

  test('labels the plan windows and reads resets in seconds, milliseconds or ISO', () => {
    const iso = new Date(NOW).toISOString()
    expect(
      toLimits([
        { kind: 'five_hour', percentUsed: 5, resetsAt: String(NOW / 1000) },
        { kind: 'seven_day', percentUsed: 45, resetsAt: iso },
        { kind: 'other', percentUsed: 1, resetsAt: 'soon' },
      ]),
    ).toEqual([
      { kind: 'five_hour', label: '5h', percent: 5, resetsAt: NOW },
      { kind: 'seven_day', label: '7d', percent: 45, resetsAt: NOW },
      { kind: 'other', label: 'other', percent: 1, resetsAt: undefined },
    ])
  })

  test('budgets the context to where the session compacts', () => {
    expect(parseCompactAt('80')).toBe(0.8)
    expect(parseCompactAt('0.75')).toBe(0.75)
    expect(parseCompactAt('off')).toBeUndefined()
    expect(toContext({ window: 1_000_000, tokens: 37_000 }, 0.8)).toEqual({ tokens: 37_000, budget: 800_000 })
    expect(toContext({ window: 200_000 }, undefined)).toEqual({ tokens: undefined, budget: 200_000 })
    expect(toContext({ window: 0 }, 0.8)).toBeUndefined()
  })

  test('warns on the context from 60% of the budget and alarms from 85%', () => {
    expect(contextLevel({ tokens: 470_000, budget: 800_000 })).toBe('calm')
    expect(contextLevel({ tokens: 480_000, budget: 800_000 })).toBe('warn')
    expect(contextLevel({ tokens: 680_000, budget: 800_000 })).toBe('crit')
  })

  test('judges a window by its pace once a tenth of it went by', () => {
    const week = (percent: number, leftMs: number) =>
      ({ kind: 'seven_day', label: '7d', percent, resetsAt: NOW + leftMs }) as const

    expect(Math.round(projectedPercent(week(65, 78 * HOUR), NOW)! * 10) / 10).toBe(121.3)
    expect(limitLevel(week(65, 78 * HOUR), NOW)).toBe('warn')
    expect(limitLevel(week(66, 40 * HOUR), NOW)).toBe('calm')
    expect(projectedPercent(week(5, 7 * DAY - HOUR), NOW)).toBeUndefined()
    expect(limitLevel(week(80, 7 * DAY - HOUR), NOW)).toBe('warn')
    expect(limitLevel(week(91, 6 * HOUR), NOW)).toBe('crit')
    expect(limitLevel({ kind: 'spend_limit', label: '$', percent: 74 }, NOW)).toBe('calm')
  })

  test('formats tokens and time left compactly', () => {
    expect([formatTokens(950), formatTokens(37_400), formatTokens(1_000_000), formatTokens(1_250_000)]).toEqual([
      '950',
      '37k',
      '1M',
      '1.3M',
    ])
    expect([formatIn(-1), formatIn(14 * MIN), formatIn(2 * HOUR + 14 * MIN), formatIn(3 * DAY + 6 * HOUR)]).toEqual([
      'now',
      '14m',
      '2h14',
      '3d6h',
    ])
  })

  test('reads identity colors by lowercased name', () => {
    expect([...parseIdentityColors('Quimera:#39C5BB, tiamati : #8C7BFF,broken')]).toEqual([
      ['quimera', '#39C5BB'],
      ['tiamati', '#8C7BFF'],
    ])
  })
})

describe('fit', () => {
  const segment = (id: string, widths: number[], ranks: number[], dropRank = Infinity): Segment => ({
    id,
    variants: widths.map((w, i) => ({ rank: ranks[i] ?? 0, spans: [{ text: id[0]!.repeat(w) }] })),
    dropRank,
  })

  test('counts wide and combining characters as the terminal does', () => {
    expect(cellWidth('◆ ↻ ✓ █▍')).toBe(8)
    expect(cellWidth('日本')).toBe(4)
    expect(cellWidth('é')).toBe(1)
  })

  test('shrinks the lowest rank first and drops before touching what never drops', () => {
    const segments = [segment('aaa', [6, 2], [0, 10], 50), segment('bbb', [6, 3], [0, 20])]

    expect(text(fit(segments, 20))).toBe('aaaaaa  bbbbbb')
    expect(text(fit(segments, 10))).toBe('aa  bbbbbb')
    expect(text(fit(segments, 7))).toBe('aa  bbb')
    expect(text(fit(segments, 4))).toBe('bbb')
  })

  test('shrinks equal ranks together', () => {
    const segments = [segment('aaa', [4, 1], [0, 30]), segment('bbb', [4, 1], [0, 30])]

    expect(text(fit(segments, 9))).toBe('a  b')
  })

  test('refuses a fit that needs a step above the cap', () => {
    const segments = [segment('aaa', [10, 2], [0, 40])]

    expect(fitState(segments, 5, 30)).toBeNull()
    expect(fitState(segments, 5, 40)).toEqual([1])
  })
})

describe('layout', () => {
  test('keeps resources and work on one row while they fit', () => {
    expect(rows(MODEL, 120)).toEqual([
      '◆ quimera  Opus 5.5  ▌          37k/800k  5h 5% ↻2h14  7d 45% ↻3d6h   movom  +5 −224  #38 open  ✓ 4/4',
    ])
  })

  test('drops both resets together before splitting the row', () => {
    expect(rows(MODEL, 100)).toEqual(['◆ quimera  Opus 5.5  ▌          37k/800k  5h 5%  7d 45%   movom  +5 −224  #38 open  ✓ 4/4'])
  })

  test('splits into resources and work when one row would cost content', () => {
    expect(rows(MODEL, 80)).toEqual([
      '◆ quimera  Opus 5.5  ▌          37k/800k  5h 5% ↻2h14  7d 45% ↻3d6h',
      'movom  +5 −224  #38 open  ✓ 4/4',
    ])
  })

  test('keeps the context and the identity mark on the narrowest rows', () => {
    const [top] = rows(MODEL, 30)

    expect(top).toStartWith('◆')
    expect(top).toContain('5%')
  })

  test('keeps a merged PR labeled at every width', () => {
    const merged: StatusModel = { ...MODEL, pullRequest: { ...MODEL.pullRequest!, state: 'Merged' } }

    for (const columns of [150, 100, 60, 30]) {
      expect(rows(merged, columns).join('\n')).toContain('#38 merged')
    }
    expect(spanWith(merged, 150, 'merged')?.color).toBe(DARK.warn)
    expect(spanWith(merged, 150, '#38')?.href).toBe('https://github.com/acme/app/pull/38')
  })

  test('paints the context bar in the identity color over a track', () => {
    expect(bar(0.25, 4, '#39C5BB', DARK.track)).toEqual([
      { text: '█', color: '#39C5BB' },
      { text: '   ', bg: DARK.track },
    ])
    expect(bar(0.3, 10, 'c', 't')).toEqual([
      { text: '███', color: 'c' },
      { text: '       ', bg: 't' },
    ])
    expect(bar(0.05, 10, 'c', 't')).toEqual([
      { text: '▌', color: 'c', bg: 't' },
      { text: '         ', bg: 't' },
    ])
    expect(spanWith(MODEL, 120, '▌')?.color).toBe('#39C5BB')
  })

  test('colors by level: amber context and pace, red limits and failed checks', () => {
    const pressed: StatusModel = {
      ...MODEL,
      context: { tokens: 560_000, budget: 800_000 },
      limits: [{ ...MODEL.limits[0]!, percent: 93 }, { ...MODEL.limits[1]!, percent: 65 }],
      checks: { passed: 3, failed: 1, pending: 0, total: 4 },
    }

    expect(spanWith(pressed, 150, '560k/800k')?.color).toBe(DARK.warn)
    expect(spanWith(pressed, 150, '93%')?.color).toBe(DARK.crit)
    expect(spanWith(pressed, 150, '65%')?.color).toBe(DARK.warn)
    expect(spanWith(pressed, 150, '✗')?.color).toBe(DARK.crit)
    expect(spanWith(MODEL, 150, '45%')?.color).toBeUndefined()
  })

  test('draws only what it knows', () => {
    expect(rows({ limits: [], minute: 0 }, 80)).toEqual([])
    expect(rows({ limits: [], branch: 'main', minute: 0 }, 80)).toEqual(['main'])
    expect(rows({ ...MODEL, diff: { added: 0, removed: 0 }, checks: undefined }, 150).join()).not.toMatch(/\+0|✓/)
  })
})
