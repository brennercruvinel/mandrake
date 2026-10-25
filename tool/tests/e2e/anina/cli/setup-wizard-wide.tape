# `mandrake setup`, the full-screen wizard, at 140x40: the items step and the summary.
Set Cols 140
Set Rows 40
Set FPS 10
Set WaitTimeout 40
Set EndHold 3
Set Env CLI_SEED=profile
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake setup at 140x40"
Type "mandrake setup"
Key Enter
Wait "Pasta de desenvolvimento"
Sleep 1.5
Snap 01-devroot
Key Tab
Wait "Itens"
Sleep 2
Poster
Snap 02-items
Key Tab
Sleep 0.6
Key Tab
Sleep 0.6
Key Tab
Sleep 0.6
Key Tab
Wait "Resumo"
Sleep 2
Snap 03-summary
Key Esc
Sleep 1.5
Snap 04-quit
Type "exit"
Key Enter
Sleep 0.3
