#!/usr/bin/env bash
# Records the four #6 cases with anina and keeps the plain logs, under a label (before, after...).
#   tool/tests/e2e/anina/record.sh <label> [path/to/mandrake]
# Writes docs/img/e2e/<label>/<case>.svg and tool/tests/e2e/anina/logs/<label>/<case>.log.
# Needs `anina` on PATH (cargo install --git https://github.com/brennercruvinel/anina anina).
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
label="${1:?label, e.g. before or after}"
export MANDRAKE_BIN="${2:-${MANDRAKE_BIN:-$repo/target/debug/mandrake}}"
[ -x "$MANDRAKE_BIN" ] || { echo "no mandrake binary at $MANDRAKE_BIN"; exit 2; }
# The tapes run from their own folder, so the binary needs an absolute path.
MANDRAKE_BIN="$(cd "$(dirname "$MANDRAKE_BIN")" && pwd)/$(basename "$MANDRAKE_BIN")"
command -v anina > /dev/null || { echo "anina is not on PATH"; exit 2; }
img="$repo/docs/img/e2e/$label"
logs="$here/logs/$label"
out="$(mktemp -d)" || exit 2
trap 'rm -rf "$out"' EXIT
mkdir -p "$img" "$logs"
fails=0
for c in copy-fail same-second doctor-copy eof; do
  "$here/cases.sh" "$c" "$MANDRAKE_BIN" > "$logs/$c.log" 2>&1 || fails=$((fails + 1))
  echo "$c: $(grep '^verdict:' "$logs/$c.log")"
  if ! anina record "$here/$c.tape" -o "$out/$c" > "$logs/$c.anina.log" 2>&1; then
    echo "  anina record failed, see $logs/$c.anina.log"
    continue
  fi
  svg="$(find "$out/$c" -maxdepth 1 -name '*.svg' | head -1)"
  if [ -n "$svg" ]; then
    cp "$svg" "$img/$c.svg"
    echo "  $(du -h "$img/$c.svg" | cut -f1) docs/img/e2e/$label/$c.svg"
  fi
done
echo "$fails of 4 cases show the bug of #6"
