#!/usr/bin/env bash
# Records the README's demos with anina into docs/img/demo/<name>.svg.
#   tool/demo/record.sh [path/to/mandrake] [names...]
# setup and compare are tapes here; watch-wide, hex-wide and bench-view-wide are the visual audit's tapes
# in tool/tests/e2e/anina/cli/. They all run in a throwaway HOME, and nothing spends a request.
# Needs anina, python3 and, for compare, Claude Code (no login needed).
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
cli="$repo/tool/tests/e2e/anina/cli"
bin="${1:-${MANDRAKE_BIN:-$repo/target/debug/mandrake}}"
[ -x "$bin" ] || { echo "no mandrake binary at $bin"; exit 2; }
command -v anina > /dev/null || { echo "anina is not on PATH"; exit 2; }
MANDRAKE_BIN="$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")"
export MANDRAKE_BIN MANDRAKE_REPO="$repo" CLI_ENV="$cli/env.sh"
shift 2> /dev/null || true
if [ $# -gt 0 ]; then names=("$@"); else names=(setup compare watch-wide hex-wide bench-view-wide); fi
out="$(mktemp -d)" || exit 2
trap 'rm -rf "$out"' EXIT
mkdir -p "$repo/docs/img/demo"
fails=0
for n in "${names[@]}"; do
  tape="$here/$n.tape"
  [ -f "$tape" ] || tape="$cli/$n.tape"
  if anina record "$tape" --theme "$repo/cfgs/anina/mandrake.json" -o "$out/$n" > "$out/$n.log" 2>&1 && [ -f "$out/$n/$n.svg" ]; then
    cp "$out/$n/$n.svg" "$repo/docs/img/demo/$n.svg"
    echo "$n: $(du -h "$repo/docs/img/demo/$n.svg" | cut -f1) docs/img/demo/$n.svg"
  else
    echo "$n: failed"; tail -5 "$out/$n.log"; fails=$((fails + 1))
  fi
done
exit "$fails"
