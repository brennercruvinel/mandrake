#!/usr/bin/env bash
# Builds Mandrake from this checkout, installs the binary and runs the setup.
#   ./install.sh                       interactive setup
#   ./install.sh --profile <file>      apply a saved profile without asking
# Needs: cargo (rustup.rs), Claude Code, jq for the soul aliases; Go only for fancy's tab tint.
set -euo pipefail
src="$(cd "$(dirname "$0")" && pwd)"
need() { command -v "$1" >/dev/null || { echo "install.sh: $1 not found. $2" >&2; exit 1; }; }
need cargo "Install Rust from https://rustup.rs"
need claude "Install Claude Code first: https://docs.claude.com/claude-code"
command -v jq >/dev/null || echo "install.sh: jq not found; soul aliases in agents.json will be ignored" >&2
case "$(uname -s)" in Darwin|Linux) ;; *) echo "install.sh: only macOS and Linux" >&2; exit 1 ;; esac

cargo install --quiet --locked --path "$src/pkgs/cli" --root "${MANDRAKE_PREFIX:-$HOME/.local}"
bin="${MANDRAKE_PREFIX:-$HOME/.local}/bin/mandrake"
echo "installed $bin"
case ":$PATH:" in *":$(dirname "$bin"):"*) ;; *) echo "note: add $(dirname "$bin") to PATH" ;; esac

if [ "${1:-}" = --profile ]; then
  MANDRAKE_SRC="$src" "$bin" setup --profile "${2:?--profile needs a file}" --yes
else
  MANDRAKE_SRC="$src" "$bin" setup
fi
