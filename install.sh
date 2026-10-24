#!/usr/bin/env bash
# Builds Mandrake from this checkout, installs the binary and runs the setup.
#   ./install.sh                       interactive setup
#   ./install.sh --cli                 setup as plain questions, one per line
#   ./install.sh --profile <file>      apply a saved profile without asking
#   ./install.sh --cli --from <file>   plain questions, prefilled from a saved profile
# Needs: cargo (rustup.rs), Claude Code, jq for the soul aliases; Go only for fancy's tab tint.
set -euo pipefail
src="$(cd "$(dirname "$0")" && pwd)"
need() { command -v "$1" >/dev/null || { echo "install.sh: $1 not found. $2" >&2; exit 1; }; }
need cargo "Install Rust from https://rustup.rs"
need claude "Install Claude Code first: https://docs.claude.com/claude-code"
command -v jq >/dev/null || echo "install.sh: jq not found; soul aliases in agents.json will be ignored" >&2
case "$(uname -s)" in Darwin|Linux) ;; *) echo "install.sh: only macOS and Linux" >&2; exit 1 ;; esac

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$src/target}"
cargo install --quiet --locked --path "$src/pkgs/cli" --root "${MANDRAKE_PREFIX:-$HOME/.local}"
bin="${MANDRAKE_PREFIX:-$HOME/.local}/bin/mandrake"
echo "installed $bin"
case ":$PATH:" in *":$(dirname "$bin"):"*) ;; *) echo "note: add $(dirname "$bin") to PATH" ;; esac

case "${1:-}" in
  --profile) MANDRAKE_SRC="$src" "$bin" setup --profile "${2:?--profile needs a file}" --yes ;;
  --cli) shift; [ "${1:-}" = --from ] && set -- --profile "${2:?--from needs a file}"
         MANDRAKE_SRC="$src" "$bin" setup --cli "$@" ;;
  *) MANDRAKE_SRC="$src" "$bin" setup ;;
esac
