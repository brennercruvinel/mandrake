/**
 * One piece of the status line; `color` is a theme key or a raw color, absent for dim text,
 * `bg` paints the cells behind it and `href` makes it a link.
 */
export type StatusBarSpan = { text: string; color?: string; bg?: string; href?: string }

export type PullRequestState = 'Merged' | 'Draft' | 'Open' | 'Closed'

export type ChecksSummary = { passed: number; failed: number; pending: number; total: number }

export type PullRequest = { number: number; url: string; state: PullRequestState; checks: ChecksSummary }

export type DiffSize = { added: number; removed: number }

/** A rate-limit window: its short label, how full it is and when it empties, in epoch milliseconds. */
export type Limit = { kind: string; label: string; percent: number; resetsAt?: number }

/** The context fill measured against the point where the session compacts, not the whole window. */
export type ContextFill = { tokens?: number; budget: number }

export type Identity = { name: string; color?: string }

/** Everything the status bar draws, as plain data; the layout turns it into rows for a width. */
export type StatusModel = {
  identity?: Identity
  model?: string
  context?: ContextFill
  limits: Limit[]
  branch?: string
  diff?: DiffSize
  pullRequest?: Omit<PullRequest, 'checks'>
  checks?: ChecksSummary
  /** The minute the model was painted in; the reset countdowns are relative to it. */
  minute: number
}

export type LookPrefs = {
  /** Draw replies with the markdown engine; off hands the markdown back to the engine. */
  isEnabled: boolean
  /** Teach the model the markup through the system prompt. */
  isTeaching: boolean
  /** A palette name from THEMES. */
  theme: string
  /** `auto` follows the /config theme; the others force a background. */
  background: 'auto' | 'dark' | 'light'
}

declare module 'claude-code' {
  interface PluginState {
    fancy: {
      /** What the status bar draws, laid out per width when drawn; null until the first paint. */
      status: StatusModel | null
      /** The prefs `/fancy` changed, over the defaults `/config` sets. */
      prefs: Partial<LookPrefs>
      /** Whether the /config theme is a light one, read at start and on change. */
      isLightTheme: boolean
      /**
       * Streamed lines whose tags were taken out for the engine's live drawing,
       * by the line as shown (trimmed) to the line as written, so the finished
       * block draws with its tags again.
       */
      lines: Record<string, string>
    }
  }
}
