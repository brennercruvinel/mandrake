#!/usr/bin/env bash
# Keeps labels, issues and pull requests in the shape CONTRIBUTING.md describes, with gh.
#
#   tool/board.sh labels    writes .github/labels.tsv to GitHub (creates, recolors, redescribes)
#   tool/board.sh sync      puts every issue and pull request on the board: Done when closed or
#                           merged; with no status, an open issue gets Task and an open pull
#                           request In progress; any other open status is kept
#   tool/board.sh check     lists what is off the standard and exits 1 when there is anything
#
# check looks for: not exactly one type label, an unknown label, an item off the board, a pull
# request with no "Closes #", "Part of #" or "No issue: <why>", "Closes #a #b" (GitHub closes only
# #a), and an issue that says "Part of #N" without being a sub-issue of #N.
#
# Env: BOARD_REPO (default: the repo of this checkout), BOARD_OWNER and BOARD_NUMBER (the project).
set -uo pipefail
cd "$(dirname "$0")/.." || exit 2

OWNER="${BOARD_OWNER:-brennercruvinel}"
NUMBER="${BOARD_NUMBER:-8}"
REPO="${BOARD_REPO:-$(gh repo view --json nameWithOwner --jq .nameWithOwner)}" || exit 2
LABELS=.github/labels.tsv

labels_of() { # $1 = group, or "" for all
  awk -F '\t' -v g="$1" '!/^#/ && NF >= 4 && (g == "" || $1 == g) { print $2 }' "$LABELS"
}

# Every issue and pull request, one line each: kind, number, state, url, labels (comma-separated).
items() {
  gh issue list -R "$REPO" --state all --limit 1000 --json number,state,url,labels \
    --jq '.[] | ["issue", .number, .state, .url, ([.labels[].name] | join(","))] | @tsv'
  gh pr list -R "$REPO" --state all --limit 1000 --json number,state,url,labels \
    --jq '.[] | ["pr", .number, .state, .url, ([.labels[].name] | join(","))] | @tsv'
}

# Board items: url, item id, status.
board() {
  gh project item-list "$NUMBER" --owner "$OWNER" --limit 1000 --format json \
    --jq '.items[] | select(.content.url) | [.content.url, .id, (.status // "")] | @tsv'
}

cmd_labels() {
  awk -F '\t' '!/^#/ && NF >= 4 { print $2 "\t" $3 "\t" $4 }' "$LABELS" |
    while IFS=$'\t' read -r name color desc; do
      gh label create "$name" -R "$REPO" --color "$color" --description "$desc" --force > /dev/null &&
        echo "ok    label $name"
    done
}

cmd_sync() {
  local project field done_id task_id doing_id on_board kind n state url labels line id status
  project="$(gh project view "$NUMBER" --owner "$OWNER" --format json --jq .id)" || exit 2
  read -r field done_id task_id doing_id < <(gh project field-list "$NUMBER" --owner "$OWNER" --format json \
    --jq '.fields[] | select(.name == "Status") | [.id] + ([("Done", "Task", "In progress") as $s |
      .options[] | select(.name == $s) | .id]) | @tsv')
  name() { [ "$1" = "$done_id" ] && echo Done || { [ "$1" = "$doing_id" ] && echo "In progress" || echo Task; }; }
  on_board="$(board)"
  while IFS=$'\t' read -r kind n state url labels; do
    line="$(grep -F "$url"$'\t' <<< "$on_board")"
    if [ -z "$line" ]; then
      id="$(gh project item-add "$NUMBER" --owner "$OWNER" --url "$url" --format json --jq .id)" || continue
      status=""
      echo "added $kind #$n"
    else
      IFS=$'\t' read -r _ id status <<< "$line"
    fi
    local want=""
    case "$state" in
      CLOSED | MERGED) [ "$status" = Done ] || want="$done_id" ;;
      *) [ -n "$status" ] || want="$([ "$kind" = pr ] && echo "$doing_id" || echo "$task_id")" ;;
    esac
    if [ -n "$want" ]; then
      gh project item-edit --id "$id" --project-id "$project" --field-id "$field" \
        --single-select-option-id "$want" > /dev/null && echo "ok    $kind #$n status $(name "$want")"
    fi
  done < <(items)
}

cmd_check() {
  local types known on_board problems=0 kind n state url labels count l body parent all
  types="$(labels_of type)"
  known="$(labels_of "")"
  on_board="$(board | cut -f1)"
  say() { echo "FAIL  $1"; problems=$((problems + 1)); }
  while IFS=$'\t' read -r kind n state url labels; do
    count=0
    IFS=',' read -ra all <<< "$labels"
    for l in "${all[@]}"; do
      grep -qxF "$l" <<< "$types" && count=$((count + 1))
      grep -qxF "$l" <<< "$known" || say "$kind #$n: label '$l' is not in $LABELS"
    done
    [ "$count" -eq 1 ] || say "$kind #$n: $count type labels (one of: $(tr '\n' ' ' <<< "$types"))"
    grep -qxF "$url" <<< "$on_board" || say "$kind #$n: not on the board (tool/board.sh sync)"
    # Inline code is a quote ("`Closes #25 #30`" as an example), not a keyword.
    body="$(gh "$kind" view "$n" -R "$REPO" --json body --jq .body | sed 's/`[^`]*`//g')"
    if grep -qiE '(close[sd]?|fix(e[sd])?|resolve[sd]?) #[0-9]+[ ,]+#[0-9]+' <<< "$body"; then
      say "$kind #$n: 'Closes #a #b' closes only #a; write 'Closes #a, closes #b'"
    fi
    if [ "$kind" = pr ] && ! grep -qiE '(close[sd]?|fix(e[sd])?|resolve[sd]?|part of) #[0-9]+|^no issue: ' <<< "$body"; then
      say "pr #$n: no 'Closes #<issue>', 'Part of #<epic>' or 'No issue: <why>'"
    fi
    if [ "$kind" = issue ]; then
      parent="$(sed -nE 's/^Part of #([0-9]+).*/\1/p' <<< "$body" | head -1)"
      if [ -n "$parent" ]; then
        [ "$(gh api "repos/$REPO/issues/$n/parent" --jq .number 2> /dev/null)" = "$parent" ] ||
          say "issue #$n: says 'Part of #$parent' but is not its sub-issue"
      fi
    fi
  done < <(items)
  if [ "$problems" -eq 0 ]; then echo "board: everything follows the standard"; else echo "board: $problems to fix"; exit 1; fi
}

case "${1:-}" in
  labels) cmd_labels ;;
  sync) cmd_sync ;;
  check) cmd_check ;;
  *) sed -n '2,14p' "$0"; exit 2 ;;
esac
