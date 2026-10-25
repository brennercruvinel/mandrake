# `mandrake watch` at 140x40: the same screen as watch.tape with room to breathe.
Set Cols 140
Set Rows 40
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=traffic
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake watch at 140x40 · requests arriving live"
Type "traffic_later 3; mandrake watch"
Key Enter
Wait "q sair"
Sleep 5
Poster
Snap 01-live
Key Down
Sleep 0.6
Key Enter
Sleep 2
Snap 02-hex
Key q
Sleep 0.8
Key q
Sleep 0.8
# q twice covers hex open or not; C-u drops a stray q at the prompt
Key C-u
Type "exit"
Key Enter
Sleep 0.3
