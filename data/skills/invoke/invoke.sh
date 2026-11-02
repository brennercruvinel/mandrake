#!/usr/bin/env bash
# /invoke: who this session is and what its memory recorded. `/invoke <soul>` in a session opened
# without one binds it first (soul bind). The memory is any command that takes
# `invoke <session-id> [args]` and prints the resume block; Mandrake ships none, the profile names one.
#   invoke.sh <session-id> [soul] [args...]
set -uo pipefail
sid="${1:-}"; shift || true
soul="${MANDRAKE_SHELL:-${XDG_DATA_HOME:-$HOME/.local/share}/mandrake/shell}/soul"
[ -n "${MANDRAKE_HOME:-}" ] && [ -z "${MANDRAKE_SHELL:-}" ] && soul="$MANDRAKE_HOME/data/shell/soul"
[ -x "$soul" ] || { echo "soul: none ($soul is missing; run mandrake setup)"; exit 0; }
if [ -n "${1:-}" ] && "$soul" resolve "$1" >/dev/null 2>&1; then
  "$soul" bind "$1" --session "$sid" 2>&1 || exit 0
  shift
fi
name="$("$soul" whoami --session "$sid")" || { echo "soul: none (claude invoke <soul>, or /invoke <soul> here: $("$soul" list))"; exit 0; }
echo "soul: $name"
mem="${MANDRAKE_MEMORY_CMD:-}"
if [ -z "$mem" ]; then echo "memory: none configured (skills.memory_cmd in the Mandrake profile)"; exit 0; fi
# The memory runs as the soul: with its env, even when the session was bound after it opened.
out="$(eval "$("$soul" env "$name")" && $mem invoke "$sid" "$@" 2>&1)" || { echo "memory failed: $(head -1 <<<"$out")"; exit 0; }
printf '%s\n' "$out"
