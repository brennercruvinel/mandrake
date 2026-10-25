# `mandrake setup --profile f --yes`: a saved profile applied with no questions, then once more.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 40
Set EndHold 3
Set Env CLI_SEED=profile
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "a profile from another machine: two links"
Type "cat mandrake-profile.toml | grep -v '^$' | head -12"
Key Enter
Sleep 1.5
Snap 01-profile

Caption "mandrake setup --profile mandrake-profile.toml --yes"
Type "clear; mandrake setup --profile mandrake-profile.toml --yes"
Key Enter
Wait "Erros"
Sleep 2
Poster
Snap 02-applied

Caption "the same command again: nothing should change"
Type "clear; mandrake setup --profile mandrake-profile.toml --yes"
Key Enter
Wait "Erros"
Sleep 2
Snap 03-again
Type "clear; ls -l ~/.claude"
Key Enter
Sleep 1.5
Snap 04-result
Type "exit"
Key Enter
Sleep 0.3
