# `mandrake doctor`: installed, then with a lost link and the shell block gone from ~/.bashrc.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=installed
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake doctor · right after setup (here MANDRAKE_HOME isolates it: the service is not loaded)"
Type "mandrake doctor"
Key Enter
Wait "claude"
Sleep 2
Snap 01-installed

Caption "a link lost and the shell block deleted from ~/.bashrc"
Type "clear; rm ~/.claude/agents; perl -ni -e 'print unless /mandrake/' ~/.bashrc; mandrake doctor"
Key Enter
Wait "claude"
Sleep 2
Poster
Snap 02-broken

Caption "does it say how to fix it? the setup again"
Type "clear; mandrake setup --yes | tail -4; mandrake doctor | grep -v '^ok'"
Key Enter
Sleep 4
Snap 03-recovered
Type "exit"
Key Enter
Sleep 0.3
