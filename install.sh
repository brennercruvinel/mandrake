#!/usr/bin/env bash
# Builds Mandrake from this checkout, installs the binary and runs the setup.
#   ./install.sh                       interactive setup
#   ./install.sh --cli                 setup as plain questions, one per line
#   ./install.sh --profile <file>      apply a saved profile without asking
#   ./install.sh --cli --from <file>   plain questions, prefilled from a saved profile
# Needs: cargo (rustup.rs), Claude Code, jq for the soul aliases; Go only for fancy's tab tint.
set -euo pipefail
src="$(cd "$(dirname "$0")" && pwd)"
need() { command -v "$1" >/dev/null || { echo "install.sh: $1 não encontrado. $2" >&2; exit 1; }; }
need cargo "Instale o Rust: https://rustup.rs"
need claude "Instale antes o Claude Code: https://docs.claude.com/claude-code"
command -v jq >/dev/null || echo "install.sh: jq não encontrado; os apelidos das souls em agents.json serão ignorados" >&2
case "$(uname -s)" in Darwin|Linux) ;; *) echo "install.sh: só macOS e Linux" >&2; exit 1 ;; esac

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$src/target}"
cargo install --quiet --locked --force --path "$src/rust/cli" --root "${MANDRAKE_PREFIX:-$HOME/.local}"
bin="${MANDRAKE_PREFIX:-$HOME/.local}/bin/mandrake"
echo "instalado: $bin"
case ":$PATH:" in *":$(dirname "$bin"):"*) ;; *) echo "nota: $(dirname "$bin") ainda não está no PATH; o bloco que o setup escreve põe ele lá num shell novo" ;; esac

case "${1:-}" in
  --profile) MANDRAKE_SRC="$src" "$bin" setup --profile "${2:?--profile needs a file}" --yes ;;
  --cli) shift; [ "${1:-}" = --from ] && set -- --profile "${2:?--from needs a file}"
         MANDRAKE_SRC="$src" "$bin" setup --cli "$@" ;;
  *) MANDRAKE_SRC="$src" "$bin" setup ;;
esac
