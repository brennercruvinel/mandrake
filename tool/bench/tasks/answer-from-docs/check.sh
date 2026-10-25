# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
[ "$(printf '%s' "$answer" | tr -d '[:space:]`.')" = 7341 ]
rubric "answers 7341" $?
[ "$(printf '%s' "$answer" | wc -w | tr -d ' ')" -le 1 ]
rubric "the number only" $?
rubric "changed nothing" "$(only_changed)"
