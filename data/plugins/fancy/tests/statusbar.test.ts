import type { On, ProcessRunResult, RenderElement, RenderPropsOf } from 'claude-code'
import { describe, expect, mock, test } from 'claude-code/testing'
import type { Engine } from 'claude-code/testing'

import { parseNumstat, summarizeChecks, toPullRequest, toStatusOptions } from '../hooks/statusbar/format'

const ok = (stdout: string): ProcessRunResult => ({
  exitCode: 0,
  stdout,
  stderr: '',
  isStdoutTruncated: false,
  isStderrTruncated: false,
})

const failed: ProcessRunResult = { ...ok(''), exitCode: 1 }

const PULL_REQUEST_URL = 'https://github.com/acme/app/pull/42'

const PULL_REQUEST = JSON.stringify({
  number: 42,
  url: PULL_REQUEST_URL,
  state: 'OPEN',
  isDraft: false,
  statusCheckRollup: [
    { __typename: 'CheckRun', status: 'COMPLETED', conclusion: 'SUCCESS' },
    { __typename: 'CheckRun', status: 'COMPLETED', conclusion: 'SKIPPED' },
    { __typename: 'CheckRun', status: 'COMPLETED', conclusion: 'FAILURE' },
    { __typename: 'StatusContext', state: 'SUCCESS' },
  ],
})

const HINT: RenderPropsOf['PromptHint'] = { isDraft: false, isWorking: false, hint: '? for shortcuts' }

const ENV: Record<string, string> = { TIAMATI_AGENT: 'quimera', CLAUDE_AUTOCOMPACT_PCT_OVERRIDE: '80' }

type World = { branch: string; commands?: string[]; env?: Record<string, string> }

/** Answers git, gh, the environment and the session the way a feature branch with an open PR would. */
function stubWorld(on: On, { branch, commands = [], env = ENV }: World): void {
  on('process.run', ($, e) => {
    const command = e.argv.join(' ')
    commands.push(command)

    if (command === 'gh pr view --json number,url,state,isDraft,statusCheckRollup') {
      return { value: ok(PULL_REQUEST) }
    }

    const answers: Record<string, ProcessRunResult> = {
      'git rev-parse --show-toplevel': ok('/repo\n'),
      'git symbolic-ref --short refs/remotes/origin/HEAD': ok('origin/main\n'),
      'git branch --show-current': ok(`${branch}\n`),
      'git merge-base HEAD origin/main': ok('abc123\n'),
      'git diff --numstat abc123': ok('40\t10\tapp/Foo.php\n5\t2\tresources/js/foo.tsx\n-\t-\tlogo.png\n'),
    }

    return { value: answers[command] ?? failed }
  })
  on('env.get', ($, e) => ({ value: env[e.name] }))
  on('session.model', () => ({ value: 'claude-opus-5-5[1m]' }))
  on('session.start', ($, e) => ({ cwd: e.cwd }))
  on('session.usage', () => ({
    value: {
      startedAt: 0,
      context: { window: 1_000_000, tokens: 37_000 },
      rateLimits: [
        { kind: 'five_hour', percentUsed: 23.4 },
        { kind: 'seven_day', percentUsed: 40.6 },
      ],
    },
  }))
  on('session.measure', ($, e) => ({ changed: e.changed }))
  on('ui.status', () => ({ value: undefined }))
  on('ui.log', () => ({ value: undefined }))
  on('ui.render', { component: 'PromptHint' }, ($, e) => h('Text', null, e.props.hint) as RenderElement)
}

async function startSession($: Engine, on: On): Promise<void> {
  const clock = mock.clock(on)

  await $.session.start({ cwd: '/repo', surface: 'terminal', isInteractive: true })
  await clock.advance(0)
}

/** The status rows the mod drew, as text, for a terminal `columns` wide. */
async function statusRows($: Engine, columns: number, surface: 'terminal' | 'desktop' = 'terminal'): Promise<string[]> {
  const ui = await $.ui.mount({
    plugin: 'fancy',
    surface,
    component: 'PromptHint',
    props: HINT,
    viewport: { columns, rows: 40 },
  } as Parameters<Engine['ui']['mount']>[0])
  const rows = (await ui.findAll({ type: 'Text' })).filter(t => t.props.wrap === 'truncate-end').map(t => t.text)
  await ui.unmount()
  return rows
}

describe('git and gh', () => {
  test('sums numstat lines and skips binary files', () => {
    expect(parseNumstat('40\t10\ta\n5\t2\tb\n-\t-\tc.png\n')).toEqual({ added: 45, removed: 12 })
  })

  test('counts skipped and neutral checks as passed, running ones as pending', () => {
    expect(
      summarizeChecks([
        { __typename: 'CheckRun', status: 'COMPLETED', conclusion: 'SUCCESS' },
        { __typename: 'CheckRun', status: 'COMPLETED', conclusion: 'NEUTRAL' },
        { __typename: 'CheckRun', status: 'IN_PROGRESS', conclusion: null },
        { __typename: 'StatusContext', state: 'PENDING' },
        { __typename: 'StatusContext', state: 'ERROR' },
      ]),
    ).toEqual({ passed: 2, failed: 1, pending: 2, total: 5 })
  })

  test('maps the PR state, draft included', () => {
    const pullRequest = { number: 42, url: PULL_REQUEST_URL }

    expect(toPullRequest({ ...pullRequest, state: 'OPEN', isDraft: true }).state).toBe('Draft')
    expect(toPullRequest({ ...pullRequest, state: 'MERGED', isDraft: false }).state).toBe('Merged')
    expect(toPullRequest({ ...pullRequest, state: 'CLOSED', isDraft: false }).state).toBe('Closed')
    expect(toPullRequest({ ...pullRequest, state: 'OPEN', isDraft: false }).state).toBe('Open')
  })
})

describe('options', () => {
  test('shows every section and draws glyphs any monospace font has by default', () => {
    const { show, icons, identityColors } = toStatusOptions({})

    expect(Object.values(show).every(Boolean)).toBe(true)
    expect(icons).toEqual({ branch: '', diff: '', pullRequest: '', checksPassed: '✓', checksFailed: '✗', checksPending: '◌' })
    expect(identityColors.get('quimera')).toBe('#39C5BB')
  })

  test('draws the configured icons, the default for an empty one and none for none', () => {
    const { icons } = toStatusOptions({ iconBranch: 'B>', iconChecksPassed: '', iconChecksFailed: ' None ' })

    expect([icons.branch, icons.checksPassed, icons.checksFailed]).toEqual(['B>', '✓', ''])
  })

  test('takes identity colors from the option over the defaults', () => {
    const colors = toStatusOptions({ identityColors: 'quimera:red' }).identityColors

    expect([colors.get('quimera'), colors.get('tiamati')]).toEqual(['red', undefined])
  })
})

describe('prompt hint', () => {
  test('is left alone until there is something to show', async ($, on) => {
    stubWorld(on, { branch: 'main' })

    const ui = await $.ui.mount({ plugin: 'fancy', surface: 'terminal', component: 'PromptHint', props: HINT })

    expect((await ui.find({ type: 'Text', text: '? for shortcuts' }))?.text).toBe('? for shortcuts')
    expect(await ui.find({ type: 'Text', text: /quimera/ })).toBeUndefined()
  })

  test('draws who, model, context, limits and git in one row beside the engine hint', async ($, on) => {
    stubWorld(on, { branch: 'feat/status-bar' })
    await startSession($, on)

    for (const surface of ['terminal', 'desktop'] as const) {
      expect(await statusRows($, 140, surface)).toEqual([
        '◆ quimera  Opus 5.5  ▌          37k/800k  5h 23%  7d 41%   feat/status-bar  +45 −12  #42 open  ✗ 3/4',
      ])
    }

    const ui = await $.ui.mount({ plugin: 'fancy', surface: 'terminal', component: 'PromptHint', props: HINT })
    expect(await ui.find({ type: 'Text', text: '? for shortcuts' })).toBeDefined()
    expect((await ui.find({ type: 'Link', text: '#42' }))?.props.href).toBe(PULL_REQUEST_URL)
    expect((await ui.find({ type: 'Text', text: /^◆ quimera$/ }))?.props.color).toBe('#39C5BB')
  })

  test('splits into two rows on a narrow terminal', async ($, on) => {
    stubWorld(on, { branch: 'feat/status-bar' })
    await startSession($, on)

    expect(await statusRows($, 70)).toEqual([
      '◆ quimera  Opus 5.5  ▌          37k/800k  5h 23%  7d 41%',
      'feat/status-bar  +45 −12  #42 open  ✗ 3/4',
    ])
  })

  test('follows the measurements: limits, context and their colors', async ($, on) => {
    stubWorld(on, { branch: 'main' })
    await startSession($, on)
    await $.session.measure({
      context: { window: 1_000_000, tokens: 700_000 },
      rateLimits: [
        { kind: 'five_hour', percentUsed: 93 },
        { kind: 'seven_day', percentUsed: 80 },
      ],
      changed: ['context', 'rateLimits'],
    })

    const ui = await $.ui.mount({ plugin: 'fancy', surface: 'terminal', component: 'PromptHint', props: HINT })

    expect((await ui.find({ type: 'Text', text: /^ 700k\/800k$/ }))?.props.color).toBe('#F85149')
    expect((await ui.find({ type: 'Text', text: /^93%$/ }))?.props.color).toBe('#F85149')
    expect((await ui.find({ type: 'Text', text: /^80%$/ }))?.props.color).toBe('#D29922')
  })

  test('leaves out the identity when the variable is unset or the option is off', async ($, on) => {
    stubWorld(on, { branch: 'main', env: {} })
    await startSession($, on)

    expect((await statusRows($, 140))[0]).toStartWith('Opus 5.5')
  })

  test(
    'skips gh and git diff when their sections are off',
    { options: { showDiff: false, showPullRequest: false, showChecks: false, showIdentity: false, showModel: false } },
    async ($, on) => {
      const commands: string[] = []
      stubWorld(on, { branch: 'feat/status-bar', commands })
      await startSession($, on)

      expect(commands.some(command => command.startsWith('gh ') || command.startsWith('git diff'))).toBe(false)
      expect(await statusRows($, 140)).toEqual(['▌          37k/800k  5h 23%  7d 41%   feat/status-bar'])
    },
  )
})
