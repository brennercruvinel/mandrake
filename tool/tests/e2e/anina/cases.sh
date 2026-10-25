#!/usr/bin/env bash
# The four cases of issue #6, each in a throwaway HOME, against a real `mandrake` binary.
# Every case prints what it runs and what it finds, then one verdict line:
#   verdict: FIXED ...   exit 0, the behavior the issue asks for
#   verdict: BUG ...     exit 1, the behavior the issue describes
# Never touches the real home: HOME, MANDRAKE_HOME and CLAUDE_CONFIG_DIR point into a temp
# folder, and `claude` is a stub, so no plugin install reaches a real Claude Code.
#   tool/tests/e2e/anina/cases.sh <copy-fail|same-second|doctor-copy|eof|all> [path/to/mandrake]
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
bin="${2:-${MANDRAKE_BIN:-$here/../../../../target/debug/mandrake}}"
[ -x "$bin" ] || { echo "no mandrake binary at $bin"; exit 2; }

made=()
trap '[ -n "${KEEP:-}" ] || rm -rf "${made[@]}"' EXIT
fresh() {
  t="$(mktemp -d "${TMPDIR:-/tmp}/mandrake-case.XXXXXX")" || { echo "mktemp failed"; exit 2; }
  [ -d "$t" ] || exit 2; made+=("$t")
  mkdir -p "$t/home/.claude" "$t/bin"
  printf '#!/bin/sh\nexit 0\n' > "$t/bin/claude"; chmod +x "$t/bin/claude"
}
m() {
  env HOME="$t/home" MANDRAKE_HOME="$t/mh" CLAUDE_CONFIG_DIR="$t/home/.claude" SHELL=/bin/bash \
    PATH="$t/bin:$PATH" "$bin" "$@"
}
tidy() { sed "s#$t#\$T#g; s/^/  /"; }
if [ -t 1 ]; then bold=$'\033[1m'; reset=$'\033[0m'; else bold=; reset=; fi
say() { printf '\n%s$ %s%s\n' "$bold" "$*" "$reset"; }
run() { say "mandrake $*"; m "$@" 2>&1 | tidy; return "${PIPESTATUS[0]}"; }
profile() { mkdir -p "$t/mh/config"; printf '%s\n' "$@" > "$t/mh/config/mandrake.toml"; }
verdict() { if [ "$1" = ok ]; then printf '\nverdict: FIXED %s\n' "$2"; return 0; fi
  printf '\nverdict: BUG %s\n' "$2"; return 1; }
journals() { find "$t/mh/state/journal" -name '*.json' ! -name '*.rolled-back.json' 2>/dev/null | wc -l | tr -d ' '; }
backups() { find "$t/mh/state/backups" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l | tr -d ' '; }

copy_fail() {
  echo "== 1. a copy that fails midway, then rollback"
  fresh; mkdir -p "$t/home/.claude/rules"; echo keep-me > "$t/home/.claude/rules/original.txt"
  profile '[links.rules]' 'action = "copy"' "target = \"$t/does-not-exist\""
  run link apply; echo "  exit $?"
  run link rollback; echo "  exit $?"
  say "ls ~/.claude/rules"; ls "$t/home/.claude/rules" 2>&1 | tidy
  say "find state/backups -type f"; (cd "$t/mh/state" 2>/dev/null && find backups -type f) | tidy
  if [ "$(cat "$t/home/.claude/rules/original.txt" 2>/dev/null)" = keep-me ]; then
    verdict ok "original.txt is back in ~/.claude/rules"
  else verdict bug "original.txt is not in ~/.claude/rules; it stays in the backup"; fi
}

same_second() {
  echo "== 2. two applies in the same second, then rollback twice"
  local try
  for try in 1 2 3 4 5; do
    fresh; mkdir -p "$t/home/.claude/rules" "$t/a" "$t/b"; echo keep-me > "$t/home/.claude/rules/original.txt"
    local s0; s0="$(date +%s)"
    profile '[links.rules]' 'action = "link"' "target = \"$t/a\""
    m link apply > "$t/apply-a.log" 2>&1
    profile '[links.rules]' 'action = "link"' "target = \"$t/b\""
    m link apply > "$t/apply-b.log" 2>&1
    [ "$(date +%s)" = "$s0" ] && break
    echo "  (the two applies crossed a second, trying again)"
  done
  say "mandrake link apply   # rules -> a"; tidy < "$t/apply-a.log"
  say "mandrake link apply   # rules -> b, same second"; tidy < "$t/apply-b.log"
  local n; n="$(journals)"
  say "ls state/journal"; ls "$t/mh/state/journal" | tidy
  run link rollback; run link rollback
  local rp="$t/home/.claude/rules"
  say "readlink ~/.claude/rules || ls ~/.claude/rules"
  if [ -L "$rp" ]; then echo "link -> $(readlink "$rp")"; elif [ -d "$rp" ]; then echo "folder: $(ls "$rp")"
  else echo "missing"; fi | tidy
  if [ "$n" = 2 ] && [ "$(cat "$t/home/.claude/rules/original.txt" 2>/dev/null)" = keep-me ] \
    && [ ! -L "$t/home/.claude/rules" ]; then
    verdict ok "two journals, and two rollbacks bring the original folder back"
  else verdict bug "$n journal(s) for two applies; the original folder is not back"; fi
}

doctor_copy() {
  echo "== 3. doctor on a correct copy, and a second setup"
  fresh; mkdir -p "$t/src-rules"; echo rule > "$t/src-rules/a.md"
  cat > "$t/p.toml" <<EOF
[links.rules]
action = "copy"
target = "$t/src-rules"
EOF
  say "mandrake setup --profile p.toml --yes"; m setup --profile "$t/p.toml" --yes > "$t/s1.log" 2>&1
  grep -E 'links|Erros' "$t/s1.log" | tidy
  local b1; b1="$(backups)"
  say "diff -r src-rules ~/.claude/rules"; diff -r "$t/src-rules" "$t/home/.claude/rules" && echo "  same"
  say "mandrake doctor | grep links"; m doctor 2>&1 | grep -i 'links' | tidy
  local dl; dl="$(m doctor 2>&1 | grep -i 'links' | grep -c FAIL)"
  say "mandrake setup --profile p.toml --yes   # again"; m setup --profile "$t/p.toml" --yes > "$t/s2.log" 2>&1
  grep -E 'links' "$t/s2.log" | tidy
  local b2; b2="$(backups)"
  echo "  backup folders: $b1 after the first setup, $b2 after the second"
  if [ "$dl" = 0 ] && [ "$b2" = "$b1" ]; then verdict ok "doctor passes on the copy and the second setup backs nothing up"
  else verdict bug "doctor fails on an identical copy ($dl FAIL) and setup made $((b2 - b1)) new backup(s)"; fi
}

eof() {
  echo "== 4. setup --cli with input closed"
  fresh
  say "mandrake setup --cli < /dev/null"; m setup --cli < /dev/null > "$t/cli.log" 2>&1; local code=$?
  tail -5 "$t/cli.log" | tidy; echo "  exit $code"
  local wrote=""
  [ -f "$t/mh/config/mandrake.toml" ] && wrote="$wrote profile"
  [ -f "$t/home/.claude/settings.json" ] && wrote="$wrote settings.json"
  { cat "$t"/home/.bashrc "$t"/home/.bash_profile "$t"/home/.zshrc "$t"/home/.profile 2>/dev/null || :; } \
    | grep -q ">>> mandrake >>>" && wrote="$wrote shell-block"
  echo "  written:${wrote:- nothing}"
  if [ "$code" != 0 ] && [ -z "$wrote" ]; then verdict ok "closed input exits $code and applies nothing"
  else verdict bug "closed input exits $code and wrote:$wrote"; fi
}

case "${1:-all}" in
  copy-fail) copy_fail ;;
  same-second) same_second ;;
  doctor-copy) doctor_copy ;;
  eof) eof ;;
  all) f=0; for c in copy_fail same_second doctor_copy eof; do "$c" || f=$((f + 1)); echo; done; exit "$f" ;;
  *) echo "unknown case $1"; exit 2 ;;
esac
