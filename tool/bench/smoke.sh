#!/usr/bin/env bash
# The task bench end to end without spending: every session goes to a stand-in API on loopback
# (tool/tests/e2e/anina/cli/fake-upstream.py), with a clean Claude Code config and prompt-trim from
# this checkout. The tasks fail by design; what shows is the pipeline and the report, and the
# difference in input tokens between the two sides.
#   tool/bench/smoke.sh [task,task...]
set -uo pipefail
repo="$(cd "$(dirname "$0")/../.." && pwd)"
command -v claude > /dev/null || { echo "Claude Code is not on PATH"; exit 2; }
t="$(mktemp -d)" || exit 2
port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
python3 "$repo/tool/tests/e2e/anina/cli/fake-upstream.py" "$port" &
api=$!
trap 'kill "$api" 2> /dev/null; wait "$api" 2> /dev/null; rm -rf "$t"' EXIT
cp -R "$repo/data/plugins/prompt-trim" "$t/prompt-trim"
python3 -c 'import json,sys; print("export const CORE = " + json.dumps(open(sys.argv[1]).read()))' \
  "$repo/data/prompt/cores/coding.md" > "$t/prompt-trim/hooks/core.js"
for _ in $(seq 30); do (exec 3<> "/dev/tcp/127.0.0.1/$port") 2> /dev/null && break; sleep 0.1; done
XDG_STATE_HOME="$t/state" CLAUDE_CONFIG_DIR="$t/claude" \
  python3 "$repo/tool/bench/tasks.py" run --api "http://127.0.0.1:$port" --trim "$t/prompt-trim" \
  --only "${1:-answer-from-docs,py-off-by-one,careful-cleanup}" | sed "s#$t#<tmp>#g"
