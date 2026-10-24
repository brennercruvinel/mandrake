import type { PluginOptions } from 'claude-code'

import type { LookPrefs } from '../../types'
import type { Ctx } from '../markdown/inline'
import { THEMES, THEME_NAMES, paletteFor } from '../markdown/themes'

/** How many restored lines the session keeps; the oldest go first. */
export const MAX_LINES = 3000

/** Replies longer than this skip the markdown engine: a tree has a size bound. */
export const MAX_TEXT = 30000

/** The prefs `/config` sets as defaults; `/fancy` stores the ones it changes on top of them. */
export function lookState(options: PluginOptions) {
  const theme = typeof options.theme === 'string' && THEMES[options.theme] !== undefined ? options.theme : 'vivid'
  const defaults: LookPrefs = { isEnabled: options.markdown !== false, isTeaching: true, theme, background: 'auto' }
  /** `teachMarkup` off keeps the guide out of the prompt whatever `/fancy teach` says. */
  return { defaults, canTeach: options.teachMarkup !== false }
}

export type Look = ReturnType<typeof lookState>

/** Whether a /config theme value names a light theme. */
export const isLightName = (theme: unknown) => /light/i.test(String(theme ?? ''))

/** The drawing context for the prefs in force, or undefined while the engine is off. */
export function contextOf(p: LookPrefs, isLightTheme: boolean, t: Ctx['t'], columns: number | undefined): Ctx | undefined {
  if (!p.isEnabled) return undefined
  const isLight = p.background === 'auto' ? isLightTheme : p.background === 'light'
  return { t, p: paletteFor(p.theme, isLight), isLight, width: Math.max(20, columns ?? 80), listDepth: 0, isQuoted: false }
}

export function usage(c: Look, p: LookPrefs, isLight: boolean): string {
  const themes = THEME_NAMES.map(name => {
    const label = name === p.theme ? `**${name}**` : name
    return THEMES[name]!.title.length > 1 ? `<grad theme="${name}">${label}</grad>` : label
  }).join(' · ')
  const background = p.background === 'auto' ? `auto (${isLight ? 'light' : 'dark'})` : p.background
  const teaching = !c.canTeach
    ? '<muted>off (teachMarkup is off in /config)</muted>'
    : p.isTeaching
      ? '<ok>on</ok>'
      : '<muted>off</muted>'
  return [
    `fancy is ${p.isEnabled ? '<ok>on</ok>' : '<err>off</err>'} · theme **${p.theme}** · background ${background} · teaching the model ${teaching}`,
    '',
    `Themes: ${themes}`,
    '',
    '`/fancy on|off` · `/fancy <theme>` · `/fancy dark|light|auto` · `/fancy teach on|off` · `/fancy demo`',
  ].join('\n')
}

/** What `/fancy <args>` changes, the demo it shows, or the option it does not know. */
export function commandOf(args: string): { change: Partial<LookPrefs> } | { demo: true } | { unknown: string } | undefined {
  const [first, second] = args.trim().toLowerCase().split(/\s+/).filter(Boolean)
  if (first === undefined) return undefined
  if (first === 'demo') return { demo: true }
  if (first === 'on' || first === 'off') return { change: { isEnabled: first === 'on' } }
  if (first === 'teach' && (second === 'on' || second === 'off')) return { change: { isTeaching: second === 'on' } }
  if (first === 'dark' || first === 'light' || first === 'auto') return { change: { background: first } }
  if (first === 'theme' && second !== undefined && THEMES[second] !== undefined) return { change: { theme: second, isEnabled: true } }
  if (THEMES[first] !== undefined) return { change: { theme: first, isEnabled: true } }
  return { unknown: args.trim() }
}
