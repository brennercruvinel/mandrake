# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
p="$(mktemp -d)"; mkdir -p "$p/my src"; echo a > "$p/my src/one file.txt"; echo b > "$p/my src/two.txt"
(cd "$p" && bash "$w/backup.sh" "my src" "my dst") > /dev/null 2>&1 &&
  [ "$(cat "$p/my dst/one file.txt" 2> /dev/null)" = a ] && [ "$(cat "$p/my dst/two.txt" 2> /dev/null)" = b ]
rubric "copies files with spaces" $?
rubric "only backup.sh changed" "$(only_changed backup.sh)"
(cd "$p" && bash "$w/backup.sh" "my src" "my dst") > /dev/null 2>&1
rubric "a second run still works" $?
rm -rf "$p"
