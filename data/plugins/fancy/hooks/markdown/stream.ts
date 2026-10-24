// Look's tags taken out of streamed text and put back on the finished block.

import { isTag } from './inlines'

/** Markdown with the markup tags taken out, for the engine's own renderer. */
export function stripTags(text: string): string {
  return text.replace(/<\/\s*>/g, '').replace(/<\/?([a-zA-Z#][\w#-]*)(?:\s+[^<>]*?)?\s*\/?>/g, (whole, name: string) =>
    isTag(name) ? '' : whole,
  )
}

/**
 * Streamed lines with the markup tags taken out, fences and code spans left
 * alone; `inFence` carries a fence open across flushes. Returns the lines,
 * whether a fence is still open after them, and each changed line by how it
 * now shows (trimmed) to how it was written.
 */
export function stripStreamed(delta: string, inFence: boolean): [string, boolean, Record<string, string>] {
  let isOpen = inFence
  const changed: Record<string, string> = {}
  const lines = delta.split('\n').map(line => {
    if (/^ {0,3}(`{3,}|~{3,})/.test(line)) {
      isOpen = !isOpen
      return line
    }
    if (isOpen) return line
    const shown = line
      .split(/(`+[^`]*`+)/)
      .map((part, k) => (k % 2 === 1 ? part : stripTags(part)))
      .join('')
    // A line of tags alone would show blank and could not be found again.
    if (shown === line || shown.trim() === '') return line
    changed[shown.trim()] = line
    return shown
  })
  return [lines.join('\n'), isOpen, changed]
}

/** The block's text with each line `stripStreamed` changed put back as written. */
export function restoreLines(text: string, written: Readonly<Record<string, string>>): string {
  return text
    .split('\n')
    .map(line => {
      const key = line.trim()
      return key !== '' && Object.hasOwn(written, key) ? written[key]! : line
    })
    .join('\n')
}
