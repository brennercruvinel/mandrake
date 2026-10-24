export type Segment =
  | { kind: 'md'; text: string; gap: boolean }
  | { kind: 'tag'; tag: string; rest: string; gap: boolean }

const FENCE = /^\s*(`{3,}|~{3,})/

// A tag counts only at the start of a line, optionally in backticks: `[DONE]` title
export function markerPattern(names: Iterable<string>): RegExp {
  return new RegExp(`^\\s*\`?\\[(${[...names].join('|')})\\]\`?(?:\\s+(.*))?$`)
}

export function split(text: string, marker: RegExp): Segment[] {
  const out: Segment[] = []
  let buf: string[] = []
  let fence = ''
  let prevBlank = false

  const flush = () => {
    const body = buf.join('\n')
    if (body.trim()) out.push({ kind: 'md', text: body.replace(/^\n+|\n+$/g, ''), gap: !buf[0]?.trim() })
    buf = []
  }

  for (const line of text.split('\n')) {
    const delim = FENCE.exec(line)?.[1]
    if (delim) fence = !fence ? delim : delim.startsWith(fence) ? '' : fence

    const m = fence ? null : marker.exec(line)
    const tag = m?.[1]
    if (!m || !tag) {
      buf.push(line)
      prevBlank = !line.trim()
      continue
    }

    flush()
    out.push({ kind: 'tag', tag, rest: m[2] ?? '', gap: prevBlank })
    prevBlank = false
  }

  flush()
  return out
}
