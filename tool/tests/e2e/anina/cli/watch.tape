# `mandrake watch` at 80x24: three requests already through the proxy, three more arriving live.
Set Cols 80
Set Rows 24
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=traffic
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake watch · requests live, without Mandrake on the left, with it on the right"
Type "traffic_later 3; mandrake watch"
Key Enter
Wait "q sair"
Sleep 1.5
Snap 01-open
Sleep 4
Poster
Snap 02-live

Caption "Enter · the newest request in hex: focus starts on the side that has requests"
Key Enter
Sleep 2
Snap 02b-enter-hex
Key q
Sleep 0.8

Caption "Tab · the other side; up/down · a request"
Key Tab
Sleep 1
Snap 03-tab
Key Tab
Sleep 0.5
Key Down
Sleep 0.6
Key Down
Sleep 1
Snap 04-row

Caption "Enter · that request in hex (audited ones, marked with a diamond)"
Key Enter
Sleep 2
Snap 05-hex
Key q
Sleep 0.8
Key q
Sleep 0.8
# q twice covers hex open or not; C-u drops a stray q at the prompt
Key C-u
Type "exit"
Key Enter
Sleep 0.3
