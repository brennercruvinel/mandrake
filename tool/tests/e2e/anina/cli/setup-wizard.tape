# `mandrake setup`, the full-screen wizard, walked to the end with mostly the defaults, at 80x24.
Set Cols 80
Set Rows 24
Set FPS 10
Set WaitTimeout 40
Set EndHold 3
Set Env CLI_SEED=profile
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake setup · step 1, the development folder"
Type "mandrake setup"
Key Enter
Wait "Pasta de desenvolvimento"
Sleep 2
Snap 01-devroot

Caption "step 2 · each entry of ~/.claude: keep, link or copy"
Key Tab
Wait "Itens"
Sleep 1.5
Key Down
Sleep 0.6
Key Down
Sleep 0.6
Snap 02-items

Caption "step 3 · plugins"
Key Tab
Wait "Plugins"
Sleep 1.5
Snap 03-plugins

Caption "step 4 · the local proxy"
Key Tab
Wait "Proxy"
Sleep 1.5
Snap 04-proxy

Caption "step 5 · souls and the invoke skill"
Key Tab
Wait "Souls"
Sleep 1.5
Snap 05-souls

Caption "step 6 · the summary, Enter applies"
Key Tab
Wait "Resumo"
Sleep 2
Poster
Snap 06-summary
Key Enter
Wait "Erros"
Sleep 2
Snap 07-applied
Type "exit"
Key Enter
Sleep 0.3
