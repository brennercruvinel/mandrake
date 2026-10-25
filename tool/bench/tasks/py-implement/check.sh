# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
cp "$task/hidden/test_hidden.py" "$w/" && (cd "$w" && python3 -m unittest -q test_hidden) > /dev/null 2>&1
rubric "hidden tests pass" $?
rubric "only slug.py changed" "$(only_changed slug.py test_hidden.py)"
! grep -qE '^(import|from) (slugify|unidecode|text_unidecode)' "$w/slug.py"
rubric "no new dependency" $?
