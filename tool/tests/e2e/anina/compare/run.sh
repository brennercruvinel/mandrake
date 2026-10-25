#!/usr/bin/env bash
# `mandrake compare` in a throwaway setup: its own MANDRAKE_HOME pointing at this checkout, a clean
# Claude Code config (no user CLAUDE.md, no plugins) and a placeholder API key. Nothing reaches the
# API: compare answers locally, so no login is needed and no private instructions show up.
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
mkdir -p "$t/mh/data" "$t/claude"
ln -s "$repo" "$t/mh/data/src"
MANDRAKE_HOME="$t/mh" CLAUDE_CONFIG_DIR="$t/claude" ANTHROPIC_API_KEY=sk-ant-compare-local-placeholder \
  "$bin" compare "$@"
