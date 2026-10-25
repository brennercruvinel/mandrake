#!/usr/bin/env bash
# Records install.tape with anina under a label and keeps the screen text as the log.
#   tool/tests/e2e/anina/install/record.sh <label> [checkout]
# The checkout defaults to this one; pass an older one (a worktree at main) for the "before" side.
# Writes docs/img/e2e/install/<label>.svg and tool/tests/e2e/anina/logs/install/<label>.txt.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../../../../.." && pwd)"
label="${1:?label, e.g. before or after}"
if [ -n "${2:-}" ]; then
  MANDRAKE_CHECKOUT="$(cd "$2" && pwd)" || exit 2
  export MANDRAKE_CHECKOUT
fi
command -v anina > /dev/null || { echo "anina is not on PATH"; exit 2; }
# anina gives the tape its own HOME; the sandbox still needs the real toolchain.
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
out="$(mktemp -d)" || exit 2
old="$(ls -d /tmp/mandrake-install.* 2> /dev/null)"
# The sandbox HOMEs this run made go too; older ones are left alone.
tidy() {
  rm -rf "$out"
  for d in /tmp/mandrake-install.*; do
    [ -d "$d" ] && ! grep -qxF "$d" <<< "$old" && rm -rf "$d"
  done
}
trap tidy EXIT
mkdir -p "$repo/docs/img/e2e/install" "$here/../logs/install"
anina record "$here/install.tape" --theme "$repo/cfgs/anina/mandrake.json" -o "$out/install" 2>&1 | sed "s#$out#\$OUT#g"
cp "$out/install/install.svg" "$repo/docs/img/e2e/install/$label.svg" || exit 1
sed 's#/tmp/mandrake-install\.[A-Za-z0-9]*#$T#g' "$out/install/install.txt" \
  > "$here/../logs/install/$label.txt"
grep -h '^ *verdict:' "$here/../logs/install/$label.txt" | tail -1
