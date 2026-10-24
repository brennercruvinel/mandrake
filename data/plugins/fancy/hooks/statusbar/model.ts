import type { SessionContextUsage, SessionRateLimit } from 'claude-code'

import type { ContextFill, Limit } from '../../types'

export type { ContextFill, Identity, Limit, StatusModel } from '../../types'

export type Level = 'calm' | 'warn' | 'crit'

const MINUTE_MS = 60_000
const HOUR_MS = 60 * MINUTE_MS
const DAY_MS = 24 * HOUR_MS

const WINDOWS: Record<string, { label: string; lengthMs?: number }> = {
  five_hour: { label: '5h', lengthMs: 5 * HOUR_MS },
  seven_day: { label: '7d', lengthMs: 7 * DAY_MS },
  spend_limit: { label: '$' },
}

const CONTEXT_WARN = 0.6
const CONTEXT_CRIT = 0.85
const LIMIT_WARN_PERCENT = 75
const LIMIT_CRIT_PERCENT = 90
/** Below this share of a window gone by, a projection says more about the clock than the pace. */
const MIN_ELAPSED_TO_PROJECT = 0.1

export const toMinute = (nowMs: number): number => Math.floor(nowMs / MINUTE_MS)

function toEpochMs(value: string | number | undefined): number | undefined {
  if (value === undefined) return undefined
  if (typeof value === 'number') return value < 1e12 ? value * 1000 : value
  const numeric = Number(value)
  if (Number.isFinite(numeric)) return toEpochMs(numeric)
  const parsed = Date.parse(value)
  return Number.isNaN(parsed) ? undefined : parsed
}

export function toLimits(rateLimits: readonly SessionRateLimit[]): Limit[] {
  return rateLimits.map(limit => ({
    kind: limit.kind,
    label: WINDOWS[limit.kind]?.label ?? limit.kind,
    percent: limit.percentUsed,
    resetsAt: toEpochMs(limit.resetsAt),
  }))
}

/**
 * `compactAt` is the share of the window where the session compacts
 * (CLAUDE_AUTOCOMPACT_PCT_OVERRIDE); unknown, the whole window is the budget.
 */
export function toContext(context: SessionContextUsage, compactAt: number | undefined): ContextFill | undefined {
  if (!(context.window > 0)) return undefined
  const share = compactAt !== undefined && compactAt > 0 && compactAt <= 1 ? compactAt : 1
  return { tokens: context.tokens, budget: Math.round(context.window * share) }
}

/** `80` or `0.8` from the environment variable; anything else is unknown. */
export function parseCompactAt(value: string | undefined): number | undefined {
  const n = Number.parseFloat(value ?? '')
  if (!Number.isFinite(n) || n <= 0) return undefined
  return n > 1 ? n / 100 : n
}

/** `claude-opus-5-5[1m]` and `opus` read as `Opus 5.5` and `Opus`; a name already spelled out stays. */
export function prettyModel(id: string | undefined): string | undefined {
  const name = id?.replace(/\[[^\]]*\]$/, '').trim()
  if (!name) return undefined
  const match = /^(?:claude-)?([a-z]+)-(\d+)(?:-(\d{1,2}))?(?:-\d{8})?$/i.exec(name)
  if (match) {
    const [, family, major, minor] = match
    return `${capitalize(family!)} ${minor === undefined ? major : `${major}.${minor}`}`
  }
  return /^[a-z]+$/.test(name) ? capitalize(name) : name
}

const capitalize = (word: string): string => word.charAt(0).toUpperCase() + word.slice(1)

/** `quimera:#39C5BB,tiamati:#8C7BFF` by name, lowercased. */
export function parseIdentityColors(value: unknown): Map<string, string> {
  const colors = new Map<string, string>()
  if (typeof value !== 'string') return colors
  for (const pair of value.split(',')) {
    const [name, color] = pair.split(':').map(part => part.trim())
    if (name && color) colors.set(name.toLowerCase(), color)
  }
  return colors
}

export function contextLevel(fill: ContextFill): Level {
  const share = (fill.tokens ?? 0) / fill.budget
  return share >= CONTEXT_CRIT ? 'crit' : share >= CONTEXT_WARN ? 'warn' : 'calm'
}

/**
 * Where the window would end at the pace so far, in percent; undefined while
 * too little of it has gone by or its length or reset is unknown.
 */
export function projectedPercent(limit: Limit, nowMs: number): number | undefined {
  const lengthMs = WINDOWS[limit.kind]?.lengthMs
  if (lengthMs === undefined || limit.resetsAt === undefined) return undefined
  const elapsed = 1 - (limit.resetsAt - nowMs) / lengthMs
  if (elapsed < MIN_ELAPSED_TO_PROJECT || elapsed > 1) return undefined
  return limit.percent / elapsed
}

/** Red near the cap; amber when the pace runs out before the reset, or past 75% when no pace is known. */
export function limitLevel(limit: Limit, nowMs: number): Level {
  if (limit.percent >= LIMIT_CRIT_PERCENT) return 'crit'
  const projected = projectedPercent(limit, nowMs)
  if (projected !== undefined) return projected >= 100 ? 'warn' : 'calm'
  return limit.percent >= LIMIT_WARN_PERCENT ? 'warn' : 'calm'
}

/** `37k`, `800k`, `1M`, `1.2M`. */
export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`
  if (n >= 1_000) return `${Math.round(n / 1_000)}k`
  return `${Math.round(n)}`
}

/** Time left as `3d6h`, `2h14`, `14m`, `now`. */
export function formatIn(ms: number): string {
  if (ms <= 0) return 'now'
  const minutes = Math.ceil(ms / MINUTE_MS)
  if (minutes < 60) return `${minutes}m`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h${String(minutes % 60).padStart(2, '0')}`
  return `${Math.floor(hours / 24)}d${hours % 24}h`
}
