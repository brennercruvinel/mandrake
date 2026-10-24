
import http from 'node:http'

const UPSTREAM = process.env.PROMPT_PROXY_UPSTREAM ?? 'https://api.anthropic.com'
const PORT = Number(process.env.PROMPT_PROXY_PORT ?? 8787)
const PREFIXES = new Set([
  "You are Claude Code, Anthropic's official CLI for Claude.",
  "You are Claude Code, Anthropic's official CLI for Claude, running within the Claude Agent SDK.",
  "You are a Claude agent, built on Anthropic's Claude Agent SDK.",
])
const HOP = new Set(['host', 'content-length', 'accept-encoding', 'connection'])
const BACK = new Set(['content-encoding', 'content-length', 'transfer-encoding', 'connection'])

const untag = text => text.replace(/^<system-reminder>\n?/, '').replace(/\n?<\/system-reminder>\s*$/, '')

function trim(raw) {
  const body = JSON.parse(raw)
  let cut = 0
  if (Array.isArray(body.system)) {
    const n = body.system.length
    body.system = body.system.filter(b => !(b?.type === 'text' && PREFIXES.has(String(b.text).trim())))
    cut += n - body.system.length
  }
  for (const m of body.messages ?? []) {
    if (typeof m.content === 'string') { m.content = untag(m.content); continue }
    if (!Array.isArray(m.content)) continue
    for (const p of m.content) if (p?.type === 'text' && p.text.startsWith('<system-reminder>')) { p.text = untag(p.text); cut++ }
  }
  for (const k of ['metadata', 'safeguards']) if (k in body) { delete body[k]; cut++ }
  return { raw: JSON.stringify(body), cut }
}

http.createServer(async (req, res) => {
  const path = req.url.split('?')[0]
  try {
    const chunks = []
    for await (const c of req) chunks.push(c)
    let raw = Buffer.concat(chunks).toString('utf8')
    let cut = 0
    if (req.method === 'POST' && path.startsWith('/v1/messages') && raw) ({ raw, cut } = trim(raw))
    const headers = Object.fromEntries(Object.entries(req.headers).filter(([k]) => !HOP.has(k)))
    const up = await fetch(UPSTREAM + req.url, {
      method: req.method,
      headers,
      body: ['GET', 'HEAD'].includes(req.method) ? undefined : raw,
    })
    console.log(`${new Date().toISOString()} ${req.method} ${path} cut=${cut} status=${up.status}`)
    const out = {}
    up.headers.forEach((v, k) => { if (!BACK.has(k)) out[k] = v })
    res.writeHead(up.status, out)
    if (up.body) for await (const c of up.body) res.write(c)
    res.end()
  } catch (err) {
    console.log(`${new Date().toISOString()} ${req.method} ${path} error=${err?.message ?? err}`)
    if (!res.headersSent) res.writeHead(502, { 'content-type': 'application/json' })
    res.end(JSON.stringify({ type: 'error', error: { type: 'proxy_error', message: 'prompt-proxy could not reach the API' } }))
  }
}).listen(PORT, '127.0.0.1', () => console.log(`prompt-proxy on 127.0.0.1:${PORT} -> ${UPSTREAM}`))
