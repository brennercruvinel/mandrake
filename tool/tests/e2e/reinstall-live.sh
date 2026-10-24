#!/usr/bin/env bash
# Full reinstall on a real machine, in one run (open Claude Code sessions lose their API only for the
# seconds between steps 3 and 5):
#   1. backup: a zip (mode 600) of ~/.claude (links kept as links, its own small files with them),
#      ~/.claude.json, the settings file the link points to, the shell rc, and every piece of an
#      existing Mandrake install (profile, data, state, binary, service);
#   2. to the Trash: the ~/.claude links, the Mandrake install, the rc block and the two settings
#      keys (the settings file itself is a link target and stays, without them);
#   3. ./install.sh --cli, answered through a pseudo-terminal like a person would: the bracketed default on
#      every question (they come from the saved profile), except the answers given in ANSWERS;
#   4. mandrake doctor, and a real `claude -p` through the new proxy;
#   5. every error, one per line.
#   tool/tests/e2e/reinstall-live.sh <profile.toml> [backup-dir]
#   ANSWERS='core (=padrao'   prompt-substring=answer pairs, separated by ';'
set -uo pipefail
profile="${1:?usage: reinstall-live.sh <profile.toml> [backup-dir]}"
src="$(cd "$(dirname "$0")/../../.." && pwd)"
stamp="$(date +%Y%m%d-%H%M%S)"
backup_dir="${2:-$src/.backup}"
claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
cfg="${XDG_CONFIG_HOME:-$HOME/.config}/mandrake"; data="${XDG_DATA_HOME:-$HOME/.local/share}/mandrake"
state="${XDG_STATE_HOME:-$HOME/.local/state}/mandrake"; bin="$HOME/.local/bin/mandrake"
rc="$HOME/.zshrc"; case "${SHELL:-}" in */bash) rc="$HOME/.bashrc"; [ "$(uname -s)" = Darwin ] && rc="$HOME/.bash_profile" ;; esac
if [ "$(uname -s)" = Darwin ]; then
  trash="$HOME/.Trash/mandrake-reinstall-$stamp"; svc="$HOME/Library/LaunchAgents/dev.mandrake.proxy.plist"
else
  trash="${XDG_DATA_HOME:-$HOME/.local/share}/Trash/files/mandrake-reinstall-$stamp"; svc="$HOME/.config/systemd/user/mandrake-proxy.service"
fi
errors=()
err() { errors+=("$1"); echo "  ERRO $1"; }
links_of() { (cd "$claude_dir" && find . -maxdepth 2 -type l ! -path './debug/*' -exec sh -c 'echo "${1#./} -> $(readlink "$1")"' _ {} \; | sort); }
settings="$(cd "$claude_dir" && readlink settings.json || echo "$claude_dir/settings.json")"
cp "$profile" "/tmp/mandrake-profile-$stamp.toml"; profile="/tmp/mandrake-profile-$stamp.toml"

echo "== 1. backup"
b="$backup_dir/$stamp"; mkdir -p "$b/claude" "$b/files"; chmod 700 "$backup_dir"
(cd "$claude_dir" && find . -maxdepth 2 \( -type l -o -type f -size -2M \) ! -path './debug/*' -print | while read -r f; do
  mkdir -p "$b/claude/$(dirname "$f")"; cp -P "$f" "$b/claude/$f"; done)
before="$(links_of)"; printf '%s\n' "$before" > "$b/links.txt"
for f in "$HOME/.claude.json" "$settings" "$rc" "$svc" "$bin" "$profile"; do [ -f "$f" ] && cp "$f" "$b/files/"; done
for d in "$cfg" "$data" "$state"; do [ -d "$d" ] && tar -C "$(dirname "$d")" --exclude 'mandrake/src/target' --exclude 'mandrake/audit' -czf "$b/files/$(basename "$(dirname "$d")")-mandrake.tgz" mandrake; done
(cd "$backup_dir" && zip -qry "$stamp.zip" "$stamp" && chmod 600 "$stamp.zip" && rm -rf "$stamp")
echo "  ok   $backup_dir/$stamp.zip ($(unzip -l "$backup_dir/$stamp.zip" | tail -1 | awk '{print $2}') arquivos, $(grep -c . <<<"$before") links)"

echo "== 2. para a Lixeira: $trash"
mkdir -p "$trash/claude" "$trash/mandrake"
while read -r l _; do [ -n "$l" ] || continue; mkdir -p "$trash/claude/$(dirname "$l")"; mv "$claude_dir/$l" "$trash/claude/$l"; done <<<"$before"
echo "  ok   $(grep -c . <<<"$before") links de $claude_dir"
if [ "$(uname -s)" = Darwin ]; then launchctl bootout "gui/$(id -u)/dev.mandrake.proxy" 2>/dev/null; else systemctl --user disable --now mandrake-proxy 2>/dev/null; fi
for f in "$svc" "$bin"; do [ -e "$f" ] && mv "$f" "$trash/mandrake/" && echo "  ok   $f"; done
for d in "$cfg" "$data" "$state"; do [ -d "$d" ] && mv "$d" "$trash/mandrake/$(basename "$(dirname "$d")")-mandrake" && echo "  ok   $d"; done
cp "$rc" "$trash/rc"; awk '/^# >>> mandrake >>>$/{skip=1} !skip{print} /^# <<< mandrake <<<$/{skip=0}' "$trash/rc" > "$rc"; echo "  ok   bloco do Mandrake tirado de $rc"
cp "$settings" "$trash/settings.json"; jq 'del(.env.ANTHROPIC_BASE_URL, .env.CLAUDE_CODE_PLUGIN_DIRS)' "$trash/settings.json" > "$settings"
echo "  ok   ANTHROPIC_BASE_URL e CLAUDE_CODE_PLUGIN_DIRS tirados de $settings"
echo "  sobrou: $(links_of | grep -c .) links, mandrake no PATH: $(command -v mandrake || echo nenhum)"

echo "== 3. ./install.sh --cli --from <perfil salvo>, respondido por um terminal"
python3 - "$src" "$profile" "${ANSWERS:-}" <<'PY'
import os, pty, sys, re, select
src, prof, answers = sys.argv[1], sys.argv[2], sys.argv[3]
special = [tuple(p.split("=", 1)) for p in answers.split(";") if "=" in p]
pid, fd = pty.fork()
if pid == 0:
    os.chdir(src); os.execvp("bash", ["bash", "./install.sh", "--cli", "--from", prof])
buf = b""; out = sys.stdout.buffer
while True:
    r, _, _ = select.select([fd], [], [], 300)
    if not r: print("\n  ERRO install.sh sem saída por 300 s"); break
    try: chunk = os.read(fd, 4096)
    except OSError: break
    if not chunk: break
    out.write(chunk); out.flush(); buf += chunk
    tail = buf.decode("utf8", "replace").rsplit("\n", 1)[-1]
    m = re.search(r"\[([^\]]*)\]: $", tail)
    if m:
        # Type the bracketed default (or the given answer), so the log shows every answer.
        reply = next((a for q, a in special if q in tail), m.group(1))
        os.write(fd, (reply + "\r").encode()); buf = b""
_, st = os.waitpid(pid, 0)
sys.exit(os.waitstatus_to_exitcode(st))
PY
[ $? = 0 ] || err "install.sh terminou com erro"

echo "== 4. checagens"
up=0; for _ in $(seq 1 30); do nc -z 127.0.0.1 "$(sed -n 's/^port = \([0-9]*\)$/\1/p' "$profile" | head -1)" 2>/dev/null && { up=1; break; }; sleep 0.3; done
[ "$up" = 1 ] && echo "  ok   proxy respondendo" || err "proxy não respondeu"
after="$(links_of)"
[ "$after" = "$before" ] && echo "  ok   os mesmos $(grep -c . <<<"$after") links de antes" || { err "links diferentes do backup"; diff <(echo "$before") <(echo "$after") | sed 's/^/       /'; }
while read -r line; do case "$line" in FAIL*) err "doctor: ${line#FAIL }" ;; *) echo "  $line" ;; esac; done < <("$HOME/.local/bin/mandrake" doctor 2>&1)
n0="$(cat "$state/events.jsonl" 2>/dev/null | grep -c .)"
reply="$(cd /tmp && claude -p -- "Responda só: ok-reinstall" 2>&1 | tail -1)"
sleep 4
n1="$(cat "$state/events.jsonl" 2>/dev/null | grep -c .)"
[ "$reply" = ok-reinstall ] && echo "  ok   claude -p respondeu: $reply" || err "claude -p respondeu: $reply"
[ "$n1" -gt "$n0" ] && echo "  ok   a requisição passou pelo proxy novo: $(tail -1 "$state/events.jsonl" | jq -r '"tokens \(.tokens_before) -> \(.tokens_after), cortes system \(.cuts.system_blocks) reminders \(.cuts.reminders) campos \(.cuts.fields|join(","))"')" || err "nenhum evento novo no proxy"

echo "== 5. erros (${#errors[@]})"
if [ ${#errors[@]} = 0 ]; then echo "  nenhum"; else for e in "${errors[@]}"; do echo "  - $e"; done; fi
echo "backup: $backup_dir/$stamp.zip   lixeira: $trash"
exit ${#errors[@]}
