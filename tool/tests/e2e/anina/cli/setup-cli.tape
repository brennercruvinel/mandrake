# `mandrake setup --cli`: the questions one per line. Links `agents`, takes the default for the rest.
Set Cols 100
Set Rows 30
Set FPS 10
Set WaitTimeout 40
Set EndHold 3
Set Env CLI_SEED=profile
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake setup --cli · step 1, the development folder"
Type "mandrake setup --cli"
Key Enter
Wait "pasta ["
Sleep 1.2
Snap 01-devroot
Key Enter

Caption "step 2 · 23 entries of ~/.claude, one question each"
Wait "1/23"
Key Enter
Sleep 0.3
Key Enter
Sleep 0.3
Key Enter
Wait "4/23"
Sleep 1
Snap 02-items
Caption "agents: l links it to a folder under the development folder"
Type "l" 0.2
Key Enter
Wait "destino"
Sleep 1
Snap 03-link
Key Enter

Caption "the other 19 entries, plugins, proxy and prompts: Enter takes the default each time"
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Key Enter
Sleep 0.15
Wait "aplicar"
Sleep 1.5
Poster
Snap 04-summary

Caption "aplicar [s] · Enter applies"
Key Enter
Wait "Erros"
Sleep 2
Snap 05-applied
Caption "where did reviewer.md, the agent that was in ~/.claude/agents, go?"
Type "clear; ls -l ~/.claude; ls -A ~/.claude/agents; find ~/../mh/state/backups -type f"
Key Enter
Sleep 1.5
Snap 06-result
Type "exit"
Key Enter
Sleep 0.3
