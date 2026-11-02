# Mandrake's shell file, sourced from the rc by the block `mandrake setup` writes (bash and zsh).
# `claude|codex|opencode [flags] invoke <soul>` opens a soul in that CLI; any other call is the CLI as it
# is (claude through the proxy when it answers). The work is in `soul`, next to this file: each function
# here is one line with no helper, so a shell that keeps only the functions (Claude Code's Bash
# snapshot drops the ones named _*) still has a working one.
# shellcheck shell=bash
_mandrake_env="${XDG_DATA_HOME:-$HOME/.local/share}/mandrake/shell/env.sh"
[ -n "${MANDRAKE_HOME:-}" ] && _mandrake_env="$MANDRAKE_HOME/data/shell/env.sh"
# shellcheck source=/dev/null
[ -f "$_mandrake_env" ] && . "$_mandrake_env"
export MANDRAKE_SHELL="${_mandrake_env%/env.sh}"

# The folder install.sh puts `mandrake` in, and tui-embed its apps (~/.local/bin by default). macOS
# does not put it on PATH, so a new shell would answer `command not found` right after the install.
if [ -n "${MANDRAKE_BIN_DIR:-}" ] && [ -d "$MANDRAKE_BIN_DIR" ]; then
  case ":$PATH:" in *":$MANDRAKE_BIN_DIR:"*) ;; *) export PATH="$MANDRAKE_BIN_DIR:$PATH" ;; esac
fi

claude() { "${MANDRAKE_SHELL:-$HOME/.local/share/mandrake/shell}/soul" run claude "$@"; }
codex() { "${MANDRAKE_SHELL:-$HOME/.local/share/mandrake/shell}/soul" run codex "$@"; }
opencode() { "${MANDRAKE_SHELL:-$HOME/.local/share/mandrake/shell}/soul" run opencode "$@"; }
