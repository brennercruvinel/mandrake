# README demo: `mandrake compare`, slowly, through every part of the request.
Set Cols 120
Set Rows 36
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Set Env BASH_SILENCE_DEPRECATION_WARNING=1
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "mandrake compare · one prompt through Claude Code, without and with Mandrake; answered locally, nothing spent"
Type "../tests/e2e/anina/compare/run.sh \"$MANDRAKE_BIN\""
Key Enter
Wait '/q sair/' 180s

Caption "1 · the system prompt: Claude Code's default on the left, Mandrake's short core on the right"
Sleep 4
Snap 01-system
Key PageDown
Sleep 2.5
Key PageDown
Sleep 2.5
Key PageDown
Sleep 2.5

Caption "2 · the tools: every description is cut; the schemas stay"
Key Right
Sleep 4
Poster
Snap 02-tools
Key PageDown
Sleep 2.5
Key PageDown
Sleep 2.5
Key PageDown
Sleep 2.5

Caption "3 · the messages: the reminders around your prompt go, your prompt stays"
Key Right
Sleep 4
Snap 03-messages
Key PageDown
Sleep 2.5
Key PageDown
Sleep 2.5

Caption "the whole request, part by part"
Key q
Wait '/^\$/'
Type "../tests/e2e/anina/compare/run.sh \"$MANDRAKE_BIN\" --print"
Key Enter
Wait '/tokens \(bytes/' 180s
Sleep 4
Snap 04-table
Type "exit"
Key Enter
Sleep 0.3
