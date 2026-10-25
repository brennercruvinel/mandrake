# `mandrake audit` then `mandrake hex` at 80x24: the newest request before and after the cut.
Set Cols 80
Set Rows 24
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=traffic
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake audit · the ids hex needs"
Type "mandrake audit"
Key Enter
Wait "economia"
Sleep 1.5
Snap 01-audit

Caption "mandrake hex · with no id, the newest one"
Type "clear; mandrake hex"
Key Enter
Wait "q voltar"
Sleep 2
Poster
Snap 02-open

Caption "n · next cut"
Key n
Sleep 1.2
Snap 03-next
Key n
Sleep 1.2

Caption "Tab · guide on the other side"
Key Tab
Sleep 1.2
Snap 04-tab
Key q
Sleep 0.8
Type "exit"
Key Enter
Sleep 0.3
