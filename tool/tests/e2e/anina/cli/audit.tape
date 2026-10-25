# `mandrake audit`: the requests whose bodies were kept, and what happens once the bodies are gone.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=traffic
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake audit · three requests kept, newest first"
Type "mandrake audit"
Key Enter
Wait "saved"
Sleep 2
Poster
Snap 01-list

Caption "mandrake audit --last 1"
Type "clear; mandrake audit --last 1"
Key Enter
Sleep 1.5
Snap 02-last

Caption "the kept bodies gone (deleted, or past audit_keep_days): does the list still offer them?"
Type 'clear; rm -rf ~/../mh/state/audit; mandrake audit; mandrake hex $(mandrake audit | head -1 | cut -f1)'
Key Enter
Wait "audit"
Sleep 2.5
Snap 03-bodies-gone
Type "exit"
Key Enter
Sleep 0.3
