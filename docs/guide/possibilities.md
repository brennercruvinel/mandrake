# Possibilities

Where Mandrake can grow, roughly in order of how close each is. None of this exists yet.

- **An effectiveness bench.** `bench` shows tokens saved. Showing that a core and soul make the
  agent *better* needs a small task suite with pass or fail checks, run with and without them.
- **Rules as data for the plugin.** prompt-trim's lists live in code; reading them from the profile
  would let a team tune the cut without editing JavaScript.
- **The live A/B in one command.** `mandrake ab "<prompt>"` starting both proxies and both
  sessions, with a token ceiling, and opening `watch` on the result.
- **Watch inside Claude Code.** `watch --embed` in a tui-embed pane, next to the conversation.
- **Profiles per team.** A shared base profile plus a personal overlay, merged at setup.
- **Other agents.** The proxy cuts an Anthropic API payload; the same idea fits any CLI that takes
  a base URL. The catalog would grow a section per agent.
- **Memory as a pack.** `invoke` already calls any memory command; a reference memory (append-only,
  exportable) could ship as an optional pack.
- **Security harness.** The proxy is a choke point: it can enforce egress rules, flag secrets on the
  way out, and record what an agent sent, for review.
