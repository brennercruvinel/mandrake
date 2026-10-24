import type { StatusBarSpan } from '../../types'

/**
 * A segment of the bar and the forms it can shrink to. Each later variant
 * carries the rank at which the fitter reaches for it; `dropRank` is when the
 * segment leaves the row, never when it is Infinity. Lower ranks give first,
 * whatever segment they belong to, so detail goes before content does.
 */
export type Segment = {
  id: string
  variants: readonly { rank: number; spans: readonly StatusBarSpan[] }[]
  dropRank: number
}

type Step = { segment: number; to: number; rank: number }

const GAP = 2
const WIDE = /[\u{1100}-\u{115F}\u{2E80}-\u{A4CF}\u{AC00}-\u{D7A3}\u{F900}-\u{FAFF}\u{FE30}-\u{FE4F}\u{FF00}-\u{FF60}\u{FFE0}-\u{FFE6}\u{1F300}-\u{1FAFF}]/u
const ZERO = /[\u{0300}-\u{036F}\u{200B}-\u{200F}\u{FE00}-\u{FE0F}]/u

/** Terminal cells a string takes: wide East Asian and emoji count two, combining marks none. */
export function cellWidth(text: string): number {
  let width = 0
  for (const char of text) width += ZERO.test(char) ? 0 : WIDE.test(char) ? 2 : 1
  return width
}

export const spansWidth = (spans: readonly StatusBarSpan[]): number =>
  spans.reduce((sum, span) => sum + cellWidth(span.text), 0)

/** The variant index each segment is in, `-1` once dropped. */
type State = number[]

function rowWidth(segments: readonly Segment[], state: State): number {
  let width = 0
  let count = 0
  segments.forEach((segment, i) => {
    if (state[i]! < 0) return
    width += spansWidth(segment.variants[state[i]!]!.spans)
    count += 1
  })
  return width + Math.max(0, count - 1) * GAP
}

function stepsOf(segments: readonly Segment[]): Step[] {
  const steps: Step[] = []
  segments.forEach((segment, i) => {
    segment.variants.forEach((variant, to) => to > 0 && steps.push({ segment: i, to, rank: variant.rank }))
    if (Number.isFinite(segment.dropRank)) steps.push({ segment: i, to: -1, rank: segment.dropRank })
  })
  return steps.sort((a, b) => a.rank - b.rank || a.segment - b.segment)
}

/**
 * The richest state that fits `width`, shrinking by rank; with `maxRank`, null
 * when it takes a step above it to fit. Past every step the row is returned
 * as is and the surface cuts its end.
 */
export function fitState(segments: readonly Segment[], width: number, maxRank = Infinity): State | null {
  const state: State = segments.map(() => 0)
  if (rowWidth(segments, state) <= width) return state
  const steps = stepsOf(segments)
  for (let i = 0; i < steps.length; ) {
    const rank = steps[i]!.rank
    if (rank > maxRank) return null
    // One rank is one move: equal ranks shrink together, so twin segments stay alike.
    for (; i < steps.length && steps[i]!.rank === rank; i++) {
      const step = steps[i]!
      const current = state[step.segment]!
      if (current >= 0 && (step.to < 0 || step.to > current)) state[step.segment] = step.to
    }
    if (rowWidth(segments, state) <= width) return state
  }
  return Number.isFinite(maxRank) ? null : state
}

export function spansOf(segments: readonly Segment[], state: State): StatusBarSpan[] {
  const spans: StatusBarSpan[] = []
  segments.forEach((segment, i) => {
    if (state[i]! < 0) return
    if (spans.length > 0) spans.push({ text: ' '.repeat(GAP) })
    spans.push(...segment.variants[state[i]!]!.spans)
  })
  return spans
}

export function fit(segments: readonly Segment[], width: number): StatusBarSpan[] {
  return spansOf(segments, fitState(segments, width)!)
}
