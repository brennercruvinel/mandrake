# `mandrake --help` and the help of two subcommands: what a newcomer reads to find their way.
Set Cols 100
Set Rows 36
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=fresh
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake --help · two groups: to start, and the rest"
Type "mandrake --help"
Key Enter
Wait "Mostra a versão"
Sleep 2
Poster
Snap 01-help

Caption "mandrake link --help"
Type "clear; mandrake link --help"
Key Enter
Wait "Mostra esta ajuda"
Sleep 1.5
Snap 02-link

Caption "mandrake proxy --help"
Type "clear; mandrake proxy --help"
Key Enter
Wait "Mostra esta ajuda"
Sleep 1.5
Snap 03-proxy

Caption "mandrake bench --help · does it say it spends requests?"
Type "clear; mandrake bench --help"
Key Enter
Wait "Mostra esta ajuda"
Sleep 1.5
Snap 04-bench
Type "exit"
Key Enter
Sleep 0.3
