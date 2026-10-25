# `mandrake` with no arguments: before anything is installed, then after `setup --yes`.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 60
Set EndHold 3
Set Env CLI_SEED=fresh
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake · a new machine: what does it say first?"
Type "mandrake"
Key Enter
Wait "mandrake setup"
Sleep 1.5
Snap 01-fresh

Caption "mandrake setup --yes · the defaults, no questions"
Type "clear; mandrake setup --yes"
Key Enter
Wait "Erros"
Sleep 1.5
Snap 02-setup-yes

Caption "mandrake · the same command, now installed"
Type "clear; mandrake"
Key Enter
Wait "Para começar"
Sleep 2
Poster
Snap 03-installed
Type "exit"
Key Enter
Sleep 0.3
