#!/usr/bin/env bash
# End to end on a throwaway machine: a fake ~/.claude with links into a dev folder is captured,
# its links are removed, and `mandrake setup --profile --yes` must bring the same links back,
# with doctor passing on everything but the proxy (not started here). Never touches the real home.
#   tool/tests/e2e/setup.sh [path/to/mandrake]
set -uo pipefail
# A shell with Mandrake installed exports MANDRAKE_SOULS, MANDRAKE_CORE... and env.sh keeps what is
# already set, so the throwaway install would read the real souls. Only MANDRAKE_SRC passes through.
for v in $(env | sed -n 's/^\(MANDRAKE_[A-Z_]*\)=.*/\1/p'); do [ "$v" = MANDRAKE_SRC ] || unset "$v"; done
unset ANTHROPIC_BASE_URL
bin="${1:-$(cd "$(dirname "$0")/../../.." && pwd)/target/debug/mandrake}"
t="$(mktemp -d)"; trap 'rm -rf "$t"' EXIT
fails=0
check() { if [ "$2" = "$3" ]; then echo "ok    $1"; else echo "FAIL  $1: got '$2', want '$3'"; fails=$((fails + 1)); fi; }
m() { env HOME="$t/home" MANDRAKE_HOME="$t/mh" CLAUDE_CONFIG_DIR="$t/home/.claude" SHELL=/bin/zsh "$bin" "$@"; }

mkdir -p "$t/home/.claude/agents" "$t/dev/devops/rules" "$t/dev/agent/skills/invoke" "$t/dev/souls/nova"
echo '{"env":{"KEEP":"1"},"enabledPlugins":{"fancy@other":true}}' > "$t/dev/devops/settings.json"
echo "# Nova" > "$t/dev/souls/nova/NOVA.md"; echo '{}' > "$t/dev/souls/nova/settings.json"
echo "---" > "$t/dev/agent/skills/invoke/SKILL.md"; echo "# toth" > "$t/dev/souls/toth.md"
ln -s "$t/dev/devops/settings.json" "$t/home/.claude/settings.json"
ln -s "$t/dev/devops/rules" "$t/home/.claude/rules"
ln -s "$t/dev/agent/skills" "$t/home/.claude/skills"
ln -s "$t/dev/souls/toth.md" "$t/home/.claude/agents/toth.md"
echo "export KEEP=1" > "$t/home/.zshrc"
before="$(cd "$t/home/.claude" && find . -type l -exec sh -c 'echo "$1 -> $(readlink "$1")"' _ {} \; | sort)"

m capture --out "$t/p.toml" >/dev/null
check "capture finds the dev root" "$(grep -c "^dev_root = \"$t/dev\"" "$t/p.toml")" 1
check "capture finds the 4 links" "$(grep -c 'action = "link"' "$t/p.toml")" 4
check "capture sees fancy on" "$(grep -c '^fancy = true' "$t/p.toml")" 1
printf '\n' >> "$t/p.toml"
sed -i.bak 's/^\[souls\]$/[souls]\ndir = "souls"/' "$t/p.toml" 2>/dev/null
grep -q '^dir = "souls"' "$t/p.toml" || printf '[souls]\ndir = "souls"\n' >> "$t/p.toml"

find "$t/home/.claude" -type l -delete
m setup --profile "$t/p.toml" --yes > "$t/setup.log" 2>&1
check "setup exits 0" "$?" 0
after="$(cd "$t/home/.claude" && find . -type l -exec sh -c 'echo "$1 -> $(readlink "$1")"' _ {} \; | sort)"
check "same links after setup" "$after" "$before"
check "settings keeps its keys" "$(jq -r .env.KEEP "$t/dev/devops/settings.json")" 1
check "settings never gets the base url" "$(jq -r .env.ANTHROPIC_BASE_URL "$t/dev/devops/settings.json")" "null"
check "env.sh carries the proxy port" "$(grep -c "MANDRAKE_PORT='8787'" "$t/mh/data/shell/env.sh")" 1
mkdir -p "$t/home/.local/bin"
check "a new shell finds mandrake's folder" "$(env -u MANDRAKE_BIN_DIR HOME="$t/home" MANDRAKE_HOME="$t/mh" PATH=/usr/bin:/bin bash -c ". '$t/mh/data/shell/mandrake.sh'; echo \"\$PATH\"" | grep -c "^$t/home/.local/bin:")" 1
check "settings gets prompt-trim" "$(jq -r .env.CLAUDE_CODE_PLUGIN_DIRS "$t/dev/devops/settings.json")" "$t/mh/data/src/data/plugins/prompt-trim"
check "prompt-trim copied" "$([ -f "$t/mh/data/src/data/plugins/prompt-trim/hooks/register.js" ] && echo y)" y
check "fancy skipped, already on" "$(grep -c 'plugin fancy: already on as fancy@other' "$t/setup.log")" 1
check "rc keeps its line and gets one block" "$(grep -c KEEP "$t/home/.zshrc")$(grep -c '>>> mandrake >>>' "$t/home/.zshrc")" 11
check "env.sh points at the souls" "$(grep -c "MANDRAKE_SOULS='$t/dev/souls'" "$t/mh/data/shell/env.sh")" 1
m setup --profile "$t/p.toml" --yes > "$t/setup2.log" 2>&1
check "second setup changes no link" "$(grep -c 'ok   links: sem mudança' "$t/setup2.log")" 1
check "second setup leaves one block" "$(grep -c '>>> mandrake >>>' "$t/home/.zshrc")" 1
m doctor > "$t/doctor.log" 2>&1
check "the core goes into prompt-trim" "$(grep -c '# Core' "$t/mh/data/src/data/plugins/prompt-trim/hooks/core.js")" 1
check "setup reports no errors" "$(grep -c 'Erros: nenhum' "$t/setup.log")" 1
check "doctor fails only on the proxy" "$(grep FAIL "$t/doctor.log" | grep -vc 'proxy')" 0
# The launcher finds the soul and builds the claude call (claude stubbed).
mkdir -p "$t/bin"; cat > "$t/bin/claude" <<'STUB'
#!/bin/sh
prev=""; for a in "$@"; do printf '%s\n' "$a"; [ "$prev" = --system-prompt-file ] && head -1 "$a"; prev="$a"; done
STUB
chmod +x "$t/bin/claude"
call="$(env -u ANTHROPIC_BASE_URL HOME="$t/home" MANDRAKE_HOME="$t/mh" MANDRAKE_PROXY=0 PATH="$t/bin:$PATH" bash -c ". '$t/mh/data/shell/mandrake.sh'; claude --x invoke NOVA" | tr '\n' ' ')"
check "launcher opens the soul" "$(grep -c -- "--settings $t/dev/souls/nova/settings.json --system-prompt-file" <<<"$call")" 1
check "launcher names it and passes the flags" "$(grep -c -- '--name Nova --x' <<<"$call")" 1
check "the core goes before the soul" "$(grep -c -- '# Core' <<<"$call")" 1
# Proxy down: the launcher must still start claude, without a base url, and say why.
down="$(env -u ANTHROPIC_BASE_URL HOME="$t/home" MANDRAKE_HOME="$t/mh" PATH="$t/bin:$PATH" bash -c ". '$t/mh/data/shell/mandrake.sh'; MANDRAKE_PORT=1 claude --y; env | grep -c ANTHROPIC_BASE_URL" 2>&1)"
check "proxy down: claude still starts" "$(grep -c -- '--y' <<<"$down")" 1
check "proxy down: the session goes direct, and says so" "$(grep -c 'vai direto para a API' <<<"$down")" 1
# A stale MANDRAKE_* inherited from a parent shell loses to the installed value (#33), and doctor
# names it in the shell that still has it.
souls="$(env HOME="$t/home" MANDRAKE_HOME="$t/mh" MANDRAKE_SOULS=/old PATH="$t/bin:$PATH" bash -c ". '$t/mh/data/shell/mandrake.sh'; echo \"\$MANDRAKE_SOULS\"")"
check "a stale MANDRAKE_SOULS loses to the installed one" "$souls" "$t/dev/souls"
check "doctor names the stale variable" "$(MANDRAKE_SOULS=/old m doctor 2>&1 | grep -c 'MANDRAKE_SOULS é /old')" 1
if [ "$fails" -gt 0 ]; then echo "--- setup.log"; cat "$t/setup.log"; echo "--- doctor.log"; cat "$t/doctor.log"; fi
exit "$fails"
