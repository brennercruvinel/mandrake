# shellcheck shell=bash
# The shell every CLI tape types into: a throwaway HOME, a stub `claude`, `mandrake` on PATH.
# Used as `bash --rcfile env.sh -i` from a tape, or sourced by hand to try a command the same way:
#   MANDRAKE_BIN=target/debug/mandrake MANDRAKE_REPO=. CLI_SEED=installed . tool/tests/e2e/anina/cli/env.sh
# CLI_SEED prepares the machine before the prompt shows, quietly:
#   fresh       nothing installed (default)
#   profile     a small ~/.claude and ~/mandrake-profile.toml, nothing installed
#   installed   `mandrake setup --profile --yes` with a small ~/.claude and two links
#   api         installed, plus a fake API on a local port; the proxy is not started
#   traffic     installed, plus a fake API, the proxy with audit on and three requests through it
#   bench       installed, plus `mandrake bench --fixtures` against the fake API (no real request)
# Nothing here reaches the real HOME, the real Claude Code or the network.

: "${MANDRAKE_BIN:?path to the mandrake binary}"
: "${MANDRAKE_REPO:?path to the mandrake checkout}"
cli_dir="$MANDRAKE_REPO/tool/tests/e2e/anina/cli"
T="$(mktemp -d "${TMPDIR:-/tmp}/mandrake-cli.XXXXXX")"
mkdir -p "$T/home/.claude" "$T/bin" "$T/work"
ln -s "$MANDRAKE_BIN" "$T/bin/mandrake"
# The stub stands in for Claude Code: it says what it was asked to run and exits.
printf '#!/bin/sh\necho "[claude stub] claude $*"\n' > "$T/bin/claude"
chmod +x "$T/bin/claude"
# A shell with Mandrake installed exports MANDRAKE_PORT, MANDRAKE_SOULS, MANDRAKE_PREFIX... and the
# installed env.sh keeps what is already set, so the sandbox would reach the real proxy, souls and
# prefix (#33). Only the two inputs above pass through.
for v in $(env | sed -n 's/^\(MANDRAKE_[A-Z_]*\)=.*/\1/p'); do
  case "$v" in MANDRAKE_BIN | MANDRAKE_REPO) ;; *) unset "$v" ;; esac
done
export HOME="$T/home" MANDRAKE_HOME="$T/mh" CLAUDE_CONFIG_DIR="$T/home/.claude" SHELL=/bin/bash
export PATH="$T/bin:$PATH" MANDRAKE_SRC="$MANDRAKE_REPO" TERM="${TERM:-xterm-256color}"
unset ANTHROPIC_API_KEY ANTHROPIC_BASE_URL MANDRAKE_BENCH_AUTH CLAUDE_CODE_OAUTH_TOKEN
free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'; }
PORT="$(free_port)" API_PORT="$(free_port)"
export PORT API_PORT
pids=()
cli_cleanup() {
  [ "${#pids[@]}" -gt 0 ] && kill "${pids[@]}" 2> /dev/null
  [ -n "${KEEP:-}" ] || rm -rf "$T"
}
trap cli_cleanup EXIT

seed_claude() {
  echo "# my rules" > "$HOME/.claude/CLAUDE.md"
  mkdir -p "$HOME/.claude/agents" "$T/work/claude/agents" "$T/work/claude/commands"
  echo "reviewer" > "$HOME/.claude/agents/reviewer.md"
  echo "deploy" > "$T/work/claude/commands/deploy.md"
}

profile_file() { # $1 = audit true|false, $2 = upstream
  cat > "$T/work/profile.toml" << EOF
dev_root = "$T/work"

[links.agents]
action = "link"
target = "claude/agents"

[links.commands]
action = "link"
target = "claude/commands"

[proxy]
port = $PORT
upstream = "$2"
strip_fields = ["metadata"]
audit = $1
EOF
}

seed_installed() { # $1 = audit, $2 = upstream
  seed_claude
  profile_file "${1:-false}" "${2:-https://api.anthropic.com}"
  mandrake setup --profile "$T/work/profile.toml" --yes > "$T/work/setup.log" 2>&1
}

wait_port() { # opens and closes a TCP connection, so no stray request reaches the proxy's log
  for _ in $(seq 50); do (: > "/dev/tcp/127.0.0.1/$1") 2> /dev/null && return; sleep 0.1; done
}

start_api() {
  python3 "$cli_dir/fake-upstream.py" "$API_PORT" & pids+=("$!")
  wait_port "$API_PORT"
}

start_proxy() {
  mandrake proxy run > "$T/work/proxy.log" 2>&1 & pids+=("$!")
  wait_port "$PORT"
}

send() { # $1 = fixture name; through the proxy, as Claude Code would
  curl -s -o /dev/null -X POST "http://127.0.0.1:$PORT/v1/messages" \
    -H 'content-type: application/json' -H 'anthropic-version: 2023-06-01' \
    -H 'x-api-key: fake' --data-binary "@$MANDRAKE_REPO/tool/tests/fixtures/$1.json"
}

traffic_later() { # requests that arrive a moment later, while a screen is open
  (sleep "${1:-2}"; for f in tools short reminders; do send "$f"; sleep 1; done) > /dev/null 2>&1 &
  pids+=("$!")
}

case "${CLI_SEED:-fresh}" in
  fresh) ;;
  profile) # a profile to apply, nothing installed yet
    seed_claude
    profile_file false https://api.anthropic.com
    cp "$T/work/profile.toml" "$HOME/mandrake-profile.toml" ;;
  installed) seed_installed ;;
  api) # installed, with a fake API the proxy can reach; the proxy itself is not started
    seed_installed true "http://127.0.0.1:$API_PORT"
    start_api ;;
  traffic)
    seed_installed true "http://127.0.0.1:$API_PORT"
    start_api; start_proxy
    for f in short tools reminders; do send "$f"; done
    sleep 1 ;;
  bench)
    seed_installed false "http://127.0.0.1:$API_PORT"
    start_api
    MANDRAKE_BENCH_AUTH="x-api-key: fake" mandrake bench --fixtures "$MANDRAKE_REPO/tool/tests/fixtures" \
      > "$T/work/bench.log" 2>&1 ;;
  *) echo "env.sh: unknown CLI_SEED $CLI_SEED" ;;
esac

cd "$HOME" || return
PS1='\[\e[2m\]~\[\e[0m\] $ '
clear
