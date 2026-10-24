#!/usr/bin/env bash
# Puts back the ~/.claude links from a saved map ("path -> target" per line) when Claude Code has
# replaced them by real folders, keeping what it wrote in them. Nothing is deleted:
#   - a missing path gets its link;
#   - a real folder or file moves to ~/.claude-relink-<stamp>/ (same disk, instant), the link goes in,
#     then its files merge into the target: new files are copied, a .jsonl present on both sides gets
#     the new lines appended, any other file present on both sides keeps the target's copy;
#   - a link to somewhere else is left alone and reported.
#   relink.sh <links.txt> [--dry-run]
set -uo pipefail
map="${1:?usage: relink.sh <links.txt> [--dry-run]}"; dry="${2:-}"
claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
stage="$HOME/.claude-relink-$(date +%Y%m%d-%H%M%S)"
say() { echo "$@"; }
merge() { # staged path -> target, same shape
  local from="$1" to="$2" f rel
  if [ -f "$from" ]; then
    case "$from" in *.jsonl) [ -f "$to" ] && cat "$from" >> "$to" || cp -p "$from" "$to"; echo "     anexado $(wc -l <"$from" | tr -d ' ') linhas" ;;
      *) [ -e "$to" ] || cp -p "$from" "$to" ;; esac
    return
  fi
  local new=0 app=0 kept=0
  while IFS= read -r -d '' f; do
    rel="${f#"$from"/}"
    if [ ! -e "$to/$rel" ]; then mkdir -p "$(dirname "$to/$rel")"; cp -p "$f" "$to/$rel"; new=$((new + 1))
    elif [[ "$rel" == *.jsonl ]] && ! cmp -s "$f" "$to/$rel"; then cat "$f" >> "$to/$rel"; app=$((app + 1))
    else kept=$((kept + 1)); fi
  done < <(find "$from" -type f -print0)
  echo "     $new arquivos novos copiados, $app transcripts com linhas anexadas, $kept já existiam (ficou a cópia do destino)"
}
while IFS= read -r line; do
  [ -n "$line" ] || continue
  path="${line%% -> *}"; target="${line#* -> }"; at="$claude_dir/$path"
  if [ -L "$at" ]; then
    [ "$(readlink "$at")" = "$target" ] && say "ok    $path já é link" || say "AVISO $path aponta para $(readlink "$at"), deixado como está"
    continue
  fi
  [ -e "$target" ] || { say "ERRO  $path: destino $target não existe, pulado"; continue; }
  if [ ! -e "$at" ]; then
    say "link  $path -> $target"
    [ -n "$dry" ] || { mkdir -p "$(dirname "$at")"; ln -s "$target" "$at"; }
    continue
  fi
  say "troca $path ($( [ -d "$at" ] && echo "pasta com $(find "$at" -type f | wc -l | tr -d ' ') arquivos" || echo arquivo)) -> link, conteúdo junto ao destino"
  [ -n "$dry" ] && continue
  mkdir -p "$stage/$(dirname "$path")"
  mv "$at" "$stage/$path" && ln -s "$target" "$at" && merge "$stage/$path" "$target"
done < "$map"
[ -n "$dry" ] || [ ! -d "$stage" ] || say "o que estava nas pastas novas ficou guardado em $stage"
