// fancy's one hooks module. Every call on `$` lives here, the engine follows
// `$` only within a file; what draws and parses lives in the modules beside it.
//
// The AssistantMessage renderer runs one pipeline, in order:
//   1. guard      summaries and unreadable text keep the engine's drawing
//   2. restore    markup tags taken out while the reply streamed are put back
//   3. cut        mermaid and zenuml fences are handed down the chain verbatim
//   4. tags       status tag lines become badge rows, outside every fence
//   5. markdown   the runs between parse to blocks and inlines, the markup tags included
//   6. colorize   color literals in the visible text draw on their own color
//   7. paint      the theme's palette, and the gutter bullet on the first row
// With the markdown engine off (or past MAX_TEXT) stage 5 hands each run to
// the plugins beneath and the engine, and stage 6 draws colored lines alone.

import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register, RenderElement, RenderInputOf, RenderSurface } from 'claude-code'

import type { LookPrefs } from '../types'
import { segment } from './swatches/segment'
import { colorLine } from './swatches/swatch'
import { MAX_LINES, MAX_TEXT, commandOf, contextOf, lookState, isLightName, usage } from './markup/prefs'
import { DEMO, GUIDE } from './markup/text'
import { blocks, column } from './markdown/block'
import type { Ctx } from './markdown/inline'
import { CONFIG } from './markdown/native'
import { parseCached, restoreLines, stripStreamed } from './markdown/parse'
import { paletteFor } from './markdown/themes'
import { type Row, assemble, hasColors, tagRow } from './message/rows'
import { partsOf } from './message/segments'
import { parseNumstat, toPullRequest, toStatusOptions } from './statusbar/format'
import type { DiffSize, GhPullRequest, PullRequest, StatusOptions } from './statusbar/format'
import { hintRows } from './statusbar/hint'
import { layout, paletteOf } from './statusbar/layout'
import { parseCompactAt, prettyModel, toContext, toLimits, toMinute } from './statusbar/model'
import type { ContextFill, Identity, Limit, StatusModel } from './statusbar/model'
import { readConfig } from './badges/config'
import { markerPattern } from './badges/parse'

type Message = RenderInputOf<'AssistantMessage', RenderSurface>

// ---------------------------------------------------------------- status bar

const GIT_REFRESH_MS = 5_000
const PULL_REQUEST_REFRESH_MS = 60_000
const TOUCHES_PULL_REQUEST = /\bgh\s+pr\b|\bgit\s+(push|checkout|switch)\b/

const CLOCK_REFRESH_MS = 60_000
const DEFAULT_COLUMNS = 100

const statusModel = atom({ plugin: 'fancy', key: 'status' } as const, null as StatusModel | null)

/** What the session reported and git answered, before the options pick what shows. */
type StatusInfo = {
  identity?: Identity
  model?: string
  context?: ContextFill
  limits: Limit[]
  branch?: string
  diff?: DiffSize
  pullRequest?: PullRequest
}

const info: StatusInfo = { limits: [] }
const progress = { isRefreshingGit: false, isRefreshingPullRequest: false }
let lastModel: string | undefined
let defaultBranch: string | undefined
let compactAt: number | undefined
let statusOptions: StatusOptions = toStatusOptions({})

function withoutChecks(pullRequest: PullRequest | undefined): StatusModel['pullRequest'] {
  if (pullRequest === undefined) return undefined
  const { checks: _, ...rest } = pullRequest
  return rest
}

function toModel(nowMs: number): StatusModel | null {
  const { show } = statusOptions
  const model: StatusModel = {
    identity: show.identity ? info.identity : undefined,
    model: show.model ? info.model : undefined,
    context: show.context ? info.context : undefined,
    limits: show.usage ? info.limits : [],
    branch: show.branch ? info.branch : undefined,
    diff: show.diff ? info.diff : undefined,
    pullRequest: show.pullRequest ? withoutChecks(info.pullRequest) : undefined,
    checks: show.checks ? info.pullRequest?.checks : undefined,
    minute: toMinute(nowMs),
  }
  const { minute: _, limits, ...rest } = model
  return limits.length > 0 || Object.values(rest).some(value => value !== undefined) ? model : null
}

/** Stores the model for the PromptHint hook; writing it redraws the hint, at most once a minute when idle. */
async function paint($: EngineInterface): Promise<void> {
  const model = toModel(await $.clock.now())
  const text = JSON.stringify(model)

  if (text !== lastModel) {
    lastModel = text
    await update($, statusModel, () => model)
  }
}

async function readIdentity($: EngineInterface): Promise<Identity | undefined> {
  const name = (await $.env.get('TIAMATI_AGENT'))?.trim()
  if (!name) return undefined
  const color = statusOptions.identityColors.get(name.toLowerCase())
  return color === undefined ? { name } : { name, color }
}

async function readSession($: EngineInterface): Promise<void> {
  info.model = prettyModel(await $.session.model().catch(() => undefined))
}

async function git($: EngineInterface, args: readonly string[]): Promise<string | undefined> {
  const { exitCode, stdout } = await $.process.run(['git', ...args], { timeoutMs: 5_000 })

  return exitCode === 0 ? stdout.trim() : undefined
}

async function readBranchAndDiff($: EngineInterface): Promise<boolean> {
  if ((await git($, ['rev-parse', '--show-toplevel'])) === undefined) {
    const wasInRepository = info.branch !== undefined
    info.branch = undefined
    info.diff = undefined
    info.pullRequest = undefined

    return wasInRepository
  }

  defaultBranch ??= (await git($, ['symbolic-ref', '--short', 'refs/remotes/origin/HEAD'])) ?? 'origin/main'

  const branch =
    (await git($, ['branch', '--show-current'])) || (await git($, ['rev-parse', '--short', 'HEAD']))
  const hasBranchChanged = branch !== info.branch

  info.branch = branch
  info.diff = statusOptions.show.diff ? await readDiff($, defaultBranch) : undefined

  if (hasBranchChanged) {
    info.pullRequest = undefined
  }

  return hasBranchChanged
}

/** Lines changed against the merge base with the default branch, uncommitted work included. */
async function readDiff($: EngineInterface, baseBranch: string): Promise<DiffSize | undefined> {
  const base = (await git($, ['merge-base', 'HEAD', baseBranch])) ?? 'HEAD'
  const numstat = await git($, ['diff', '--numstat', base])

  return numstat === undefined ? undefined : parseNumstat(numstat)
}

/** Reads branch and diff size; resolves whether the branch changed since the last read. */
async function refreshGit($: EngineInterface): Promise<boolean> {
  if (progress.isRefreshingGit) {
    return false
  }

  progress.isRefreshingGit = true

  try {
    return await readBranchAndDiff($)
  } catch (error) {
    $.ui.log(`fancy: status bar: git failed: ${String(error)}`, { to: 'debug' })

    return false
  } finally {
    progress.isRefreshingGit = false
    await paint($)
  }
}

async function refreshPullRequest($: EngineInterface): Promise<void> {
  const branch = info.branch
  const isWanted = statusOptions.show.pullRequest || statusOptions.show.checks

  if (!isWanted || branch === undefined || `origin/${branch}` === defaultBranch || progress.isRefreshingPullRequest) {
    return
  }

  progress.isRefreshingPullRequest = true

  try {
    const { exitCode, stdout } = await $.process.run(
      ['gh', 'pr', 'view', '--json', 'number,url,state,isDraft,statusCheckRollup'],
      { timeoutMs: 15_000 },
    )

    if (info.branch === branch) {
      info.pullRequest = exitCode === 0 ? toPullRequest(JSON.parse(stdout) as GhPullRequest) : undefined
    }
  } catch (error) {
    $.ui.log(`fancy: status bar: gh failed: ${String(error)}`, { to: 'debug' })
  } finally {
    progress.isRefreshingPullRequest = false
    await paint($)
  }
}

async function refreshAll($: EngineInterface): Promise<void> {
  await readSession($).catch(() => undefined)
  await refreshGit($)
  await refreshPullRequest($)
}

async function refreshGitThenPullRequestOnBranchChange($: EngineInterface): Promise<void> {
  if (await refreshGit($)) {
    await refreshPullRequest($)
  }
}

// ---------------------------------------------------------------- replies

const prefs = atom({ plugin: 'fancy', key: 'prefs' } as const, {})
const isLightTheme = atom({ plugin: 'fancy', key: 'isLightTheme' } as const, false)
const written = atom({ plugin: 'fancy', key: 'lines' } as const, {})

let look = lookState({})
let tags = readConfig({})
let marker = markerPattern(tags.tags.keys())
const lastText = new Map<string, string>()

async function loadLight($: EngineInterface): Promise<void> {
  const rows = await $.config.list()
  const theme = rows.find(row => row.key === 'theme')
  await update($, isLightTheme, () => isLightName(theme?.value))
}

/** The saved prefs and the /config theme, read into the session's state. */
async function hydrate($: EngineInterface): Promise<void> {
  const stored = (await $.store.get('prefs')) as Partial<LookPrefs> | undefined
  if (stored !== undefined) await update($, prefs, () => stored)
  await loadLight($).catch(() => undefined)
}

/** The prefs in force: what `/fancy` changed over the `/config` defaults. */
async function current($: EngineInterface): Promise<LookPrefs> {
  return { ...look.defaults, ...(await read($, prefs)) }
}

/** The drawing context in force, or undefined while the markdown engine is off. */
async function contextFor($: EngineInterface, t: Ctx['t'], columns: number | undefined): Promise<Ctx | undefined> {
  return contextOf(await current($), await read($, isLightTheme), t, columns)
}

async function draw($: EngineInterface, e: Message, next: (e: Message) => Promise<RenderElement>, text: string) {
  const t = $.ui.resolve(e)
  const { Text } = t
  const ctx = text.length > MAX_TEXT ? undefined : await contextFor($, t, e.viewport?.columns)
  const t0 = performance.now()
  const parts = partsOf(ctx === undefined ? text : restoreLines(text, await read($, written)), marker)

  if (parts.every(p => p.kind === 'foreign')) return next(e)
  if (ctx === undefined && !parts.some(p => p.kind === 'tag') && !hasColors(parts)) return next(e)

  const rows: Row[] = []
  const own = (element: RenderElement, gap: boolean) => rows.push({ element, gap, isOwn: true })
  const below = async (part: string, gap: boolean) => {
    const isFirstOfReply = rows.length === 0 && e.props.isFirstOfReply
    rows.push({ element: await next({ ...e, props: { ...e.props, text: part, isFirstOfReply } }), gap, isOwn: false })
  }
  const inner: Ctx | undefined = ctx && { ...ctx, width: ctx.width - 2 }

  for (const part of parts) {
    if (part.kind === 'foreign') {
      await below(part.text, part.gap)
    } else if (part.kind === 'tag') {
      own(tagRow(part, tags, t, inner), part.gap)
    } else if (inner) {
      own(column(parseCached(part.text), inner), part.gap)
    } else {
      const pieces = segment(part.text)
      if (!pieces.some(s => s.kind === 'line')) {
        await below(part.text, part.gap)
        continue
      }
      for (const [k, s] of pieces.entries()) {
        const gap = k === 0 && part.gap
        if (s.kind === 'markdown') await below(s.text, gap)
        else own(s.kind === 'blank' ? <Text> </Text> : colorLine(s, t), gap)
      }
    }
  }

  if (CONFIG.debugTiming) {
    const repeat = lastText.get(e.requestId) === text
    lastText.set(e.requestId, text)
    $.ui.log(
      `fancy ${e.requestId} chars=${text.length} parts=${parts.length} draw=${(performance.now() - t0).toFixed(2)}ms sameText=${repeat}`,
      { to: 'debug' },
    )
  }

  return assemble(rows, t, { surface: e.surface, isFirstOfReply: e.props.isFirstOfReply, ctx })
}

// ---------------------------------------------------------------- register

export const register: Register = (on, options) => {
  statusOptions = toStatusOptions(options)
  look = lookState(options)
  tags = readConfig(options)
  marker = markerPattern(tags.tags.keys())

  // One start for both features; a failure in the markup half leaves the status bar starting.
  on('session.start', async ($, e, next) => {
    await hydrate($).catch(error => $.ui.log(`fancy: prefs not read: ${String(error)}`, { to: 'debug' }))
    await $.command
      .register({
        name: 'fancy',
        description: 'Colorful replies: switch on or off, pick a theme, show a demo',
        argumentHint: '[on|off|demo|<theme>|dark|light|auto|teach on|off]',
      })
      .catch(error => $.ui.log(`fancy: /fancy not registered: ${String(error)}`, { to: 'debug' }))

    const started = await next(e)

    defaultBranch = undefined
    $.ui.status(undefined)
    compactAt = parseCompactAt(await $.env.get('CLAUDE_AUTOCOMPACT_PCT_OVERRIDE'))
    info.identity = await readIdentity($)
    const usage = await $.session.usage()
    info.limits = toLimits(usage.rateLimits)
    info.context = toContext(usage.context, compactAt)
    await paint($)

    $.clock.after(0, () => void refreshAll($))
    $.clock.every(GIT_REFRESH_MS, () => void refreshGitThenPullRequestOnBranchChange($))
    $.clock.every(PULL_REQUEST_REFRESH_MS, () => void refreshPullRequest($))
    $.clock.every(CLOCK_REFRESH_MS, () => void paint($))

    return started
  })

  // `/clear` (and a resume or fork) goes on under a new session whose state
  // starts empty, and no `session.start` fires for it: read the prefs again.
  on('classic.SessionStart', async ($, e, next) => {
    await hydrate($).catch(() => undefined)
    return next(e)
  })

  // Only watches: the new theme picks the light or dark palette, and the
  // change itself goes on untouched.
  on('config.set', { key: 'theme' }, async ($, e, next) => {
    await update($, isLightTheme, () => isLightName(e.value))
    return next(e)
  })

  on('prompt.compose', async ($, e, next) => {
    const result = await next(e)
    const p = await current($)
    const isDrawn = e.surfaces.length > 0 && !e.traits.includes('bare') && !e.traits.includes('print')
    if (!look.canTeach || !p.isEnabled || !p.isTeaching || !isDrawn) return result
    return { sections: [...result.sections, { id: 'fancy:guide', text: GUIDE, scope: 'session' as const }] }
  })

  on('command.run', { command: 'fancy' }, async ($, e) => {
    const command = commandOf(e.args)
    if (command !== undefined && 'demo' in command) return { text: DEMO }
    if (command !== undefined && 'change' in command) {
      await $.store.set('prefs', await update($, prefs, p => ({ ...p, ...command.change })))
    }
    const text = usage(look, await current($), await read($, isLightTheme))
    return { text: command !== undefined && 'unknown' in command ? `Unknown option \`${command.unknown}\`.\n\n${text}` : text }
  })

  // While a reply streams the engine draws it line by line with its own
  // renderer; take the tags out there so they never flash, and let the
  // finished block draw through the pipeline.
  // The engine hands AssistantMessage the text as shown, so each line taken
  // apart here is remembered and put back before the finished block parses.
  const streams = new Map<string, { inFence: boolean; changed: Record<string, string> }>()
  on('classic.MessageDisplay', async ($, e, next) => {
    const result = await next(e)
    if (!(await current($)).isEnabled) return result
    const stream = streams.get(e.message_id) ?? { inFence: false, changed: {} }
    const [text, inFence, changed] = stripStreamed(result.displayContent ?? e.delta, stream.inFence)
    stream.inFence = inFence
    Object.assign(stream.changed, changed)
    if (!e.final) {
      // A reply interrupted mid-stream never sends its final delta.
      if (!streams.has(e.message_id) && streams.size >= 50) streams.delete(streams.keys().next().value!)
      streams.set(e.message_id, stream)
    } else {
      streams.delete(e.message_id)
      if (Object.keys(stream.changed).length > 0) {
        await update($, written, lines => {
          const merged = Object.entries({ ...lines, ...stream.changed })
          return Object.fromEntries(merged.slice(-MAX_LINES))
        })
      }
    }
    return { ...result, displayContent: text }
  })

  on('session.measure', async ($, e, next) => {
    info.limits = toLimits(e.rateLimits)
    info.context = toContext(e.context, compactAt)
    await paint($)

    return next(e)
  })

  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    const result = await next(e)

    if (TOUCHES_PULL_REQUEST.test(e.command)) {
      $.clock.after(0, () => void refreshAll($))
    }

    return result
  })

  on('turn.complete', ($, e, next) => {
    $.clock.after(0, () => void refreshAll($))

    return next(e)
  })

  on('ui.render', { component: 'PromptHint' }, async ($, e, next) => {
    const model = await read($, statusModel)

    if (model === null) {
      return next(e)
    }

    const rows = layout(model, e.viewport?.columns ?? DEFAULT_COLUMNS, {
      icons: statusOptions.icons,
      palette: paletteOf(await read($, isLightTheme)),
      nowMs: model.minute * CLOCK_REFRESH_MS,
    })

    return rows.length === 0 ? next(e) : hintRows(await next(e), rows, $.ui.resolve(e))
  })

  on('ui.render', { component: 'AssistantMessage' }, async ($, e, next) => {
    const text: unknown = e.props.text
    if (typeof text !== 'string' || e.props.isSummary) return next(e)
    try {
      return await draw($, e, next, text)
    } catch (error) {
      $.ui.log(`fancy: AssistantMessage left to the engine: ${String(error)}`, { to: 'debug' })
      return next(e)
    }
  })

  on('ui.render', { component: 'CommandOutput', props: { command: 'fancy' } }, async ($, e, next) => {
    if (e.props.isErrored) return next(e)
    const isLight = await read($, isLightTheme)
    const t = $.ui.resolve(e)
    const ctx = (await contextFor($, t, e.viewport?.columns)) ?? {
      t,
      p: paletteFor('vivid', isLight),
      isLight,
      width: Math.max(20, e.viewport?.columns ?? 80),
      listDepth: 0,
      isQuoted: false,
    }
    const { Box } = ctx.t
    return (
      <Box flexDirection="column" paddingLeft={2}>
        {blocks(parseCached(e.props.text.replace(/^fancy:\s*/, '')), { ...ctx, width: ctx.width - 2 })}
      </Box>
    )
  })
}
