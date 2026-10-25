#!/usr/bin/env bash
# #14: the records answer the three questions their issue asks, run from a clone so nothing here
# changes. Needs git, rg and uvx (towncrier runs through uvx).
#   tool/tests/e2e/anina/records/check.sh [base-branch]
set -uo pipefail
repo="$(cd "$(dirname "$0")/../../../../.." && pwd)"
base="${1:-origin/main}"
t="$(mktemp -d)" || exit 2
trap 'rm -rf "$t"' EXIT
git clone -q "$repo" "$t/r" && cd "$t/r" || exit 2
git fetch -q "$repo" "$base:refs/remotes/base" 2>/dev/null || git branch -q -f base "$(git -C "$repo" rev-parse "$base")"
cmp_ref="$(git rev-parse -q --verify refs/remotes/base || git rev-parse base)"
say() { printf '\n$ %s\n' "$*"; }
fails=0
say "rg -l '^status: accepted' docs/adr"
n="$(rg -l '^status: accepted' docs/adr | tee /dev/stderr | wc -l | tr -d ' ')" 2>&1
[ "$n" = 4 ] && echo "ok: the four seed ADRs" || { echo "FAIL: $n accepted ADRs"; fails=$((fails + 1)); }
say "uvx towncrier build --draft --version 0.2.0"
uvx -q towncrier build --draft --version 0.2.0 2>/dev/null | sed -n '/^## /,$p'
say "uvx towncrier check   (this branch, with fragments)"
if uvx -q towncrier check --compare-with "$cmp_ref" > /dev/null 2>&1; then echo "ok: passes"; else echo "FAIL"; fails=$((fails + 1)); fi
git switch -q -c no-fragment && echo >> docs/TERMS.md && git commit -qam "a change without a fragment"
say "uvx towncrier check   (a branch that changes a file and adds no fragment)"
if uvx -q towncrier check --compare-with "$(git rev-parse HEAD~1)" > /dev/null 2>&1; then echo "FAIL: passed"; fails=$((fails + 1)); else echo "ok: fails, as it should"; fi
printf '\nverdict: %s\n' "$([ "$fails" = 0 ] && echo "PASS records of #14" || echo "FAIL $fails check(s)")"
[ "$fails" = 0 ]
