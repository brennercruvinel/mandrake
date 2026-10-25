# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
(cd "$w" && node test.js) > /dev/null 2>&1 && ! grep -rq getUsr "$w"/*.js
rubric "renamed and the test passes" $?
(cd "$w" && node -e "import('./users.js').then(m => process.exit(typeof m.getUser === 'function' ? 0 : 1))")
rubric "users.js exports getUser" $?
rubric "only users.js and greet.js changed" "$(only_changed users.js greet.js)"
