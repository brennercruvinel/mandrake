import type { ClientModule } from 'claude-code'

import type { Frame } from '../types'

type Size = { cols: number; rows: number }

const BOLD = 1
const DIM = 2
const ITALIC = 4
const UNDERLINE = 8
const REVERSED = 16

const screen: ClientModule<Frame | null, Size> = (frame, surface) => {
  const { Box, Text } = surface.elements

  surface.onKey(k => surface.post({ type: 'key', key: k.key, shift: k.shift === true, ctrl: k.ctrl === true }))
  surface.onPointer(p => {
    if (p.type === 'down' && (p.button ?? 'left') === 'left') surface.post({ type: 'click', x: p.x, y: p.y })
  })

  const { columns: cols, rows } = surface
  if (cols > 0 && rows > 0 && (surface.state?.cols !== cols || surface.state?.rows !== rows)) {
    surface.post({ type: 'resize', cols, rows })
    surface.setState({ cols, rows })
  }

  if (!frame) return <Text dimColor>abtop: iniciando…</Text>

  return (
    <Box flexDirection="column">
      {frame.lines.map(runs => (
        <Text wrap="truncate-end">
          {runs.map(([text, fg, bg, mods]) => (
            <Text
              color={fg || undefined}
              backgroundColor={bg || undefined}
              bold={(mods & BOLD) !== 0}
              dimColor={(mods & DIM) !== 0}
              italic={(mods & ITALIC) !== 0}
              underline={(mods & UNDERLINE) !== 0}
              inverse={(mods & REVERSED) !== 0}
            >
              {text}
            </Text>
          ))}
        </Text>
      ))}
    </Box>
  )
}

export default screen
