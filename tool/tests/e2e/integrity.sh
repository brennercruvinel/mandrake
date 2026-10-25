#!/usr/bin/env bash
# Install, migration and rollback over real folders, one throwaway HOME per case: an entry that is a
# real folder, a file, a link elsewhere, a dangling link or nothing, against a target that is
# missing, empty or full. Each case runs `setup --profile --yes`, then doctor, then rollback, and
# checks that nothing a person had disappears from view and that rollback gives the same tree back.
# Never touches the real home.
#   tool/tests/e2e/integrity.sh [path/to/mandrake]
set -uo pipefail
for v in $(env | sed -n 's/^\(MANDRAKE_[A-Z_]*\)=.*/\1/p'); do [ "$v" = MANDRAKE_SRC ] || unset "$v"; done
unset ANTHROPIC_BASE_URL
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
bin="${1:-$repo/target/debug/mandrake}"
export MANDRAKE_SRC="${MANDRAKE_SRC:-$repo}"
root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
fails=0
case_name=""
check() {
  if [ "$2" = "$3" ]; then echo "ok    $case_name: $1"; else
    echo "FAIL  $case_name: $1: got '$2', want '$3'"
    fails=$((fails + 1))
  fi
}
m() { env HOME="$t/home" MANDRAKE_HOME="$t/mh" CLAUDE_CONFIG_DIR="$t/home/.claude" SHELL=/bin/bash "$bin" "$@"; }
# Every path under a folder with what it is: a link and where it points, or a file and its bytes.
tree_of() {
  (cd "$1" 2> /dev/null && find . -mindepth 1 | LC_ALL=C sort | while read -r p; do
    if [ -L "$p" ]; then echo "$p -> $(readlink "$p")"
    elif [ -d "$p" ]; then echo "$p/"
    else echo "$p $(cksum < "$p" | cut -d' ' -f1)"; fi
  done)
}
# The tree link rollback answers for: every entry but settings.json, which the install step writes
# (its old text goes to the backup) and `link rollback` does not undo.
links_of() { tree_of "$1" | grep -v '^\./settings\.json '; }
# start <name>: a fresh machine with an empty ~/.claude and a dev folder.
start() {
  case_name="$1"
  t="$root/$1"
  mkdir -p "$t/home/.claude" "$t/dev"
  printf 'export KEEP=1\n' > "$t/home/.bashrc"
}
# profile <entry> <action> <target>: a profile with one entry, nothing optional.
profile() {
  cat > "$t/p.toml" << EOF
dev_root = "$t/dev"

[links."$1"]
action = "$2"
target = "$3"
EOF
}
setup() { m setup --profile "$t/p.toml" --yes > "$t/setup.log" 2>&1; }
links_ok() { m doctor 2>&1 | grep -c '^ok   links:'; }
rollback() { m link rollback > "$t/rollback.log" 2>&1; }

start folder-to-missing
mkdir -p "$t/home/.claude/agents"; echo r > "$t/home/.claude/agents/reviewer.md"
profile agents link claude/agents
before="$(links_of "$t/home/.claude")"
setup; check "setup exits 0" "$?" 0
check "the file shows through the link" "$(cat "$t/home/.claude/agents/reviewer.md" 2>&1)" r
check "the target got the file" "$(cat "$t/dev/claude/agents/reviewer.md" 2>&1)" r
check "doctor: links as the profile says" "$(links_ok)" 1
echo n > "$t/home/.claude/agents/new.md"
rollback; check "rollback exits 0" "$?" 0
check "rollback gives the same tree" "$(links_of "$t/home/.claude")" "$before"
check "what was written through the link is kept" "$(cat "$t/mh/state/backups"/*/.seeded/agents/new.md 2>&1)" n

start folder-to-empty
mkdir -p "$t/home/.claude/agents" "$t/dev/claude/agents"; echo r > "$t/home/.claude/agents/reviewer.md"
profile agents link claude/agents
before="$(links_of "$t/home/.claude")"
setup; check "setup exits 0" "$?" 0
check "the file shows through the link" "$(cat "$t/home/.claude/agents/reviewer.md" 2>&1)" r
rollback
check "rollback gives the same tree" "$(links_of "$t/home/.claude")" "$before"
check "the empty target is empty again" "$(tree_of "$t/dev/claude/agents")" ""

start folder-to-full
mkdir -p "$t/home/.claude/agents" "$t/dev/claude/agents"
echo mine > "$t/home/.claude/agents/mine.md"; echo theirs > "$t/dev/claude/agents/theirs.md"
profile agents link claude/agents
before="$(links_of "$t/home/.claude")"
setup; check "setup exits 0" "$?" 0
check "the link shows the target" "$(cat "$t/home/.claude/agents/theirs.md" 2>&1)" theirs
check "the entry is in the backup" "$(cat "$t/mh/state/backups"/*/agents/mine.md 2>&1)" mine
check "the summary says where it went" "$(grep -c 'agents: vai para o backup' "$t/setup.log")" 1
rollback
check "rollback gives the same tree" "$(links_of "$t/home/.claude")" "$before"
check "the target keeps its files" "$(tree_of "$t/dev/claude/agents")" "./theirs.md $(echo theirs | cksum | cut -d' ' -f1)"

start file-to-missing
echo '{"mine":1}' > "$t/home/.claude/settings.json"
profile settings.json link claude/settings.json
setup; check "setup exits 0" "$?" 0
check "settings keeps its keys through the link" "$(jq -r .mine "$t/home/.claude/settings.json" 2>&1)" 1
check "settings is a link" "$([ -L "$t/home/.claude/settings.json" ] && echo y)" y
rollback
check "rollback gives the original settings back" "$(jq -c . "$t/home/.claude/settings.json" 2>&1)" '{"mine":1}'

start link-elsewhere
mkdir -p "$t/old/agents" "$t/dev/claude/agents"; echo o > "$t/old/agents/o.md"; echo d > "$t/dev/claude/agents/d.md"
ln -s "$t/old/agents" "$t/home/.claude/agents"
profile agents link claude/agents
before="$(links_of "$t/home/.claude")"
setup; check "setup exits 0" "$?" 0
check "the link points at the target" "$(readlink "$t/home/.claude/agents")" "$t/dev/claude/agents"
rollback
check "rollback gives the old link back" "$(links_of "$t/home/.claude")" "$before"
check "the old folder is untouched" "$(cat "$t/old/agents/o.md")" o

start dangling-link
ln -s "$t/nowhere" "$t/home/.claude/agents"
profile agents link claude/agents
before="$(links_of "$t/home/.claude")"
setup; check "setup exits 0" "$?" 0
check "the link points at the target" "$(readlink "$t/home/.claude/agents")" "$t/dev/claude/agents"
check "the target exists" "$([ -d "$t/dev/claude/agents" ] && echo y)" y
rollback
check "rollback gives the dangling link back" "$(links_of "$t/home/.claude")" "$before"

start nothing-there
profile agents link claude/agents
setup; check "setup exits 0" "$?" 0
check "the link points at the target" "$(readlink "$t/home/.claude/agents")" "$t/dev/claude/agents"
check "doctor: links as the profile says" "$(links_ok)" 1
rollback
check "rollback leaves nothing" "$(tree_of "$t/home/.claude" | grep -c agents)" 0

start copy-from-missing
mkdir -p "$t/home/.claude/rules"; echo mine > "$t/home/.claude/rules/mine.md"
profile rules copy claude/rules
before="$(links_of "$t/home/.claude")"
setup; code=$?
check "setup refuses a copy from nowhere" "$code" 1
check "and says what is missing" "$(grep -q 'a cópia vem de .*claude/rules, que não existe' "$t/setup.log" && echo y)" y
check "the entry is untouched" "$(links_of "$t/home/.claude")" "$before"

start reinstall
mkdir -p "$t/home/.claude/agents"; echo r > "$t/home/.claude/agents/reviewer.md"
profile agents link claude/agents
before="$(links_of "$t/home/.claude")"
setup; setup
check "the second setup changes no link" "$(grep -c 'links: sem mudança' "$t/setup.log")" 1
check "one journal" "$(find "$t/mh/state/journal" -name '*.json' | wc -l | tr -d ' ')" 1
rollback
check "one rollback gives the same tree" "$(links_of "$t/home/.claude")" "$before"
m link rollback > /dev/null 2>&1
check "a second rollback has nothing to undo" "$?" 1

# Migration: a machine's ~/.claude captured as a profile, and a new machine set up from it.
start migration
a="$t"; mkdir -p "$a/dev/claude/agents" "$a/dev/claude/rules"
echo r > "$a/dev/claude/agents/reviewer.md"; echo x > "$a/dev/claude/rules/x.md"
ln -s "$a/dev/claude/agents" "$a/home/.claude/agents"; ln -s "$a/dev/claude/rules" "$a/home/.claude/rules"
m capture --out "$a/p.toml" > /dev/null 2>&1
check "capture finds both links" "$(grep -c 'action = "link"' "$a/p.toml")" 2
start migration-new-machine
b="$t"; cp -R "$a/dev/claude" "$b/dev/claude"
sed "s#$a/dev#$b/dev#" "$a/p.toml" > "$b/p.toml"
setup; check "setup on the new machine exits 0" "$?" 0
check "agents arrives" "$(cat "$b/home/.claude/agents/reviewer.md" 2>&1)" r
check "rules arrives" "$(cat "$b/home/.claude/rules/x.md" 2>&1)" x
check "doctor: links as the profile says" "$(links_ok)" 1

echo
[ "$fails" -eq 0 ] && echo "integrity: every case passed" || echo "integrity: $fails failed"
exit "$fails"
