import { atom, memberOf, read, update } from 'claude-code'
import type { EngineInterface, HookStream, ProcessSpawnChunk, ProcessSpawnResult, Register } from 'claude-code'

import type { Frame } from '../types'

const apps = {
  abtop: {
    title: 'abtop',
    description: 'abtop num painel: todas as sessões de agente desta máquina',
    argumentHint: undefined,
    option: 'abtop_path',
    argv: (): string[] => [],
  },
  gitlogue: {
    title: 'gitlogue',
    description: 'gitlogue num painel: os commits deste repositório sendo reescritos',
    argumentHint: '[caminho do repositório] [opções do gitlogue]',
    option: 'gitlogue_path',
    argv: (tokens: string[]): string[] => {
      const [first, ...rest] = tokens
      return first && /^[/.]/.test(first) ? ['--path', first, ...rest] : tokens
    },
  },
} as const

type AppId = keyof typeof apps

const APP_IDS = Object.keys(apps) as AppId[]
const frame = atom({ plugin: 'tui-embed', key: 'frame' } as const, null as Frame | null)
const error = atom({ plugin: 'tui-embed', key: 'error' } as const, null as string | null)
const self = atom({ plugin: 'tui-embed', key: 'self' } as const, null as string | null)

const bins: Record<AppId, string> = { abtop: 'abtop', gitlogue: 'gitlogue' }
const sockets: Partial<Record<AppId, string>> = {}
const children: Partial<Record<AppId, HookStream<ProcessSpawnChunk, ProcessSpawnResult>>> = {}
const extra: Record<AppId, string[]> = { abtop: [], gitlogue: [] }
let sessionKey = ''
let launches = 0

/** Frames reach the pane at most this often; an app may draw far faster (gitlogue ~20/s). */
const FRAME_MS = 66
const latest: Partial<Record<AppId, Frame>> = {}
const queued: Partial<Record<AppId, boolean>> = {}

const isApp = (id: string): id is AppId => id in apps

/** Splits a command's arguments on spaces, keeping quoted runs whole. */
function tokens(args: string): string[] {
  return [...args.matchAll(/"([^"]*)"|'([^']*)'|(\S+)/g)].map(m => m[1] ?? m[2] ?? m[3] ?? '')
}

function lastFrame(lines: string[]): Frame | null {
  for (let i = lines.length - 1; i >= 0; i--) {
    try {
      return JSON.parse(lines[i] ?? '') as Frame
    } catch {
      continue
    }
  }
  return null
}

async function openIds($: EngineInterface) {
  return (await $.ui.panes()).map(p => p.id).filter(isApp)
}

function queue($: EngineInterface, id: AppId, next: Frame) {
  latest[id] = next
  if (queued[id]) return
  queued[id] = true
  $.clock.after(FRAME_MS, () => void publish($, id))
}

async function publish($: EngineInterface, id: AppId) {
  queued[id] = false
  const next = latest[id]
  delete latest[id]
  if (next) await update($, memberOf(frame, { requestId: id }), () => next)
}

async function ensureKey($: EngineInterface) {
  if (!sessionKey) sessionKey = (await $.session.id()).slice(0, 8)
}

/** Spawns the app; nothing is awaited before it is recorded, so two opens never race. */
async function start($: EngineInterface, id: AppId) {
  if (children[id]) return
  const app = apps[id]
  launches += 1
  const socket = `/tmp/tui-embed-${sessionKey}-${id}-${launches}.sock`
  sockets[id] = socket
  const stream = $.process.spawn({ argv: [bins[id], '--embed', socket, ...extra[id]] })
  children[id] = stream
  await update($, memberOf(error, { requestId: id }), () => null)
  let pending = ''
  let stderr = ''
  try {
    for await (const chunk of stream) {
      if (chunk.stream === 'stderr') {
        stderr = (stderr + chunk.text).slice(-2000)
        continue
      }
      pending += chunk.text
      const end = pending.lastIndexOf('\n')
      if (end < 0) continue
      const next = lastFrame(pending.slice(0, end).split('\n'))
      pending = pending.slice(end + 1)
      if (next) queue($, id, next)
    }
    if (children[id] === stream) {
      await update($, memberOf(error, { requestId: id }), () => `${app.title} saiu. ${stderr.trim()} /${id} abre de novo.`)
    }
  } catch (err) {
    const why = err instanceof Error ? err.message : String(err)
    if (children[id] === stream) {
      await update($, memberOf(error, { requestId: id }), () => `${app.title} parou (${bins[id]}): ${why}. /${id} abre de novo; ${app.option} no /config muda o binário.`)
    }
  } finally {
    if (children[id] === stream) delete children[id]
  }
}

function stop(id: AppId) {
  const running = children[id]
  delete children[id]
  void running?.return(undefined as never).catch(() => undefined)
}

async function forward($: EngineInterface, id: AppId, data: unknown) {
  const socket = sockets[id]
  if (!children[id] || !socket) return
  await $.http.fetch('http://embed/input', {
    method: 'POST',
    socketPath: socket,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(data),
  })
}

async function send($: EngineInterface, sessionId: string, project: string, text: string) {
  const body = text.trim()
  if (!body) return
  const sent = await $.session.send({ to: { sessionId }, text: body })
  $.ui.toast(sent.isDelivered ? `mensagem entregue a ${project}` : `não entregue: ${sent.reason}`)
}

async function open($: EngineInterface, id: AppId, args: string) {
  const argv = apps[id].argv(tokens(args))
  const changed = argv.join('\0') !== extra[id].join('\0')
  extra[id] = argv
  if (changed) {
    stop(id)
    await update($, memberOf(frame, { requestId: id }), () => null)
  }
  await $.ui.open({ id, title: apps[id].title, focus: true })
  void start($, id)
}

export const register: Register = (on, options) => {
  bins.abtop = String(options.abtop_path ?? 'abtop')
  bins.gitlogue = String(options.gitlogue_path ?? 'gitlogue')

  on('session.start', async ($, e, next) => {
    const started = await next(e)
    for (const id of APP_IDS) {
      const { description, argumentHint } = apps[id]
      await $.command.register({ name: id, description, ...(argumentHint ? { argumentHint } : {}) })
    }
    await ensureKey($)
    const session = await $.session.id()
    await update($, self, () => session)
    for (const id of await openIds($)) void start($, id)

    return started
  })

  on('command.run', async ($, e, next) => {
    if (!isApp(e.command)) return next(e)
    await ensureKey($)
    await open($, e.command, e.args)

    return { text: `Painel ${apps[e.command].title} aberto. Clique nele para dar o teclado ao app; Esc devolve.` }
  })

  on('ui.close', async ($, e, next) => {
    if (isApp(e.id)) stop(e.id)

    return next(e)
  })

  on('ui.message', async ($, e, next) => {
    if (!isApp(e.requestId)) return next(e)
    await forward($, e.requestId, e.data)

    return {}
  })

  on('ui.render', { component: 'Pane' }, async ($, e, next) => {
    const id = e.requestId
    if (!isApp(id)) return next(e)
    if (e.surface !== 'terminal' && e.surface !== 'desktop') {
      const { Text } = $.ui.resolve(e)
      return <Text>{`${apps[id].title}: superfície sem suporte`}</Text>
    }
    const { Box, Text, Button, Client, Input } = $.ui.resolve(e)
    const shown = await read($, memberOf(frame, e))
    const problem = await read($, memberOf(error, e))
    const target = shown?.selected
    const canSend = target?.agent_cli === 'claude' && target.session_id !== (await read($, self))
    const room = Math.max(5, e.props.scroll.bodyRows - 1 - (canSend ? 1 : 0) - (problem ? 2 : 0))

    return (
      <Box flexDirection="column">
        {problem && <Text color="error" wrap="wrap">{problem}</Text>}
        <Client key={`screen:${id}`} module="./screen.tsx" props={shown} width={e.props.bodyColumns} height={room} />
        <Box flexDirection="row" gap={2}>
          <Button key="escape" plain dimColor onPress={() => void forward($, id, { type: 'key', key: 'escape' })}>
            Esc no {apps[id].title}
          </Button>
          <Text dimColor>clique no painel para teclar; Esc do teclado volta ao prompt</Text>
        </Box>
        {canSend && target && (
          <Input
            key={`send:${target.session_id}`}
            label={`→ ${target.project_name} `}
            placeholder="mensagem para a sessão selecionada no abtop"
            submitLabel="enviar"
            onSubmit={text => void send($, target.session_id, target.project_name, text)}
          />
        )}
      </Box>
    )
  })
}
