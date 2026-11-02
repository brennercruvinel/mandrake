#!/usr/bin/env bash
# Mandrake next to other plugins and proxies, on a throwaway machine: setup keeps another plugin
# folder, another proxy's ANTHROPIC_BASE_URL and another marketplace's plugins; the launcher sends a
# session to the proxy only when what answers the port is Mandrake's, and leaves a base url set by
# someone else alone. Never touches the real home.
#   tool/tests/e2e/coexist.sh [path/to/mandrake]
set -uo pipefail
for v in $(env | sed -n 's/^\(MANDRAKE_[A-Z_]*\)=.*/\1/p'); do [ "$v" = MANDRAKE_SRC ] || unset "$v"; done
unset ANTHROPIC_BASE_URL
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
bin="${1:-$repo/target/debug/mandrake}"
export MANDRAKE_SRC="${MANDRAKE_SRC:-$repo}"
t="$(mktemp -d)"
pids=()
stop() { [ "${#pids[@]}" -gt 0 ] && kill "${pids[@]}" 2> /dev/null && wait "${pids[@]}" 2> /dev/null; pids=(); }
trap 'stop; rm -rf "$t"' EXIT
fails=0
check() { if [ "$2" = "$3" ]; then echo "ok    $1"; else echo "FAIL  $1: got '$2', want '$3'"; fails=$((fails + 1)); fi; }
# Mandrake's process itself: `m ... &` would give $! the subshell that runs the function, and killing
# that one leaves the proxy behind.
mx() { exec env HOME="$t/home" MANDRAKE_HOME="$t/mh" CLAUDE_CONFIG_DIR="$t/home/.claude" SHELL=/bin/bash "$bin" "$@"; }
m() { (mx "$@"); }
free_port() { python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])'; }
wait_port() { for _ in $(seq 50); do (exec 3<> "/dev/tcp/127.0.0.1/$1") 2> /dev/null && return 0; sleep 0.1; done; return 1; }
port="$(free_port)"

mkdir -p "$t/home/.claude" "$t/other-plugin" "$t/bin"
cat > "$t/home/.claude/settings.json" << EOF
{
  "env": {
    "CLAUDE_CODE_PLUGIN_DIRS": "$t/other-plugin",
    "ANTHROPIC_BASE_URL": "http://localhost:4000",
    "KEEP": "1"
  },
  "enabledPlugins": { "mermaid@someone": true }
}
EOF
printf '[proxy]\nport = %s\n' "$port" > "$t/p.toml"
m setup --profile "$t/p.toml" --yes > "$t/setup.log" 2>&1
check "setup exits 0" "$?" 0
s="$t/home/.claude/settings.json"
trim="$t/mh/data/src/data/plugins/prompt-trim"
check "prompt-trim joins the plugin folders" "$(jq -r .env.CLAUDE_CODE_PLUGIN_DIRS "$s")" "$trim:$t/other-plugin"
check "another proxy's base url stays" "$(jq -r .env.ANTHROPIC_BASE_URL "$s")" "http://localhost:4000"
check "another marketplace's plugin stays" "$(jq -r '.enabledPlugins["mermaid@someone"]' "$s")" true
check "the other keys stay" "$(jq -r .env.KEEP "$s")" 1
m doctor > "$t/doctor.log" 2>&1
check "doctor: prompt-trim counts as set" "$(grep -c '^ok   settings env.CLAUDE_CODE_PLUGIN_DIRS' "$t/doctor.log")" 1
check "doctor: another proxy is a note, not a failure" "$(grep -c 'outro endereço (http://localhost:4000)' "$t/doctor.log")" 1
m setup --profile "$t/p.toml" --yes > "$t/setup2.log" 2>&1
check "a second setup changes no setting" "$(jq -r .env.CLAUDE_CODE_PLUGIN_DIRS "$s")" "$trim:$t/other-plugin"

printf '#!/bin/sh\necho "base=${ANTHROPIC_BASE_URL:-none}"\n' > "$t/bin/claude"
chmod +x "$t/bin/claude"
launch() { env -u ANTHROPIC_BASE_URL HOME="$t/home" MANDRAKE_HOME="$t/mh" PATH="$t/bin:$PATH" bash -c ". '$t/mh/data/shell/mandrake.sh'; claude" 2>&1; }

# Something else on Mandrake's port: the session goes direct, and says why.
python3 -m http.server "$port" --bind 127.0.0.1 > /dev/null 2>&1 &
pids+=("$!")
wait_port "$port"
out="$(launch)"
check "another program on the port: no base url" "$(grep -c 'base=none' <<< "$out")" 1
check "and the launcher says it is not Mandrake" "$(grep -c 'não é o proxy do Mandrake' <<< "$out")" 1
stop

# Mandrake's own proxy on the port: the session goes through it.
mx proxy run --port "$port" > "$t/proxy.log" 2>&1 &
pids+=("$!")
wait_port "$port"
check "the proxy says who it is" "$(curl -fsS -m 2 "http://127.0.0.1:$port/_mandrake")" mandrake
check "Mandrake's proxy: the session goes through it" "$(launch)" "base=http://127.0.0.1:$port"
check "a base url from the shell is left alone" "$(env HOME="$t/home" MANDRAKE_HOME="$t/mh" PATH="$t/bin:$PATH" ANTHROPIC_BASE_URL=http://localhost:4000 bash -c ". '$t/mh/data/shell/mandrake.sh'; claude" 2>&1)" "base=http://localhost:4000"

stop
check "the proxy stops with the test" "$(curl -s -m 1 -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port/_mandrake")" 000

if [ "$fails" -gt 0 ]; then echo "--- setup.log"; cat "$t/setup.log"; echo "--- doctor.log"; cat "$t/doctor.log"; fi
echo
[ "$fails" -eq 0 ] && echo "coexist: every check passed" || echo "coexist: $fails failed"
exit "$fails"
