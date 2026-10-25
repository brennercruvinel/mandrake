# `mandrake bench --view` at 80x24 over a report from `bench --fixtures` (fake API, no real request).
Set Cols 80
Set Rows 24
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=bench
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake bench --view · the last report"
Type "mandrake bench --view"
Key Enter
Wait "q sair"
Sleep 2
Poster
Snap 01-view

Caption "up/down · a fixture"
Key Down
Sleep 1
Key Down
Sleep 1
Snap 02-row
Key q
Sleep 0.8

Caption "bench --view on a machine that never measured"
Type 'clear; rm -rf ~/../mh/state/bench; mandrake bench --view'
Key Enter
Sleep 2
Snap 03-none
Type "exit"
Key Enter
Sleep 0.3
