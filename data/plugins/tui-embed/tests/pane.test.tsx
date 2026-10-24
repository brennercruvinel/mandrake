import { expect, mock, test } from 'claude-code/testing'

const frameOf = (title: string, selected: unknown) => ({
  cols: 40,
  rows: 2,
  selected,
  lines: [
    [[` ${title} header`, '#ffffff', '', 1]],
    [['► first row', '#00ff00', '#000000', 0]],
  ],
})

const APPS = [
  { id: 'abtop', selected: { session_id: 's-beta', agent_cli: 'claude', project_name: 'beta', pid: 200 } },
  { id: 'gitlogue', selected: undefined },
] as const

const PANE = {
  isFocused: true,
  bodyColumns: 40,
  placement: 'dock',
  scroll: { offset: 0, bodyRows: 20 },
  view: {},
} as const

for (const surface of ['terminal', 'desktop'] as const) {
  for (const app of APPS) {
    test(`${surface} ${app.id}: draws the app's frame and forwards keys over its socket`, async ($, on) => {
      const posted: { socket: string | undefined; body: unknown }[] = []
      const spawned: string[][] = []
      const sent: unknown[] = []
      const clock = mock.clock(on)
      let exit = () => {}
      const exited = new Promise<void>(resolve => (exit = resolve))
      on('ui.open', () => ({ value: { isPlaced: true } }))
      on('session.id', () => ({ value: 'own-session-id' }))
      on('ui.toast', () => ({ value: undefined }))
      on('process.spawn', async function* ($, e) {
        spawned.push([...e.argv])
        yield { stream: 'stdout', text: 'not json\n' }
        yield { stream: 'stdout', text: `${JSON.stringify(frameOf('stale', app.selected))}\n` }
        yield { stream: 'stdout', text: `${JSON.stringify(frameOf(app.id, app.selected))}\n` }
        await exited
        return { value: { code: 0, signal: null } }
      })
      on('http.fetch', ($, e) => {
        posted.push({ socket: e.init?.socketPath, body: JSON.parse(String(e.init?.body)) })
        return { value: { status: 204, ok: true, headers: {}, text: '' } }
      })
      on('session.send', ($, e) => {
        sent.push(e)
        return { isDelivered: true }
      })

      await $.command.run({
        command: app.id,
        args: '',
        origin: { kind: 'composer' },
        presentation: { isFullscreen: true, columns: 160 },
      })
      const ui = await $.ui.mount({
        plugin: 'tui-embed',
        surface,
        component: 'Pane',
        requestId: app.id,
        props: { ...PANE, title: app.id },
      })
      const screen = `screen:${app.id}`
      const socket = spawned[0]?.[2]

      expect(spawned[0]?.slice(0, 2)).toEqual([app.id, '--embed'])
      expect(socket).toMatch(new RegExp(`^/tmp/tui-embed-own-sess-${app.id}-\\d+\\.sock$`))
      await clock.advance(100)
      expect(await ui.find({ in: screen, type: 'Text', text: new RegExp(`${app.id} header`) })).toBeDefined()
      expect(await ui.find({ in: screen, type: 'Text', text: /stale header/ })).toBeUndefined()

      await ui.key({ key: 'j', in: screen })
      expect(posted).toContainEqual({ socket, body: { type: 'key', key: 'j', shift: false, ctrl: false } })

      await ui.press({ key: 'escape' })
      expect(posted).toContainEqual({ socket, body: { type: 'key', key: 'escape' } })

      if (app.selected) {
        await ui.input({ key: `send:${app.selected.session_id}`, text: 'status?' })
        expect(sent).toHaveLength(1)
      } else {
        expect(await ui.find({ type: 'Input' })).toBeUndefined()
      }
      exit()
      await ui.unmount()
    })
  }
}

test('gitlogue: a path argument becomes --path, other options pass through, a change restarts it', async ($, on) => {
  const spawned: string[][] = []
  mock.clock(on)
  on('ui.open', () => ({ value: { isPlaced: true } }))
  on('session.id', () => ({ value: 'own-session-id' }))
  on('http.fetch', () => ({ value: { status: 204, ok: true, headers: {}, text: '' } }))
  on('process.spawn', async function* ($, e) {
    spawned.push([...e.argv])
    yield { stream: 'stdout', text: 'starting\n' }
    await new Promise<void>(() => undefined)
    return { value: { code: 0, signal: null } }
  })
  const run = (args: string) =>
    $.command.run({
      command: 'gitlogue',
      args,
      origin: { kind: 'composer' },
      presentation: { isFullscreen: true, columns: 160 },
    })
  await run('/work/my repo')
  await run('"/work/my repo" --theme nord')
  await run('--commit HEAD~5')
  const ui = await $.ui.mount({
    plugin: 'tui-embed',
    surface: 'terminal',
    component: 'Pane',
    requestId: 'gitlogue',
    props: { ...PANE, title: 'gitlogue' },
  })
  await ui.press({ key: 'escape' })

  expect(spawned.map(argv => argv.slice(3))).toEqual([
    ['--path', '/work/my', 'repo'],
    ['--path', '/work/my repo', '--theme', 'nord'],
    ['--commit', 'HEAD~5'],
  ])
  expect(new Set(spawned.map(argv => argv[2])).size).toBe(3)
  await ui.unmount()
})
