#!/usr/bin/env bash
# /invoke: who this session is and what its memory recorded. The memory is any command that takes
# `invoke <session-id> [args]` and prints the resume block; Mandrake ships none, the profile names one.
#   invoke.sh <session-id> [args...]
set -uo pipefail
sid="${1:-}"; shift || true
echo "soul: ${MANDRAKE_SOUL:-none (opened without claude invoke <soul>)}"
mem="${MANDRAKE_MEMORY_CMD:-}"
if [ -z "$mem" ]; then echo "memory: none configured (skills.memory_cmd in the Mandrake profile)"; exit 0; fi
out="$($mem invoke "$sid" "$@" 2>&1)" || { echo "memory failed: $(head -1 <<<"$out")"; exit 0; }
printf '%s\n' "$out"
