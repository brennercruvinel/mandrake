# Mandrake's shell file, sourced from the rc by the block `mandrake setup` writes (bash and zsh).
# `claude [flags] invoke <soul>` opens a soul: its prompt as the whole system prompt, its settings,
# its own gh login when ~/.config/gh-<soul> exists, and /invoke first when that skill is installed.
# Any other call is plain Claude Code.
_mandrake_env="${XDG_DATA_HOME:-$HOME/.local/share}/mandrake/shell/env.sh"
[ -n "${MANDRAKE_HOME:-}" ] && _mandrake_env="$MANDRAKE_HOME/data/shell/env.sh"
[ -f "$_mandrake_env" ] && . "$_mandrake_env"

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
    1) echo "claude invoke: which soul? ($(_mandrake_souls))" >&2; return 1 ;;
    2) _mandrake_summon "$name" "${rest[@]}" ;;
    *) command claude "$@" ;;
  esac
}

# A soul's folder from its name or an alias in <souls>/agents.json, case-insensitive.
_mandrake_soul() {
  local n d dir="${MANDRAKE_SOULS:-}"
  [ -d "$dir" ] || { echo "claude invoke: no souls folder (MANDRAKE_SOULS='$dir'); run mandrake setup" >&2; return 1; }
  n="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  d=""
  if [ -f "$dir/agents.json" ] && command -v jq >/dev/null; then
    d="$(jq -r --arg n "$n" 'to_entries[] | select([.key] + [.value] | flatten | map(ascii_downcase) | index($n))
      | .key' "$dir/agents.json" 2>/dev/null | head -1)"
  fi
  [ -n "$d" ] || d="$n"
  if [ -n "$(_mandrake_prompt "$dir/$d")" ]; then echo "$d"; return 0; fi
  echo "claude invoke: '$1' is not a soul ($(_mandrake_souls))" >&2
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
  echo "${out:-none}"
}

_mandrake_summon() (
  name="$1"; shift
  dir="$MANDRAKE_SOULS/$name"
  prompt="${TMPDIR:-/tmp}/mandrake-soul-$name-$$.md"
  # The front matter (between the first two --- lines) is for agent definitions, not for the prompt.
  awk 'NR == 1 && /^---$/ { fm = 1; next } fm && /^---$/ { fm = 0; next } !fm' "$(_mandrake_prompt "$dir")" > "$prompt"
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
