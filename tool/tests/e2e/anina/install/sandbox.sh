#!/usr/bin/env bash
# A new user's machine for the install test: an empty HOME, the PATH the OS gives a new account
# (/etc/paths, plus Homebrew where it exists) with cargo, jq and a stub claude, and no Mandrake
# variables. MANDRAKE_HOME keeps the proxy service out of launchd and systemd, so nothing outside
# the temp folder changes. Runs the command from the checkout root.
#   tool/tests/e2e/anina/install/sandbox.sh <command...>
# MANDRAKE_CHECKOUT runs another checkout (an older commit, for a before/after pair).
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
repo="${MANDRAKE_CHECKOUT:-$(cd "$here/../../../../.." && pwd)}"
real_home="$HOME"
# /tmp, not TMPDIR: on macOS TMPDIR is a per-user folder, and every path below ends up on screen.
t="$(mktemp -d /tmp/mandrake-install.XXXXXX)" || exit 2
mkdir -p "$t/home" "$t/tools"

# Tools a new user has before installing: cargo (rustup's proxies dispatch on their own name), jq.
for tool in cargo rustc rustup jq; do
  p="$(command -v "$tool")" && ln -s "$p" "$t/tools/$tool"
done
printf '#!/bin/sh\n[ "$1" = --version ] && echo "2.1.0 (Claude Code, stub)"\nexit 0\n' > "$t/tools/claude"
chmod +x "$t/tools/claude"

base="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
[ -r /etc/paths ] && base="$(paste -sd: /etc/paths)"
[ -d /opt/homebrew/bin ] && base="/opt/homebrew/bin:$base"

# A fixed prompt, so the recording shows neither the host name nor the temp path.
printf "PROMPT='\$ '\n" > "$t/home/.zshrc"
printf "PS1='\$ '\n" > "$t/home/.bashrc"

cd "$repo" || exit 2
exec env -i \
  HOME="$t/home" USER=new LOGNAME=new SHELL=/bin/zsh TERM="${TERM:-xterm-256color}" LANG=en_US.UTF-8 \
  PATH="$t/tools:$base" PS1='$ ' BASH_SILENCE_DEPRECATION_WARNING=1 \
  MANDRAKE_HOME="$t/home/.mandrake" \
  RUSTUP_HOME="${RUSTUP_HOME:-$real_home/.rustup}" CARGO_HOME="${CARGO_HOME:-$real_home/.cargo}" \
  CARGO_TARGET_DIR="$repo/target" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" \
  "$@"
