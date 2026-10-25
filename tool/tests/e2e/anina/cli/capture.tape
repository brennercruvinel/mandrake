# `mandrake capture`: this machine's ~/.claude as a profile, to take to another machine.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=installed
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake capture · prints the profile"
Type "mandrake capture"
Key Enter
Wait "invoke"
Sleep 2
Poster
Snap 01-stdout

Caption "mandrake capture --out my-profile.toml"
Type "clear; mandrake capture --out my-profile.toml; ls -l my-profile.toml"
Key Enter
Sleep 2
Snap 02-out
Type "exit"
Key Enter
Sleep 0.3
