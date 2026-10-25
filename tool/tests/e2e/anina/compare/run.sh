#!/usr/bin/env bash
# `mandrake compare` in a throwaway setup: its own HOME and MANDRAKE_HOME with prompt-trim from this
# checkout, a clean Claude Code config (no user CLAUDE.md, no plugins) and a placeholder API key.
# Nothing reaches the API: compare answers locally, so no login is needed and no private
# instructions show up.
#   tool/tests/e2e/anina/compare/run.sh [path/to/mandrake] [compare args...]
set -uo pipefail
for v in $(env | sed -n 's/^\(MANDRAKE_[A-Z_]*\)=.*/\1/p'); do unset "$v"; done
unset ANTHROPIC_BASE_URL
repo="$(cd "$(dirname "$0")/../../../../.." && pwd)"
bin="${1:-$repo/target/debug/mandrake}"
shift || true
[ -x "$bin" ] || { echo "no mandrake binary at $bin"; exit 2; }
t="$(mktemp -d)" || exit 2
trap 'rm -rf "$t"' EXIT
# The installed copy of prompt-trim, with the core written into core.js the way setup does: the
# checkout's core.js is empty on purpose.
# State goes under a throwaway HOME, so the paths compare prints start with ~ and say nothing
# about the machine that ran it.
mh="$t/home/.mandrake"
pt="$mh/data/src/data/plugins/prompt-trim"
mkdir -p "$pt" "$t/home/.claude"
cp -R "$repo/data/plugins/prompt-trim/." "$pt/"
python3 -c 'import json,sys; print("export const CORE = " + json.dumps(open(sys.argv[1]).read()))' \
  "$repo/data/prompt/cores/coding.md" > "$pt/hooks/core.js" || exit 2
HOME="$t/home" MANDRAKE_HOME="$mh" CLAUDE_CONFIG_DIR="$t/home/.claude" ANTHROPIC_API_KEY=sk-ant-compare-local-placeholder \
  "$bin" compare "$@"
