# `mandrake link plan|apply|rollback`, starting from a lost link: the recovery path.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=installed
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "installed: agents and commands are links"
Type "ls -l ~/.claude"
Key Enter
Sleep 1.5
Snap 01-installed

Caption "a link gets lost (another tool, a reinstall of Claude Code...)"
Type "clear; rm ~/.claude/commands; mandrake link plan"
Key Enter
Sleep 2
Snap 02-plan

Caption "mandrake link apply · puts it back"
Type "clear; mandrake link apply; ls -l ~/.claude"
Key Enter
Sleep 2
Poster
Snap 03-apply

Caption "mandrake link rollback · undoes the last apply"
Type "clear; mandrake link rollback; ls -l ~/.claude"
Key Enter
Sleep 2
Snap 04-rollback
Type "exit"
Key Enter
Sleep 0.3
