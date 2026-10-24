import { describe, expect, test } from 'claude-code/testing'

import { contrast, gradient, luminance, stopsOf, textOn, textOnAll } from '../hooks/badges/color'

describe('textOn', () => {
  test('auto picks the text with the higher WCAG contrast', () => {
    expect(textOn('#FFD43B', 'auto')).toBe('#000000')
    expect(textOn('#364FC7', 'auto')).toBe('#FFFFFF')
  })

  test('auto keeps the theme inverse text for theme keys and names', () => {
    expect(textOn('success', 'auto')).toBe('inverseText')
    expect(textOn('red', 'auto')).toBe('inverseText')
  })

  test('white and black are fixed', () => {
    expect(textOn('#FFD43B', 'white')).toBe('#FFFFFF')
    expect(textOn('#364FC7', 'black')).toBe('#000000')
  })
})

describe('contrast', () => {
  test('matches the WCAG extremes', () => {
    expect(contrast(1, 0)).toBe(21)
    expect(luminance('#FFFFFF')).toBe(1)
    expect(luminance('success')).toBeUndefined()
  })
})

describe('gradient', () => {
  test('interpolates one color per cell between the stops', () => {
    expect(gradient(stopsOf('#000000>#FFFFFF'), 3)).toEqual(['#000000', '#808080', '#FFFFFF'])
  })

  test('a single color or a non-hex stop paints solid', () => {
    expect(gradient(stopsOf('#13651A'), 3)).toEqual(['#13651A', '#13651A', '#13651A'])
    expect(gradient(stopsOf('success>#FFFFFF'), 2)).toEqual(['success', 'success'])
  })

  test('one text color for the whole gradient, chosen by its worst cell', () => {
    expect(textOnAll(gradient(stopsOf('#4130DF>#9B1FE8>#C2185B'), 6), 'auto')).toBe('#FFFFFF')
  })
})
