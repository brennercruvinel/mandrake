#!/usr/bin/env bash
# Issue #15: the gate fails on a 640-line code file and on a tracked CLAUDE.md, and passes once
# both are gone. Run from a git checkout with a clean index; it stages the two files, runs
# tool/fullcheck.sh, takes them back out, and runs the whole gate.
#   tool/tests/e2e/anina/gate.sh
# Ends with "verdict: GATE WORKS" (exit 0) or "verdict: GATE BROKEN" (exit 1).
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
long=tool/long-example.sh
agent=CLAUDE.md
for f in "$long" "$agent"; do
  [ -e "$f" ] && { echo "$f already exists; run this in a checkout without it"; exit 2; }
done
planted() {
  git rm -q --cached --ignore-unmatch -- "$long" "$agent" 2> /dev/null
  rm -f -- "$long" "$agent"
}
trap planted EXIT
say() { printf '\n$ %s\n' "$*"; }

say "a 640-line script and a CLAUDE.md, staged"
{ echo '#!/bin/sh'; for i in $(seq 2 640); do echo ": line $i"; done; } > "$long"
echo "Instructions for an agent." > "$agent"
git add -- "$long" "$agent"
wc -l "$long" | sed 's/^/  /'

say "tool/fullcheck.sh files"
tool/fullcheck.sh files
first=$?

say "both taken back out"
planted

say "tool/fullcheck.sh"
tool/fullcheck.sh
second=$?

if [ "$first" -ne 0 ] && [ "$second" -eq 0 ]; then
  printf '\nverdict: GATE WORKS failed with the two files, passed without them\n'
  exit 0
fi
printf '\nverdict: GATE BROKEN exit %s with the files, %s without\n' "$first" "$second"
exit 1
