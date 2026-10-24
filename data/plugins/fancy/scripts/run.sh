#!/bin/sh
# Runs the fancy-tint binary: the plugin's own build, then one on PATH,
# otherwise builds it once with `go build`. Never fails the calling hook.
root="$(cd "$(dirname "$0")/.." && pwd)"
for bin in "$root/bin/fancy-tint" "$root/bin/fancy-tint.exe"; do
  [ -x "$bin" ] && exec "$bin" "$@"
done
if command -v fancy-tint >/dev/null 2>&1; then
  exec fancy-tint "$@"
fi
if command -v go >/dev/null 2>&1 && (cd "$root/tint" && go build -o "$root/bin/" ./cmd/fancy-tint) >/dev/null 2>&1; then
  for bin in "$root/bin/fancy-tint" "$root/bin/fancy-tint.exe"; do
    [ -x "$bin" ] && exec "$bin" "$@"
  done
fi
exit 0
