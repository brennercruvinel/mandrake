# Measure and audit

Every view reads the same data: `~/.local/state/mandrake/events.jsonl`, one line per request, and,
with `[proxy] audit = true`, the bodies in `audit/<id>.before.json` and `.after.json`.

## Live: `mandrake watch`

Split screen. The left side shows requests through a proxy in `pass` mode (no Mandrake), the
right side requests through the trimming proxy. Each side has totals (requests, tokens before and
after, bytes, cache read and create) and the latest requests. The footer shows the savings so far.
`Enter` on an audited request opens its hex.

To fill both sides, run a second proxy without the cut and point one session at it:

```sh
mandrake proxy run --pass --port 8788 &
ANTHROPIC_BASE_URL=http://127.0.0.1:8788 claude -p "same prompt"   # left side
claude -p "same prompt"                                             # right side, through 8787
```

The plugin's cut happens before the request exists, so for the left side to show Claude Code
untouched, start that session without the plugin as well (`CLAUDE_CODE_PLUGIN_DIRS=`).

`mandrake watch --embed` draws without the alternate screen, for a tui-embed pane inside Claude Code.

## Hex: `mandrake hex <id>`

The request before and after the cut, side by side in hex and ASCII. Removed bytes are red,
inserted bytes green; `n` and `p` jump between changed regions, and the slider shows where you are
in the body. `mandrake audit` lists the audited ids.

## Bench: `mandrake bench`

Runs the request bodies in `tests/fixtures/*.json` through the cut and asks `count_tokens` for each
side, with `ANTHROPIC_API_KEY` (or the header in `MANDRAKE_BENCH_AUTH`). It writes
`~/.local/state/mandrake/bench/latest.json` and `.md`; `mandrake bench --view` draws it. Without a
credential it measures bytes only and says so.

## Audit

Audit bodies never contain headers. Text that looks like a secret (`sk-ant-…`, `ghp_…`,
`github_pat_…`, `AKIA…`, private key blocks, Slack tokens) becomes `[redacted]`. Files are mode
600, and those older than `audit_keep_days` are removed when the proxy starts. They still hold your
conversation and code: keep audit on while you are measuring, not by habit.
