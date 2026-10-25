#!/usr/bin/env bash
# Issue #9: a build, an e2e run and an editor leave nothing for `git status` to show.
# Run from a git checkout; it makes what a working day makes, lists what git sees, and removes
# only the editor and scratch files it created itself.
#   tool/tests/e2e/anina/hygiene.sh
# Ends with "verdict: CLEAN" (exit 0) or "verdict: DIRTY" and the paths (exit 1).
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
cd "$repo" || exit 2
git rev-parse --git-dir > /dev/null 2>&1 || { echo "not a git checkout: $repo"; exit 2; }

# Only paths that did not exist before the run are removed at the end; a real .claude/ or .vscode/
# in the checkout is left alone, and so is everything inside it.
made=()
for p in .vscode .idea .zed .claude tmp .DS_Store scratch.log; do
  [ -e "$p" ] || made+=("$p")
done
trap '[ "${#made[@]}" -eq 0 ] || rm -rf -- "${made[@]}"' EXIT
say() { printf '\n$ %s\n' "$*"; }

say "cargo build --workspace --locked"
cargo build --workspace --locked --quiet 2>&1 | tail -3
say "tool/tests/e2e/anina/cases.sh all"
"$here/cases.sh" all 2>&1 | grep '^verdict:'
say "python3 -m py_compile tool/tests/e2e/pty-answer.py"
python3 -m py_compile tool/tests/e2e/pty-answer.py
say "an editor opens the repo: .vscode .idea .zed .claude tmp .DS_Store scratch.log"
for p in "${made[@]}"; do
  case "$p" in
    .DS_Store) touch "$p" ;;
    scratch.log) echo scratch > "$p" ;;
    *) mkdir -p "$p" && touch "$p/x" ;;
  esac
done

say "git status --short"
left="$(git status --short)"
if [ -z "$left" ]; then
  echo "  (nothing)"
  printf '\nverdict: CLEAN build, e2e run and editor files are all ignored\n'
  exit 0
fi
printf '%s\n' "$left" | sed 's/^/  /'
printf '\nverdict: DIRTY %s path(s) show up in git status\n' "$(printf '%s\n' "$left" | wc -l | tr -d ' ')"
exit 1
