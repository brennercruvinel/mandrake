# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
# The visible and the hidden tests pass; the test file and nothing else but stats.py changed.
cp "$task/hidden/test_hidden.py" "$w/" && (cd "$w" && python3 -m unittest -q test_stats test_hidden) > /dev/null 2>&1
rubric "tests pass" $?
rubric "test file untouched" "$(same test_stats.py)"
rubric "only stats.py changed" "$(only_changed stats.py test_hidden.py)"
