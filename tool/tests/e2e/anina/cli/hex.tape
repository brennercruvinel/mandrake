# `mandrake audit` then `mandrake hex <id>` at 80x24: one request before and after the cut.
Set Cols 80
Set Rows 24
Set FPS 10
Set WaitTimeout 30
Set EndHold 3
Set Env CLI_SEED=traffic
Spawn bash --rcfile "$CLI_ENV" -i
Wait '/~ \$/'

Caption "mandrake audit · the ids hex needs"
Type "mandrake audit"
Key Enter
Wait "saved"
Sleep 1.5
Snap 01-audit

Caption "mandrake hex <id> · the newest one"
Type "clear; mandrake hex $(mandrake audit | head -1 | cut -f1)"
Key Enter
Wait "q voltar"
Sleep 2
Poster
Snap 02-open

Caption "n · next cut"
Key n
Sleep 1.2
Snap 03-next
Key n
Sleep 1.2

Caption "Tab · guide on the other side"
Key Tab
Sleep 1.2
Snap 04-tab
Key q
Sleep 0.8
Type "exit"
Key Enter
Sleep 0.3
