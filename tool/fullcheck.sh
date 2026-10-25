#!/usr/bin/env bash
# The whole quality gate in one script, run locally before pushing. There is no CI for now: this
# script is the gate, as in urna's tool/tasks/fullcheck.sh.
#
#   tool/fullcheck.sh                every check
#   tool/fullcheck.sh rust shell     only those groups: rust test ts go shell python files news e2e
#
# Each check prints "ok" or "FAIL <check>: <why>" with the tail of its output; the run ends with
# one line per failure and exits 1 when there is any. A missing tool is a failure that says how to
# get it, except anina: without it the tapes are skipped, and the run says so.
#
# Env:
#   FULLCHECK_BASE         the ref the changelog check compares with (default origin/main)
#   FULLCHECK_SKIP_NEWS=1  no changelog fragment needed (a change with the skip-news label)
#   MANDRAKE_PLUGIN_TYPES  a folder with Claude Code's plugin types (claude-code, claude-code-tools,
#                          claude-code-mcp), for a plugin that has none in .claude-plugin/types
set -uo pipefail
cd "$(dirname "$0")/.." || exit 2

# Code files over this many lines fail. Data formats are not counted.
LINE_CAP=639
# An exception is one line, "path|reason", reviewed in the pull request that adds it.
LINE_CAP_EXCEPTIONS=()
# Instruction files for agents: the code, the docs, the ADRs and the changelog are the context.
AGENT_FILES='(^|/)(AGENTS\.md|CLAUDE\.md|GEMINI\.md|\.cursorrules|\.windsurfrules)$|^\.github/copilot-instructions\.md$|(^|/)\.cursor/rules/'
RUFF_VERSION=0.17.0
TSC_VERSION=5.9.3

out="$(mktemp -d)" || exit 2
linked=()
cleanup() {
  [ "${#linked[@]}" -eq 0 ] || rm -f -- "${linked[@]}"
  rm -rf "$out"
}
trap cleanup EXIT
failed=()

# check <name> <command...>: runs it, keeps the output, prints one line.
check() {
  local name="$1" log
  shift
  log="$out/$(printf '%s' "$name" | tr -c 'A-Za-z0-9' '_').log"
  if "$@" > "$log" 2>&1; then
    printf 'ok    %s\n' "$name"
  else
    fail "$name" "$(grep -v '^\s*$' "$log" | tail -1)"
    grep -v '^\s*$' "$log" | tail -15 | sed 's/^/      /'
  fi
}
fail() {
  printf 'FAIL  %s: %s\n' "$1" "$2"
  failed+=("$1: $2")
}
skip() { printf 'skip  %s: %s\n' "$1" "$2"; }
has() { command -v "$1" > /dev/null 2>&1; }
# need <tool> <check> <how to install>: true when the tool is there, a failure line otherwise.
need() {
  has "$1" && return 0
  fail "$2" "$1 is not installed: $3"
  return 1
}
empty() { [ -z "$("$@")" ]; }

rust_checks() {
  need cargo "rust" "rustup.rs" || return
  check "cargo fmt" cargo fmt --all --check
  check "cargo clippy" cargo clippy --workspace --all-targets --locked -- -D warnings
  check "cargo test" cargo test --workspace --locked
  if need cargo-deny "cargo deny" "cargo install cargo-deny --locked"; then
    check "cargo deny" cargo deny check
  fi
}

test_only() {
  need cargo "rust" "rustup.rs" || return
  check "cargo test" cargo test --workspace --locked
}

tsc_run() {
  if has tsc; then
    tsc "$@"
  else
    npx --yes --package "typescript@$TSC_VERSION" tsc "$@"
  fi
}

ts_checks() {
  has tsc || need npx "typescript" "Node.js (nodejs.org), or tsc on PATH" || return
  local p types
  for p in data/plugins/*/tsconfig.json; do
    p="$(dirname "$p")"
    types="$p/.claude-plugin/types"
    if [ ! -e "$types" ] && [ -n "${MANDRAKE_PLUGIN_TYPES:-}" ]; then
      ln -s "$MANDRAKE_PLUGIN_TYPES" "$types" && linked+=("$types")
    fi
    if [ ! -e "$types" ]; then
      skip "tsc ${p##*/}" "no Claude Code plugin types; load the plugin once, or set MANDRAKE_PLUGIN_TYPES"
      continue
    fi
    check "tsc ${p##*/}" tsc_run --noEmit -p "$p"
  done
}

go_checks() {
  need go "go" "go.dev/dl" || return
  check "gofmt" empty gofmt -l data/plugins/fancy/tint
  check "go vet" sh -c 'cd data/plugins/fancy/tint && go vet ./...'
}

shell_checks() {
  need shellcheck "shellcheck" "brew install shellcheck, or apt install shellcheck" || return
  local f files=()
  # A loop, not mapfile: macOS ships bash 3.2.
  while IFS= read -r f; do files+=("$f"); done < <(git ls-files 'install.sh' 'tool/*.sh' 'cfgs/shell/*.sh')
  check "shellcheck" shellcheck -S warning "${files[@]}"
}

ruff_run() {
  if has uvx; then
    uvx --quiet "ruff@$RUFF_VERSION" "$@"
  else
    ruff "$@"
  fi
}

py_checks() {
  has uvx || need ruff "ruff" "docs.astral.sh/uv, then uvx runs ruff" || return
  check "ruff check" ruff_run check --isolated tool
  check "ruff format" ruff_run format --isolated --check tool
}

# Tracked code files over LINE_CAP, minus the listed exceptions.
overlong() {
  local f n e skip_it bad=0
  while IFS= read -r f; do
    [ -f "$f" ] || continue
    n="$(wc -l < "$f" | tr -d ' ')"
    [ "$n" -gt "$LINE_CAP" ] || continue
    skip_it=0
    for e in "${LINE_CAP_EXCEPTIONS[@]+"${LINE_CAP_EXCEPTIONS[@]}"}"; do
      [ "${e%%|*}" = "$f" ] && skip_it=1
    done
    [ "$skip_it" -eq 1 ] && continue
    echo "$f has $n lines, over $LINE_CAP"
    bad=1
  done < <(git ls-files '*.rs' '*.ts' '*.tsx' '*.js' '*.go' '*.py' '*.sh')
  return "$bad"
}

agent_files() {
  local f bad=0
  while IFS= read -r f; do
    echo "$f is tracked; agent instruction files stay out of the repository"
    bad=1
  done < <(git ls-files | grep -E "$AGENT_FILES")
  return "$bad"
}

file_checks() {
  check "line cap" overlong
  check "agent files" agent_files
}

news_check() {
  if [ ! -f towncrier.toml ]; then
    skip "changelog" "no towncrier.toml yet"
    return
  fi
  if [ "${FULLCHECK_SKIP_NEWS:-0}" = 1 ]; then
    skip "changelog" "skip-news"
    return
  fi
  local base="${FULLCHECK_BASE:-origin/main}"
  if has towncrier; then
    check "changelog" towncrier check --compare-with "$base"
  elif need uvx "changelog" "docs.astral.sh/uv, then uvx runs towncrier"; then
    check "changelog" uvx --quiet towncrier check --compare-with "$base"
  fi
}

e2e_checks() {
  local bin=target/debug/mandrake
  need cargo "e2e" "rustup.rs" || return
  check "cargo build" cargo build --workspace --locked
  check "e2e cases" tool/tests/e2e/anina/cases.sh all "$bin"
  check "e2e setup" tool/tests/e2e/setup.sh "$bin"
  check "e2e integrity" tool/tests/e2e/integrity.sh "$bin"
  if ! has anina; then
    skip "anina tapes" "anina is not on PATH; the cases above ran without recording"
    return
  fi
  local tape
  export MANDRAKE_BIN="$PWD/$bin"
  # The four #6 tapes; hygiene and gate run this script themselves and are recorded by hand.
  for tape in copy-fail same-second doctor-copy eof; do
    check "anina $tape" anina record "tool/tests/e2e/anina/$tape.tape" -o "$out/rec/$tape"
  done
}

all_groups=(rust ts go shell python files news e2e)
groups=("$@")
[ "${#groups[@]}" -gt 0 ] || groups=("${all_groups[@]}")
for g in "${groups[@]}"; do
  printf '\n== %s\n' "$g"
  case "$g" in
    rust) rust_checks ;;
    test) test_only ;;
    ts) ts_checks ;;
    go) go_checks ;;
    shell) shell_checks ;;
    python) py_checks ;;
    files) file_checks ;;
    news) news_check ;;
    e2e) e2e_checks ;;
    *) fail "fullcheck" "unknown group $g (rust test ts go shell python files news e2e)" ;;
  esac
done

echo
if [ "${#failed[@]}" -eq 0 ]; then
  echo "fullcheck: everything passed"
  exit 0
fi
echo "fullcheck: ${#failed[@]} failed"
printf '  %s\n' "${failed[@]}"
exit 1
