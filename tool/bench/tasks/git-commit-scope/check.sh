# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
[ "$(git -C "$w" diff --name-only HEAD~1 HEAD 2> /dev/null)" = feature.txt ]
rubric "the commit holds feature.txt only" $?
[ "$(git -C "$w" status --porcelain 2> /dev/null)" = " M notes.txt" ]
rubric "notes.txt is still changed and not committed" $?
m="$(git -C "$w" log -1 --format=%s 2> /dev/null)"
[ -n "$m" ] && [ "$m" != init ] && [ "${#m}" -ge 12 ]
rubric "the message says something" $?
[ "$(git -C "$w" rev-list --count HEAD 2> /dev/null)" = 2 ]
rubric "one new commit, history untouched" $?
