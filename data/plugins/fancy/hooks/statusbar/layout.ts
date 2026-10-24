import type { StatusBarSpan } from '../../types'
import { type Segment, fitState, spansOf } from './fit'
import type { ChecksSummary, StatusIcons } from './format'
import {
  type Level,
  type Limit,
  type StatusModel,
  contextLevel,
  formatIn,
  formatTokens,
  limitLevel,
} from './model'

/** Semantic colors, one set per background; the theme's own dim text is everything else. */
export type Palette = { accent: string; ok: string; warn: string; crit: string; merged: string; track: string }

const DARK: Palette = {
  accent: '#58A6FF',
  ok: '#3FB950',
  warn: '#D29922',
  crit: '#F85149',
  merged: '#A371F7',
  track: '#30363D',
}
const LIGHT: Palette = {
  accent: '#0969DA',
  ok: '#1A7F37',
  warn: '#9A6700',
  crit: '#CF222E',
  merged: '#8250DF',
  track: '#D0D7DE',
}

export const paletteOf = (isLight: boolean): Palette => (isLight ? LIGHT : DARK)

export type LayoutOptions = { icons: StatusIcons; palette: Palette; nowMs: number }

const BAR_CELLS = 10
const BAR_CELLS_NARROW = 5
const EIGHTHS = ' ▏▎▍▌▋▊▉'
const BRANCH_MAX = 32
const BRANCH_NARROW = 14
const IDENTITY_MARK = '◆'
const RESET_MARK = '↻'
const MINUS = '−'
const GROUP_GAP = '   '
/** Steps up to this rank still keep resources and work on one row. */
const ONE_ROW_RANK = 34

const levelColor = (level: Level, p: Palette, calm?: string): string | undefined =>
  level === 'crit' ? p.crit : level === 'warn' ? p.warn : calm

/**
 * A bar in eighth blocks over a track painted as background, so a few percent
 * already shows and the partial cell blends into the track.
 */
export function bar(share: number, cells: number, color: string | undefined, track: string): StatusBarSpan[] {
  const eighths = Math.round(Math.min(1, Math.max(0, share)) * cells * 8)
  const full = Math.floor(eighths / 8)
  const part = eighths % 8
  const empty = cells - full - (part > 0 ? 1 : 0)
  return [
    { text: '█'.repeat(full), color },
    { text: part > 0 ? EIGHTHS[part]! : '', color, bg: track },
    { text: ' '.repeat(empty), bg: track },
  ].filter(span => span.text !== '')
}

function middleCut(text: string, max: number): string {
  if ([...text].length <= max) return text
  const chars = [...text]
  const head = Math.ceil((max - 1) / 2)
  return `${chars.slice(0, head).join('')}…${chars.slice(chars.length - (max - 1 - head)).join('')}`
}

function identitySegment(model: StatusModel, p: Palette): Segment[] {
  if (model.identity === undefined) return []
  const color = model.identity.color ?? p.accent
  return [
    {
      id: 'identity',
      variants: [
        { rank: 0, spans: [{ text: `${IDENTITY_MARK} ${model.identity.name}`, color }] },
        { rank: 55, spans: [{ text: IDENTITY_MARK, color }] },
      ],
      dropRank: 90,
    },
  ]
}

function modelSegment(model: StatusModel): Segment[] {
  if (model.model === undefined) return []
  return [{ id: 'model', variants: [{ rank: 0, spans: [{ text: model.model }] }], dropRank: 45 }]
}

function contextSegment(model: StatusModel, p: Palette): Segment[] {
  const fill = model.context
  if (fill === undefined) return []
  const share = (fill.tokens ?? 0) / fill.budget
  const color = levelColor(contextLevel(fill), p, model.identity?.color ?? p.accent)
  const percent = `${Math.round(share * 100)}%`
  const drawn = (cells: number, label: string): StatusBarSpan[] => [
    ...bar(share, cells, color, p.track),
    { text: ` ${label}`, color: share >= 0.6 ? color : undefined },
  ]
  const tokens = fill.tokens === undefined ? '–' : formatTokens(fill.tokens)
  return [
    {
      id: 'context',
      variants: [
        { rank: 0, spans: drawn(BAR_CELLS, `${tokens}/${formatTokens(fill.budget)}`) },
        { rank: 35, spans: drawn(BAR_CELLS, percent) },
        { rank: 62, spans: drawn(BAR_CELLS_NARROW, percent) },
      ],
      dropRank: Infinity,
    },
  ]
}

function limitSegment(limit: Limit, nowMs: number, p: Palette, order: number): Segment {
  const level = limitLevel(limit, nowMs)
  const color = levelColor(level, p)
  const head: StatusBarSpan[] = [{ text: `${limit.label} ` }, { text: `${Math.round(limit.percent)}%`, color }]
  const reset = limit.resetsAt === undefined ? [] : [{ text: ` ${RESET_MARK}${formatIn(limit.resetsAt - nowMs)}` }]
  return {
    id: `limit:${limit.kind}`,
    variants: [
      { rank: 0, spans: [...head, ...reset] },
      { rank: 30, spans: head },
    ],
    dropRank: level === 'calm' ? 50 + order : Infinity,
  }
}

function branchSegment(model: StatusModel, icons: StatusIcons): Segment[] {
  if (model.branch === undefined) return []
  const named = (max: number): StatusBarSpan[] => [{ text: `${icons.branch}${middleCut(model.branch!, max)}` }]
  return [
    {
      id: 'branch',
      variants: [
        { rank: 0, spans: named(BRANCH_MAX) },
        { rank: 40, spans: named(BRANCH_NARROW) },
      ],
      dropRank: 70,
    },
  ]
}

function diffSegment(model: StatusModel, icons: StatusIcons, p: Palette): Segment[] {
  const diff = model.diff
  if (diff === undefined || (diff.added === 0 && diff.removed === 0)) return []
  const spans: StatusBarSpan[] = [
    { text: icons.diff },
    { text: `+${diff.added}`, color: p.ok },
    { text: ' ' },
    { text: `${MINUS}${diff.removed}`, color: p.crit },
  ]
  return [{ id: 'diff', variants: [{ rank: 0, spans: spans.filter(s => s.text !== '') }], dropRank: 42 }]
}

/** A merged or closed PR on the branch in use is a warning: the work there already left. */
function pullRequestSegment(model: StatusModel, icons: StatusIcons, p: Palette): Segment[] {
  const pr = model.pullRequest
  if (pr === undefined) return []
  const color = { Merged: p.warn, Closed: p.crit, Draft: undefined, Open: p.ok }[pr.state]
  const isStale = pr.state === 'Merged' || pr.state === 'Closed'
  const number: StatusBarSpan = { text: `${icons.pullRequest}#${pr.number}`, href: pr.url, color }
  const full = { rank: 0, spans: [number, { text: ` ${pr.state.toLowerCase()}`, color }] }
  return [
    isStale
      ? { id: 'pr', variants: [full], dropRank: Infinity }
      : { id: 'pr', variants: [full, { rank: 33, spans: [number] }], dropRank: 65 },
  ]
}

function checksSegment(checks: ChecksSummary | undefined, icons: StatusIcons, p: Palette): Segment[] {
  if (checks === undefined || checks.total === 0) return []
  const [icon, color] =
    checks.failed > 0
      ? [icons.checksFailed, p.crit]
      : checks.pending > 0
        ? [icons.checksPending, undefined]
        : [icons.checksPassed, p.ok]
  const mark: StatusBarSpan = { text: icon, color }
  return [
    {
      id: 'checks',
      variants: [
        { rank: 0, spans: [mark, { text: `${icon ? ' ' : ''}${checks.passed}`, color }, { text: `/${checks.total}` }] },
        { rank: 38, spans: icon ? [mark] : [{ text: `${checks.passed}/${checks.total}`, color }] },
      ],
      dropRank: checks.failed > 0 ? 80 : 60,
    },
  ]
}

export function resourceSegments(model: StatusModel, o: LayoutOptions): Segment[] {
  return [
    ...identitySegment(model, o.palette),
    ...modelSegment(model),
    ...contextSegment(model, o.palette),
    ...model.limits.map((limit, i) => limitSegment(limit, o.nowMs, o.palette, i)),
  ]
}

export function workSegments(model: StatusModel, o: LayoutOptions): Segment[] {
  return [
    ...branchSegment(model, o.icons),
    ...diffSegment(model, o.icons, o.palette),
    ...pullRequestSegment(model, o.icons, o.palette),
    ...checksSegment(model.checks, o.icons, o.palette),
  ]
}

/**
 * The bar as rows for `columns` cells: resources and work share one row while
 * they fit with light trimming, and split into two rows, each fitted, when not.
 */
export function layout(model: StatusModel, columns: number, o: LayoutOptions): StatusBarSpan[][] {
  const resources = resourceSegments(model, o)
  const work = workSegments(model, o)
  if (resources.length === 0 || work.length === 0) {
    const only = resources.length > 0 ? resources : work
    return only.length === 0 ? [] : [spansOf(only, fitState(only, columns)!)]
  }
  const gap = GROUP_GAP.length
  const one = fitState([...resources, ...work], columns - gap + 2, ONE_ROW_RANK)
  if (one !== null) {
    const left = spansOf(resources, one.slice(0, resources.length))
    const right = spansOf(work, one.slice(resources.length))
    return [[...left, { text: GROUP_GAP }, ...right]]
  }
  return [spansOf(resources, fitState(resources, columns)!), spansOf(work, fitState(work, columns)!)]
}
