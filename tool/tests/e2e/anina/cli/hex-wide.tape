# `mandrake hex <id>` at 160x40: is before and after side by side when there is room?
Set Cols 160
Set Rows 40
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=traffic
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake hex <id> at 160x40"
Type "mandrake hex $(mandrake audit | head -1 | cut -f1)"
Key Enter
Wait "q voltar"
Sleep 2
Poster
Snap 01-open
Key n
Sleep 1.2
Key n
Sleep 1.2
Snap 02-next
Key q
Sleep 0.8
Type "exit"
Key Enter
Sleep 0.3
