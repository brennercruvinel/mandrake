# `claude invoke <soul>`: the shell function setup installs, up to where it starts Claude Code.
# `claude` is a stub here: it prints the command it would run.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=installed
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "a new shell loads the block setup wrote in ~/.bashrc"
Type "source ~/.bashrc; type claude | head -3"
Key Enter
Sleep 1.5
Snap 01-function

Caption "claude invoke rhizo · the example soul"
Type "clear; claude invoke rhizo"
Key Enter
Wait "stub"
Sleep 2
Poster
Snap 02-rhizo

Caption "claude invoke with no soul, and with one that does not exist"
Type "clear; claude invoke; echo exit $?; claude invoke nobody; echo exit $?"
Key Enter
Sleep 2.5
Snap 03-errors

Caption "plain claude: the core prompt, no soul"
Type "clear; claude"
Key Enter
Wait "stub"
Sleep 2
Snap 04-plain
Type "exit"
Key Enter
Sleep 0.3
