#!/usr/bin/env bash
# The rebuild POC on a real machine, in one run so no open Claude Code session is left without its
# links or its API for more than a few seconds:
#   1. what Mandrake recreates goes to a folder in the Trash: the ~/.claude links named in the
#      profile, the line that sourced an older launcher, an older proxy service, and the two
#      settings keys (the settings file itself stays, it is a link target);
#   2. `mandrake setup --profile <file> --yes`;
#   3. checks: the same links as before, the proxy answering on its port, doctor.
# If the proxy does not answer, everything is put back and the old service restarted.
#   tool/tests/e2e/rebuild-live.sh <profile.toml> [old-launchd-label] [old-rc-line-pattern]
set -uo pipefail
profile="${1:?usage: rebuild-live.sh <profile.toml> [old-launchd-label] [old-rc-pattern]}"
old_label="${2:-}"; old_rc="${3:-}"
src="$(cd "$(dirname "$0")/../../.." && pwd)"
bin="${MANDRAKE_BIN:-$HOME/.local/bin/mandrake}"
claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
rc="$HOME/.zshrc"; case "${SHELL:-}" in */bash) rc="$HOME/.bashrc"; [ "$(uname -s)" = Darwin ] && rc="$HOME/.bash_profile" ;; esac
trash="$HOME/.Trash/mandrake-rebuild-$(date +%Y%m%d-%H%M%S)"
[ "$(uname -s)" = Darwin ] || trash="${XDG_DATA_HOME:-$HOME/.local/share}/Trash/files/mandrake-rebuild-$(date +%Y%m%d-%H%M%S)"
[ -x "$bin" ] || { echo "no mandrake binary at $bin (cargo install --path pkgs/cli --root ~/.local)"; exit 1; }
port="$(sed -n 's/^port = \([0-9]*\)$/\1/p' "$profile" | head -1)"; port="${port:-8787}"
mkdir -p "$trash/claude"
links_of() { (cd "$claude_dir" && find . -maxdepth 2 -type l ! -path './debug/*' -exec sh -c 'echo "${1#./} -> $(readlink "$1")"' _ {} \; | sort); }
before="$(links_of)"; printf '%s\n' "$before" > "$trash/links-before.txt"
settings="$(cd "$claude_dir" && readlink settings.json || echo "$claude_dir/settings.json")"
cp "$settings" "$trash/settings.json"; cp "$rc" "$trash/rc"

echo "1. to the Trash: $trash"
while read -r l _; do
  [ -n "$l" ] || continue
  mkdir -p "$trash/claude/$(dirname "$l")"; mv "$claude_dir/$l" "$trash/claude/$l"
done <<<"$before"
if [ -n "$old_rc" ]; then grep -v -- "$old_rc" "$trash/rc" > "$rc"; fi
if [ -n "$old_label" ] && [ "$(uname -s)" = Darwin ]; then
  launchctl bootout "gui/$(id -u)/$old_label" 2>/dev/null
  [ -f "$HOME/Library/LaunchAgents/$old_label.plist" ] && mv "$HOME/Library/LaunchAgents/$old_label.plist" "$trash/"
fi
jq 'del(.env.ANTHROPIC_BASE_URL, .env.CLAUDE_CODE_PLUGIN_DIRS)' "$trash/settings.json" > "$settings"
echo "   left: $(links_of | grep -c . ) links, settings without the two keys"

echo "2. mandrake setup"
MANDRAKE_SRC="$src" "$bin" setup --profile "$profile" --yes 2>&1 | sed 's/^/   /'

echo "3. checks"
up=0; for _ in $(seq 1 30); do nc -z 127.0.0.1 "$port" 2>/dev/null && { up=1; break; }; sleep 0.3; done
if [ "$up" != 1 ]; then
  echo "   proxy not answering on $port: putting everything back"
  "$bin" proxy uninstall 2>/dev/null; "$bin" link rollback 2>/dev/null
  (cd "$trash/claude" && find . -type l | while read -r l; do mkdir -p "$claude_dir/$(dirname "$l")"; mv "$l" "$claude_dir/$l"; done)
  cp "$trash/settings.json" "$settings"; cp "$trash/rc" "$rc"
  if [ -n "$old_label" ] && [ -f "$trash/$old_label.plist" ]; then
    mv "$trash/$old_label.plist" "$HOME/Library/LaunchAgents/"; launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/$old_label.plist"
  fi
  exit 1
fi
echo "   proxy answering on $port"
after="$(links_of)"
if [ "$after" = "$before" ]; then echo "   links: the same $(grep -c . <<<"$after") as before"; else echo "   links differ:"; diff <(echo "$before") <(echo "$after") | sed 's/^/   /'; fi
"$bin" doctor 2>&1 | sed 's/^/   /'
