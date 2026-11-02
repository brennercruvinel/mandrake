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
check "fancy skipped, already on" "$(grep -c 'plugin fancy: já ligado como fancy@other' "$t/setup.log")" 1
check "rc keeps its line and gets one block" "$(grep -c KEEP "$t/home/.zshrc")$(grep -c '>>> mandrake >>>' "$t/home/.zshrc")" 11
check "env.sh points at the souls" "$(grep -c "MANDRAKE_SOULS='$t/dev/souls'" "$t/mh/data/shell/env.sh")" 1
m setup --profile "$t/p.toml" --yes > "$t/setup2.log" 2>&1
check "second setup changes no link" "$(grep -c 'ok   links: sem mudança' "$t/setup2.log")" 1
check "second setup leaves one block" "$(grep -c '>>> mandrake >>>' "$t/home/.zshrc")" 1
# `mandrake` alone: the next step spends nothing, and every command is listed (#32).
alone="$(m 2>&1)"
check "mandrake alone suggests compare, which spends nothing" "$(grep -c '`mandrake compare` mostra' <<< "$alone")" 1
check "mandrake alone lists hex and capture too" "$(grep -c 'hex · audit · bench · link · proxy · capture' <<< "$alone")" 1
m doctor > "$t/doctor.log" 2>&1
check "the core goes into prompt-trim" "$(grep -c '# Core' "$t/mh/data/src/data/plugins/prompt-trim/hooks/core.js")" 1
check "setup reports no errors" "$(grep -c 'Erros: nenhum' "$t/setup.log")" 1
check "doctor fails only on the proxy" "$(grep FAIL "$t/doctor.log" | grep -vc 'proxy')" 0
# bash on macOS reads the first of .bash_profile, .bash_login and .profile: with only a .profile,
# the block goes there, so no new .bash_profile hides it (second review of #35).
b="$t/bashhome"; mkdir -p "$b/.claude"; echo 'export FROM_PROFILE=1' > "$b/.profile"
env HOME="$b" MANDRAKE_HOME="$b/mh" CLAUDE_CONFIG_DIR="$b/.claude" SHELL=/bin/bash MANDRAKE_PREFIX="$b/pfx" "$bin" setup --profile "$t/p.toml" --yes > "$b/setup.log" 2>&1
if [ "$(uname)" = Darwin ]; then
  check "bash on macOS: the block goes into the existing .profile" "$(grep -c '>>> mandrake >>>' "$b/.profile")$([ -e "$b/.bash_profile" ] && echo ' and a .bash_profile')" 1
else
  check "bash on Linux: the block goes into .bashrc" "$(grep -c '>>> mandrake >>>' "$b/.bashrc")" 1
fi
check "env.sh keeps a custom MANDRAKE_PREFIX" "$(grep -c "MANDRAKE_PREFIX='$b/pfx'" "$b/mh/data/shell/env.sh")" 1
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
# Codex and OpenCode open the same soul: its prompt as instructions, its env, their own server.
for c in codex opencode; do
  printf '#!/bin/sh\nfor a in "$@"; do printf "%%s\\n" "$a"; done\necho "soul=$MANDRAKE_SOUL"\necho "occ=$OPENCODE_CONFIG_CONTENT"\n' > "$t/bin/$c"
  chmod +x "$t/bin/$c"
done
sh_() { env -u ANTHROPIC_BASE_URL -u MANDRAKE_SHELL -u OPENCODE_CONFIG_CONTENT HOME="$t/home" MANDRAKE_HOME="$t/mh" MANDRAKE_PROXY=0 PATH="$t/bin:$PATH" bash -c ". '$t/mh/data/shell/mandrake.sh'; $1" 2>&1; }
cx="$(sh_ 'codex --yolo invoke Nova')"
check "codex: the soul's prompt as developer instructions" "$(grep -c '^developer_instructions=".*# Nova' <<<"$cx")" 1
check "codex: the soul's env set for its commands" "$(grep -c '^shell_environment_policy.set={.*MANDRAKE_SOUL="nova"' <<<"$cx")" 1
check "codex: its own server, --yolo as its flag" "$(grep -c -e '^--no-daemon$' -e '^--dangerously-bypass-approvals-and-sandbox$' <<<"$cx")" 2
check "codex: runs as the soul" "$(grep -c '^soul=nova$' <<<"$cx")" 1
oc="$(sh_ 'opencode invoke nova')"
check "opencode: its own server" "$(grep -c '^--standalone$' <<<"$oc")" 1
check "opencode: the soul's prompt in its instructions" "$(grep -c "^occ={\"instructions\":\[\"$t/mh/state/souls/prompt-nova.md\"\]}" <<<"$oc")" 1
check "opencode: --yolo goes after its subcommand" "$(sh_ 'opencode --yolo invoke nova run hi' | head -4 | tr '\n' ' ')" "--standalone run --auto hi "
check "a plain codex call passes through" "$(sh_ 'codex resume --last' | head -2 | tr '\n' ' ')" "resume --last "
# Claude Code's Bash keeps the functions of the shell and the exported env, but drops the functions
# named _*: claude must work with only its own.
alone="$(sh_ "f=\"\$(declare -f claude)\"; bash -c \"\$f; claude --solo\"")"
check "claude works from a snapshot with only its function" "$(grep -c -- '^--solo$' <<<"$alone")" 1
# /invoke <soul> in a session opened without one binds it; the next /invoke there knows it.
inv() { env -u MANDRAKE_SOUL -u MANDRAKE_SHELL HOME="$t/home" MANDRAKE_HOME="$t/mh" bash -c ". '$t/mh/data/shell/env.sh'; '$t/mh/data/src/data/skills/invoke/invoke.sh' $1" 2>&1; }
check "/invoke without a soul says how to get one" "$(inv s1 | grep -c '^soul: none (claude|codex|opencode invoke <soul>, or /invoke <soul> here: nova')" 1
check "/invoke NOVA binds the session" "$(inv 's1 NOVA' | grep -c '^soul: nova, ligada agora')" 1
check "the next /invoke knows it" "$(inv s1 | head -1)" "soul: nova"
check "invoking the same soul again keeps it" "$(inv 's1 nova' | head -1)" "soul: nova (já era desta sessão)"
# The session, whichever CLI runs this: a trap in place of opencode shows its service is left alone.
soulb="$t/mh/data/shell/soul"
sess() { env -u CLAUDE_CODE_SESSION_ID -u OPENCODE_SESSION_ID -u CODEX_SESSION_ID -u CODEX_THREAD_ID -u HERMES_SESSION_ID \
  HOME="$t/home" MANDRAKE_HOME="$t/mh" XDG_DATA_HOME="$t/data" PATH="$t/trap:$PATH" "$@" "$soulb" session 2>&1; }
check "session in claude" "$(sess CLAUDE_CODE_SESSION_ID=c1)" "$(printf 'claude\tc1')"
check "session in codex" "$(sess CODEX_THREAD_ID=x1)" "$(printf 'codex\tx1')"
check "session in codex is the root's" "$(sess CODEX_THREAD_ID=kid CODEX_SESSION_ID=root)" "$(printf 'codex\troot')"
check "outside a CLI it says so" "$(sess | grep -c 'fora de uma sessão')" 1
mkdir -p "$t/trap" "$t/data/opencode"
printf '#!/bin/sh\ntouch "%s/service-called"\nexit 1\n' "$t" > "$t/trap/opencode"; chmod +x "$t/trap/opencode"
if command -v sqlite3 > /dev/null; then
  sqlite3 "$t/data/opencode/opencode.db" "create table session_v2 (id text, parent_id text);
    insert into session_v2 values ('ses_root', null), ('ses_kid', 'ses_root'), ('ses_deep', 'ses_kid');"
  check "an OpenCode subagent is its root" "$(sess OPENCODE_SESSION_ID=ses_deep)" "$(printf 'opencode\tses_root')"
  check "its database answers, the service stays out" "$(ls "$t/service-called" 2>/dev/null | wc -l | tr -d ' ')" 0
  rm "$t/data/opencode/opencode.db"
fi
check "a parent it cannot read is an error, never the child" "$(sess OPENCODE_SESSION_ID=ses_kid | grep -c 'não consegui ler a sessão')" 1
# A skill rendered for any CLI, and what each CLI found gets.
cc="$t/cc"; mkdir -p "$cc/skills/ritual" "$cc/skills/open" "$cc/agents"
printf '%s\n' --- 'name: ritual' 'description: A "ritual": by hand' 'disable-model-invocation: true' 'context: fork' 'agent: scribe' --- '' \
  '!`echo sid=${CLAUDE_SESSION_ID} args=$ARGUMENTS`' 'at ${CLAUDE_SKILL_DIR}' > "$cc/skills/ritual/SKILL.md"
printf '%s\n' --- 'name: open' 'description: The model may start this' --- '' 'body' > "$cc/skills/open/SKILL.md"
printf '%s\n' --- 'name: scribe' 'description: Writes' 'tools: Bash' --- '' 'You write.' > "$cc/agents/scribe.md"
cli() { env -u CLAUDE_CODE_SESSION_ID -u OPENCODE_SESSION_ID HOME="$t/h2" MANDRAKE_HOME="$t/mh" CLAUDE_CONFIG_DIR="$cc" XDG_CONFIG_HOME="$t/h2/.config" PATH="$t/bin:$PATH" "$@"; }
mkdir -p "$t/h2"
ren="$(cli CODEX_THREAD_ID=x1 "$soulb" skill ritual 'a; touch pwned' 2>&1)"
check "a rendered skill says so in its first line" "$(head -1 <<<"$ren")" "<!-- mandrake:skill ritual -->"
check "its command runs, with the session and the arguments" "$(grep -c '^sid=x1 args=a; touch pwned$' <<<"$ren")" 1
check "its folder is filled in" "$(grep -c "^at $cc/skills/ritual$" <<<"$ren")" 1
check "a skill that does not exist is an error" "$(cli "$soulb" skill none 2>&1 | grep -c "skill 'none' não existe")" 1
cli "$soulb" install > "$t/install.log" 2>&1
oc="$t/h2/.config/opencode"
check "opencode gets the command" "$(grep -c "soul skill ritual \$ARGUMENTS" "$oc/commands/ritual.md")" 1
check "a forked skill runs in a child of its agent" "$(grep -c -E '^(agent: scribe|subagent: true)$' "$oc/commands/ritual.md")" 2
check "a description with quotes stays one YAML string" "$(grep -c '^description: "A \\"ritual\\": by hand"$' "$oc/commands/ritual.md")" 1
check "opencode gets the agent as a subagent" "$(grep -c '^mode: subagent$' "$oc/agents/scribe.md")" 1
check "codex gets the shared skill, never by itself" "$(grep -c 'allow_implicit_invocation: false' "$t/h2/.agents/skills/ritual/agents/openai.yaml")" 1
check "a skill the model may start gets no command" "$(ls "$oc/commands/open.md" 2>/dev/null | wc -l | tr -d ' ')" 0
before="$(cat "$oc/commands/ritual.md")"; cli "$soulb" install > /dev/null 2>&1
check "soul install again changes nothing" "$(cat "$oc/commands/ritual.md")" "$before"
echo mine > "$oc/commands/ritual.md"
check "a file that is not Mandrake's stays" "$(cli "$soulb" install 2>&1 | grep -c 'não é do Mandrake, mantido')" 1
check "and keeps its content" "$(cat "$oc/commands/ritual.md")" mine
# What Claude Code has reaches the others: the instructions, the agent's limits, the rest of the skills.
echo "# Rules" > "$cc/CLAUDE.md"
cli CODEX_HOME="$t/h2/.codex" "$soulb" install > "$t/install2.log" 2>&1
check "opencode reads the instructions" "$(cat "$t/h2/.config/opencode/AGENTS.md")" "# Rules"
check "codex reads the instructions" "$(cat "$t/h2/.codex/AGENTS.md")" "# Rules"
check "a skill the model starts reaches codex through a link" "$(readlink "$t/h2/.agents/skills/open")" "$cc/skills/open"
check "what an agent's tools leave out is denied" "$(grep -c '^  - action: \(edit\|webfetch\|websearch\|subagent\)$' "$oc/agents/scribe.md")" 4
check "and what they list stays" "$(grep -c '^  - action: shell$' "$oc/agents/scribe.md")" 0
echo mine > "$t/h2/.codex/own.md"; rm "$t/h2/.codex/AGENTS.md"; mv "$t/h2/.codex/own.md" "$t/h2/.codex/AGENTS.md"
check "instructions of the person stay" "$(cli CODEX_HOME="$t/h2/.codex" "$soulb" install 2>&1 | grep -c 'AGENTS.md: já existe')" 1
# One catalog, every CLI found. The stubs log what they are asked to run.
mkdir -p "$t/stub"
for c in claude codex; do printf '#!/bin/sh\necho "%s $*" >> "%s/mcp.log"\n' "$c" "$t" > "$t/stub/$c"; chmod +x "$t/stub/$c"; done
cat > "$t/catalog.json" <<'JSON'
{"vars": {"ROOT": "/srv/tools"}, "servers": {
  "alpha": {"enabled": true, "config": {"type": "stdio", "command": "${ROOT}/run.sh", "args": ["alpha"], "env": {"K": "${HOME}/v"}}},
  "beta": {"enabled": true, "config": {"type": "http", "url": "http://127.0.0.1:9/mcp", "headers": {"H": "1"}}},
  "off": {"enabled": false, "config": {"type": "stdio", "command": "x"}}}}
JSON
ocfg="$oc/opencode.json"
printf '{"model": "a/b", "mcp": {"servers": {"off": {"type": "local", "command": ["x"]}, "mine": {"type": "local", "command": ["y"]}}}}\n' > "$ocfg"
cli PATH="$t/stub:$t/bin:$PATH" "$soulb" mcp "$t/catalog.json" > "$t/mcp.out" 2>&1
check "opencode gets a local server" "$(jq -c .mcp.servers.alpha "$ocfg")" "{\"type\":\"local\",\"command\":[\"/srv/tools/run.sh\",\"alpha\"],\"environment\":{\"K\":\"$t/h2/v\"}}"
check "opencode gets a remote server" "$(jq -c .mcp.servers.beta "$ocfg")" '{"type":"remote","url":"http://127.0.0.1:9/mcp","headers":{"H":"1"}}'
check "a disabled server leaves" "$(jq -c '.mcp.servers.off // "gone"' "$ocfg")" '"gone"'
check "the rest of the file stays" "$(jq -c '[.model, .mcp.servers.mine.command[0]]' "$ocfg")" '["a/b","y"]'
check "claude gets it from the same catalog" "$(grep -c '^claude mcp add-json alpha {.*/srv/tools/run.sh' "$t/mcp.log")" 1
check "codex gets it, with its env" "$(grep -c "^codex mcp add alpha --env K=$t/h2/v -- /srv/tools/run.sh alpha$" "$t/mcp.log")" 1
check "codex gets a remote one by url" "$(grep -c '^codex mcp add beta --url http://127.0.0.1:9/mcp$' "$t/mcp.log")" 1
check "a disabled one is not added" "$(grep -c 'add.* off' "$t/mcp.log")" 0
rm "$ocfg"; printf '{// mine\n}\n' > "$oc/opencode.jsonc"
check "a config with comments stays the person's" "$(cli PATH="$t/stub:$t/bin:$PATH" "$soulb" mcp "$t/catalog.json" 2>&1 | grep -c 'tem comentários')" 1
check "and no second file appears" "$(ls "$ocfg" 2>/dev/null | wc -l | tr -d ' ')" 0
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
