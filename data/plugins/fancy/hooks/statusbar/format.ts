import type { PluginOptions } from 'claude-code'

import type { ChecksSummary, DiffSize, PullRequest, PullRequestState } from '../../types'
import { parseIdentityColors } from './model'

export type { ChecksSummary, DiffSize, PullRequest, PullRequestState } from '../../types'

/** One entry of `gh pr view --json statusCheckRollup`: a CheckRun or a StatusContext. */
export type RollupEntry = {
  __typename?: string
  status?: string | null
  conclusion?: string | null
  state?: string | null
}

export type GhPullRequest = {
  number: number
  url: string
  state: string
  isDraft: boolean
  statusCheckRollup?: RollupEntry[] | null
}

const PASSING_CONCLUSIONS = new Set(['SUCCESS', 'NEUTRAL', 'SKIPPED'])
const PENDING_STATES = new Set(['PENDING', 'EXPECTED'])

/** Sums the `git diff --numstat` output; binary files (`-\t-`) count as zero. */
export function parseNumstat(output: string): DiffSize {
  let added = 0
  let removed = 0

  for (const line of output.split('\n')) {
    const [addedText, removedText] = line.split('\t')
    added += Number.parseInt(addedText ?? '', 10) || 0
    removed += Number.parseInt(removedText ?? '', 10) || 0
  }

  return { added, removed }
}

function classifyCheck(entry: RollupEntry): 'passed' | 'failed' | 'pending' {
  if (entry.__typename === 'StatusContext' || (entry.status == null && entry.state != null)) {
    const state = entry.state ?? ''

    if (state === 'SUCCESS') {
      return 'passed'
    }

    return PENDING_STATES.has(state) ? 'pending' : 'failed'
  }

  if (entry.status !== 'COMPLETED') {
    return 'pending'
  }

  return PASSING_CONCLUSIONS.has(entry.conclusion ?? '') ? 'passed' : 'failed'
}

export function summarizeChecks(rollup: readonly RollupEntry[]): ChecksSummary {
  const summary: ChecksSummary = { passed: 0, failed: 0, pending: 0, total: rollup.length }

  for (const entry of rollup) {
    summary[classifyCheck(entry)] += 1
  }

  return summary
}

export function toPullRequest(pullRequest: GhPullRequest): PullRequest {
  const state: PullRequestState =
    pullRequest.state === 'MERGED'
      ? 'Merged'
      : pullRequest.state === 'CLOSED'
        ? 'Closed'
        : pullRequest.isDraft
          ? 'Draft'
          : 'Open'

  return {
    number: pullRequest.number,
    url: pullRequest.url,
    state,
    checks: summarizeChecks(pullRequest.statusCheckRollup ?? []),
  }
}

export type StatusSections = {
  identity: boolean
  model: boolean
  context: boolean
  usage: boolean
  branch: boolean
  diff: boolean
  pullRequest: boolean
  checks: boolean
}

export type StatusIcons = {
  branch: string
  diff: string
  pullRequest: string
  checksPassed: string
  checksFailed: string
  checksPending: string
}

export type StatusOptions = {
  show: StatusSections
  icons: StatusIcons
  /** Colors by identity name, lowercased. */
  identityColors: Map<string, string>
}

/** Glyphs every monospace font carries; a Nerd Font set goes in through the icon options. */
const DEFAULT_ICONS: StatusIcons = {
  branch: '',
  diff: '',
  pullRequest: '',
  checksPassed: '✓',
  checksFailed: '✗',
  checksPending: '◌',
}
const NO_ICON = 'none'
const DEFAULT_IDENTITY_COLORS = 'quimera:#39C5BB,tiamati:#8C7BFF,toth:#5B8DEF'

/**
 * Reads the mod's `userConfig` values. A missing section stays shown. An empty icon
 * draws its default, since Claude Code's install dialog shows text fields blank
 * whatever their default; `none` draws no icon.
 */
export function toStatusOptions(options: PluginOptions): StatusOptions {
  const isShown = (key: string): boolean => options[key] !== false
  const iconFor = (key: string, fallback: string): string => {
    const value = options[key]

    if (typeof value !== 'string' || value === '') {
      return fallback
    }

    return value.trim().toLowerCase() === NO_ICON ? '' : value
  }

  return {
    show: {
      identity: isShown('showIdentity'),
      model: isShown('showModel'),
      context: isShown('showContext'),
      usage: isShown('showUsage'),
      branch: isShown('showBranch'),
      diff: isShown('showDiff'),
      pullRequest: isShown('showPullRequest'),
      checks: isShown('showChecks'),
    },
    icons: {
      branch: iconFor('iconBranch', DEFAULT_ICONS.branch),
      diff: iconFor('iconDiff', DEFAULT_ICONS.diff),
      pullRequest: iconFor('iconPullRequest', DEFAULT_ICONS.pullRequest),
      checksPassed: iconFor('iconChecksPassed', DEFAULT_ICONS.checksPassed),
      checksFailed: iconFor('iconChecksFailed', DEFAULT_ICONS.checksFailed),
      checksPending: iconFor('iconChecksPending', DEFAULT_ICONS.checksPending),
    },
    identityColors: parseIdentityColors(
      typeof options.identityColors === 'string' && options.identityColors.trim() !== ''
        ? options.identityColors
        : DEFAULT_IDENTITY_COLORS,
    ),
  }
}
