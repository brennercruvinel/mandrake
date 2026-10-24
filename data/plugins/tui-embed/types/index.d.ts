/** One style run of a row: text, fg and bg as `#rrggbb` or `""`, modifier bits. */
export type Run = [text: string, fg: string, bg: string, mods: number]

/** The session an app has selected, when it is an agent monitor (abtop). */
export type Selected = {
  session_id: string
  agent_cli: string
  project_name: string
  pid: number
}

/** One frame of `<app> --embed <socket>`, as the app writes it to stdout. */
export type Frame = {
  cols: number
  rows: number
  selected?: Selected | null
  lines: Run[][]
}

declare module 'claude-code' {
  interface PluginState {
    'tui-embed': {
      /** The latest frame of each app, by pane id (the app's id). */
      frame: StateFamily<Frame | null>
      /** Why an app is not running, by pane id. */
      error: StateFamily<string | null>
      self: string | null
    }
  }
}
