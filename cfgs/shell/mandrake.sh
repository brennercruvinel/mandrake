# Mandrake's shell file, sourced from the rc by the block `mandrake setup` writes (bash and zsh).
# `claude [flags] invoke <soul>` opens a soul: the core prompt and its own as the whole system prompt, its settings,
# its own gh login when ~/.config/gh-<soul> exists, and /invoke first when that skill is installed.
# Any other call is plain Claude Code.
# shellcheck shell=bash
_mandrake_env="${XDG_DATA_HOME:-$HOME/.local/share}/mandrake/shell/env.sh"
[ -n "${MANDRAKE_HOME:-}" ] && _mandrake_env="$MANDRAKE_HOME/data/shell/env.sh"
# shellcheck source=/dev/null
[ -f "$_mandrake_env" ] && . "$_mandrake_env"

# The folder install.sh puts `mandrake` in, and tui-embed its apps (~/.local/bin by default). macOS
# does not put it on PATH, so a new shell would answer `command not found` right after the install.
if [ -n "${MANDRAKE_BIN_DIR:-}" ] && [ -d "$MANDRAKE_BIN_DIR" ]; then
  case ":$PATH:" in *":$MANDRAKE_BIN_DIR:"*) ;; *) export PATH="$MANDRAKE_BIN_DIR:$PATH" ;; esac
fi

# The proxy is used per session, and only when it answers: a proxy that is down sends the session
# straight to the API instead of breaking it. MANDRAKE_PORT comes from env.sh; MANDRAKE_PROXY=0 skips it.
_mandrake_route() {
  [ -n "${ANTHROPIC_BASE_URL:-}" ] && return 0
  [ "${MANDRAKE_PROXY:-1}" = 0 ] && return 0
  local port="${MANDRAKE_PORT:-8787}"
  if (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null || nc -z -w 1 127.0.0.1 "$port" 2>/dev/null; then
    export ANTHROPIC_BASE_URL="http://127.0.0.1:$port"
  else
    echo "mandrake: proxy fora do ar em 127.0.0.1:$port, esta sessão vai direto para a API (mandrake doctor)" >&2
  fi
}

claude() {
  local a name="" want=0 rest=()
  for a in "$@"; do
    if [ "$want" = 1 ]; then
      name="$(_mandrake_soul "$a")" || return 1
      want=2
    elif [ "$want" = 0 ] && { [ "$a" = invoke ] || [ "$a" = /invoke ]; }; then
      want=1
    else
      rest+=("$a")
    fi
  done
  case "$want" in
    1) echo "claude invoke: qual soul? ($(_mandrake_souls))" >&2; return 1 ;;
    2) ( _mandrake_route; _mandrake_summon "$name" "${rest[@]}" ) ;;
    *) ( _mandrake_route; command claude "$@" ) ;;
  esac
}

# A soul's folder from its name or an alias in <souls>/agents.json, case-insensitive.
_mandrake_soul() {
  local n d dir="${MANDRAKE_SOULS:-}"
  [ -d "$dir" ] || { echo "claude invoke: sem pasta de souls (MANDRAKE_SOULS='$dir'); rode mandrake setup" >&2; return 1; }
  n="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  d=""
  if [ -f "$dir/agents.json" ] && command -v jq >/dev/null; then
    d="$(jq -r --arg n "$n" 'to_entries[] | select([.key] + [.value] | flatten | map(ascii_downcase) | index($n))
      | .key' "$dir/agents.json" 2>/dev/null | head -1)"
  fi
  [ -n "$d" ] || d="$n"
  if [ -n "$(_mandrake_prompt "$dir/$d")" ]; then echo "$d"; return 0; fi
  echo "claude invoke: '$1' não é uma soul ($(_mandrake_souls))" >&2
  return 1
}

# <NAME>.md, else SOUL.md.
_mandrake_prompt() {
  local up
  up="$(basename "$1" | tr '[:lower:]' '[:upper:]')"
  if [ -f "$1/$up.md" ]; then echo "$1/$up.md"; elif [ -f "$1/SOUL.md" ]; then echo "$1/SOUL.md"; fi
}

_mandrake_souls() {
  local d out=""
  for d in "${MANDRAKE_SOULS:-/nonexistent}"/*/; do
    [ -n "$(_mandrake_prompt "${d%/}")" ] && out="$out${out:+, }$(basename "$d")"
  done
  echo "${out:-nenhuma}"
}

_mandrake_summon() (
  name="$1"; shift
  dir="$MANDRAKE_SOULS/$name"
  prompt="${TMPDIR:-/tmp}/mandrake-soul-$name-$$.md"
  # The front matter (between the first two --- lines) is for agent definitions, not for the prompt.
  # The core goes first when the profile has one; the soul follows.
  { if [ -f "${MANDRAKE_CORE:-}" ] && [ "${MANDRAKE_CORE_SOULS:-1}" = 1 ]; then cat "$MANDRAKE_CORE"; printf '\n'; fi
    awk 'NR == 1 && /^---$/ { fm = 1; next } fm && /^---$/ { fm = 0; next } !fm' "$(_mandrake_prompt "$dir")"; } > "$prompt"
  export MANDRAKE_SOUL="$name"
  [ -d "${XDG_CONFIG_HOME:-$HOME/.config}/gh-$name" ] && export GH_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/gh-$name"
  extra=(--system-prompt-file "$prompt" --name "$(printf '%s' "${name:0:1}" | tr '[:lower:]' '[:upper:]')${name:1}")
  [ -f "$dir/settings.json" ] && extra=(--settings "$dir/settings.json" "${extra[@]}")
  first=()
  [ "${MANDRAKE_INVOKE:-0}" = 1 ] && first=(/invoke)
  for a in "$@"; do case "$a" in -p|--print|-c|--continue|-r|--resume|--resume=*) first=() ;; esac; done
  command claude "${extra[@]}" "$@" ${first[@]+"${first[@]}"}
  rc=$?
  rm -f "$prompt"
  exit $rc
)
