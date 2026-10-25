# README demo: `mandrake setup`, the full-screen wizard, walked step by step on a small ~/.claude.
Set Cols 120
Set Rows 34
Set FPS 10
Set WaitTimeout 40
Set EndHold 4
Set Env CLI_SEED=profile
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake setup · one screen per step; Tab goes on, Esc leaves without changing anything"
Type "mandrake setup"
Key Enter
Wait "Pasta de desenvolvimento"
Sleep 3
Snap 01-devroot

Caption "step 2 · each entry of ~/.claude: keep it, link it to your folder, or copy it"
Key Tab
Wait "Itens"
Sleep 2.5
Key Down
Sleep 1
Key Down
Sleep 1
Key Down
Sleep 2
Snap 02-items

Caption "step 3 · plugins: prompt-trim always, fancy and tui-embed if you want them"
Key Tab
Wait "Plugins"
Sleep 3.5
Snap 03-plugins

Caption "step 4 · the local proxy that trims what the plugin cannot reach, and measures"
Key Tab
Wait "Proxy"
Sleep 3.5
Snap 04-proxy

Caption "step 5 · the core prompt, souls and the invoke skill"
Key Tab
Wait "Souls"
Sleep 3.5
Snap 05-souls

Caption "step 6 · the summary; Enter applies, and whatever was in the way goes to a backup"
Key Tab
Wait "Resumo"
Sleep 4
Poster
Snap 06-summary
Key Enter
Sleep 4
Snap 07-applied
Key C-u
Type "exit"
Key Enter
Sleep 0.3
