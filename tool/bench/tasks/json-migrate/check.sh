# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
[ "$(cd "$w" && python3 app.py 2> /dev/null)" = "db.local:5433 debug=True" ] &&
  python3 -c 'import json,sys; c=json.load(open(sys.argv[1])); sys.exit(0 if c == {"version": 2, "server": {"host": "db.local", "port": 5433}, "flags": {"debug": True}} else 1)' "$w/config.json"
rubric "version 2 and the same output" $?
rubric "FORMAT.md untouched" "$(same FORMAT.md)"
rubric "only config.json and app.py changed" "$(only_changed config.json app.py)"
