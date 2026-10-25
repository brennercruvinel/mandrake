# `mandrake proxy status|run`: the local proxy, run by hand against a fake API, then stopped.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=api
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake proxy status · before it runs"
Type "mandrake proxy status"
Key Enter
Sleep 1.5
Snap 01-status

Caption "mandrake proxy run · in the foreground (here in the background, to keep the prompt)"
Type "clear; mandrake proxy run &"
Key Enter
Sleep 2
Snap 02-run
Type "mandrake proxy status"
Key Enter
Sleep 1.5
Snap 03-status-up

Caption "a request through it, as Claude Code would send one"
Type "send short; mandrake"
Key Enter
Wait "Para começar"
Sleep 2
Poster
Snap 04-one-request

Caption "stop it"
Type "clear; kill $!; sleep 0.5; mandrake proxy status"
Key Enter
Sleep 2
Snap 05-stopped
Type "exit"
Key Enter
Sleep 0.3
