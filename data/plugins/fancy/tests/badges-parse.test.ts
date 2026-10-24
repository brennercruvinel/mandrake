import { describe, expect, test } from 'claude-code/testing'

import { DEFAULT_TAGS, parseTags, readConfig } from '../hooks/badges/config'
import { markerPattern, split } from '../hooks/badges/parse'

const marker = markerPattern(parseTags(DEFAULT_TAGS).keys())

describe('split', () => {
  test('turns a line that starts with a tag into a tag segment', () => {
    expect(split('`[DONE]` #531\n- verified', marker)).toEqual([
      { kind: 'tag', tag: 'DONE', rest: '#531', gap: false },
      { kind: 'md', text: '- verified', gap: false },
    ])
  })

  test('accepts a tag without backticks and without a title', () => {
    expect(split('[WAIT]', marker)).toEqual([{ kind: 'tag', tag: 'WAIT', rest: '', gap: false }])
  })

  test('leaves tags inside code fences and mid-sentence alone', () => {
    const text = '```md\n`[DONE]` example\n```\n\nfinished [DONE] here'
    expect(split(text, marker)).toEqual([{ kind: 'md', text, gap: false }])
  })

  test('ignores tags that are not configured', () => {
    expect(split('[TODO] later', marker)).toEqual([{ kind: 'md', text: '[TODO] later', gap: false }])
  })

  test('keeps the blank lines between blocks as gaps', () => {
    const segments = split('intro\n\n`[WARN]` risk\n\nbody', marker)
    expect(segments.map(s => s.gap)).toEqual([false, true, true])
  })
})

describe('config', () => {
  test('reads TAG:color pairs and skips malformed ones', () => {
    expect([...parseTags('OK:green, BAD, :red, X-1:#ff8800')]).toEqual([
      ['OK', 'green'],
      ['X-1', '#ff8800'],
    ])
  })

  test('falls back to the defaults when no pair is valid', () => {
    const config = readConfig({ tags: 'nonsense', style: 'other' })
    expect(config.tags.get('DONE')).toBe('#13651A')
    expect(config.style).toBe('badge')
  })
})
