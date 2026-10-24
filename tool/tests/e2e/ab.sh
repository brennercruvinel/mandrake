#!/usr/bin/env bash
# Live A/B: the same prompt through Claude Code without Mandrake (pass proxy, no prompt-trim) and
# with it (trim proxy, prompt-trim), each on its own port in a throwaway MANDRAKE_HOME, then the
# watch screen as text. Spends two small real requests of the logged-in Claude Code. The events and
# the audit bodies stay in ~/.local/state/mandrake/ab/<stamp> (they hold the prompt as sent: local only).
#   tool/tests/e2e/ab.sh ["prompt"] [path/to/mandrake]
set -uo pipefail
prompt="${1:-Answer only: ok}"
src="$(cd "$(dirname "$0")/../../.." && pwd)"
bin="${2:-$src/target/debug/mandrake}"
t="${XDG_STATE_HOME:-$HOME/.local/state}/mandrake/ab/$(date -u +%Y%m%d-%H%M%S)"; export MANDRAKE_HOME="$t"
mkdir -p "$t/config"
printf '[proxy]\naudit = true\nstrip_fields = ["metadata"]\n' > "$t/config/mandrake.toml"
"$bin" proxy run --pass --port 8798 > "$t/pass.log" 2>&1 & p1=$!
"$bin" proxy run --port 8799 > "$t/trim.log" 2>&1 & p2=$!
trap 'kill $p1 $p2 2>/dev/null' EXIT
for _ in $(seq 1 30); do nc -z 127.0.0.1 8798 2>/dev/null && nc -z 127.0.0.1 8799 2>/dev/null && break; sleep 0.2; done
kill -0 $p1 2>/dev/null && kill -0 $p2 2>/dev/null || { echo "a proxy did not start:"; cat "$t/pass.log" "$t/trim.log"; exit 1; }
trim_dir="$src/data/plugins/prompt-trim"
# --settings wins over the user's settings.json, which may already point at another proxy.
(cd "$t" && claude -p --settings '{"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:8798","CLAUDE_CODE_PLUGIN_DIRS":""}}' "$prompt" >/dev/null)
(cd "$t" && claude -p --settings "{\"env\":{\"ANTHROPIC_BASE_URL\":\"http://127.0.0.1:8799\",\"CLAUDE_CODE_PLUGIN_DIRS\":\"$trim_dir\"}}" "$prompt" >/dev/null)
for _ in $(seq 1 60); do [ "$(cat "$t/state/events.jsonl" 2>/dev/null | grep -c .)" -ge 2 ] && break; sleep 0.5; done
"$bin" watch --snapshot 150x14
echo "kept in $t (hex: MANDRAKE_HOME=$t mandrake hex <id>)"
jq -r '"\(.id)\t\(.mode)\ttokens \(.tokens_before) -> \(.tokens_after)\tbytes \(.bytes_before) -> \(.bytes_after)"' "$t/state/events.jsonl"
