# #33: a MANDRAKE_* left over from an older shell loses to the installed value, and doctor names it.
# `claude` is a stub here: it prints the command it would run.
Set Cols 110
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=installed
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "a shell that still exports an old souls folder"
Type "export MANDRAKE_SOULS=/old/souls; mandrake doctor 2>&1 | grep ambiente"
Key Enter
Wait "ambiente"
Sleep 1.5
Snap 01-doctor

Caption "a new shell loads the installed values: claude invoke uses the installed souls"
Type "clear; new_shell claude invoke rhizo 2>&1 | cut -c1-108"
Key Enter
Wait "stub"
Sleep 2
Poster
Snap 02-new-shell

Type "exit"
Key Enter
Sleep 0.3
