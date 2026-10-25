#!/usr/bin/env bash
# Records every CLI and TUI tape with anina, one SVG per tape, for the visual audit.
#   tool/tests/e2e/anina/cli/record.sh <platform> [path/to/mandrake] [tape names...]
# Writes docs/img/e2e/cli/<platform>/<tape>.svg and, per tape, the screen text anina read
# (tool/tests/e2e/anina/logs/cli/<platform>/<tape>.txt), the text of every Snap
# (<tape>.snaps.txt) and anina's own log (<tape>.anina.log).
# Each tape types into env.sh: a throwaway HOME, a stub `claude`, a fake API when it needs one.
# Needs `anina` and `python3` on PATH. Nothing here spends a real request.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../../.." && pwd)"
platform="${1:?platform, e.g. linux or macos}"
bin="${2:-${MANDRAKE_BIN:-$repo/target/debug/mandrake}}"
[ -x "$bin" ] || { echo "no mandrake binary at $bin"; exit 2; }
command -v anina > /dev/null || { echo "anina is not on PATH"; exit 2; }
command -v python3 > /dev/null || { echo "python3 is not on PATH (the fake API needs it)"; exit 2; }
MANDRAKE_BIN="$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")"
export MANDRAKE_BIN MANDRAKE_REPO="$repo" CLI_ENV="$here/env.sh"
shift 2 2> /dev/null || shift $#
if [ $# -gt 0 ]; then tapes=("$@"); else
  tapes=(); for f in "$here"/*.tape; do tapes+=("$(basename "$f" .tape)"); done
fi
img="$repo/docs/img/e2e/cli/$platform"
logs="$here/../logs/cli/$platform"
out="$(mktemp -d)" || exit 2
trap 'rm -rf "$out"' EXIT
mkdir -p "$img" "$logs"
fails=0
for t in "${tapes[@]}"; do
  if ! anina record "$here/$t.tape" --theme "$repo/cfgs/anina/mandrake.json" -o "$out/$t" > "$logs/$t.anina.log" 2>&1; then
    echo "$t: anina record failed, see $logs/$t.anina.log"
    fails=$((fails + 1))
  fi
  [ -f "$out/$t/$t.txt" ] && cp "$out/$t/$t.txt" "$logs/$t.txt"
  # Every Snap as text too: the screen at that moment, which the timeline above may have passed.
  for s in "$out/$t"/shots/*.txt; do
    [ -f "$s" ] || continue
    printf '\n### %s\n\n' "$(basename "$s" .txt)"
    cat "$s"
  done > "$logs/$t.snaps.txt"
  if [ -f "$out/$t/$t.svg" ]; then
    cp "$out/$t/$t.svg" "$img/$t.svg"
    echo "$t: $(du -h "$img/$t.svg" | cut -f1) docs/img/e2e/cli/$platform/$t.svg"
  fi
done
echo "${#tapes[@]} tapes, $fails failed"
exit "$fails"
