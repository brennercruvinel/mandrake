# `mandrake bench --view` at 120x36 over a report from `bench --fixtures` (fake API).
Set Cols 120
Set Rows 36
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=bench
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake bench --view at 120x36"
Type "mandrake bench --view"
Key Enter
Wait "q sair"
Sleep 2.5
Poster
Snap 01-view
Key q
Sleep 0.8
Type "exit"
Key Enter
Sleep 0.3
