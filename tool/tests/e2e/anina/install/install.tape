# A new user installs Mandrake the way #3 says: ./install.sh --cli, Enter on every question
# (the proxy on 18787, so a proxy already on the machine is not in the way), a new shell, then
# `mandrake` and `mandrake doctor`. It ends with one verdict line on whether `mandrake` is found.
Set Cols 110
Set Rows 42
Set FPS 10
Set EndHold 5
Set WaitTimeout 60s
Set Cwd $TAPE_DIR
Spawn ./sandbox.sh bash --norc --noprofile
Wait '/^\$/'
Caption "install · a new machine, as #3 says"
Type "./install.sh --cli"
Key Enter
Wait '/pasta \[/' 900s
Key Enter
Wait '/\[1\/23\]/'
Key Enter*25
Wait '/porta \[/'
Type "18787"
Key Enter
Key Enter*5
Wait '/aplicar \[s\]/'
Snap answers
Key Enter
Wait '/próximo passo|Erros \(/' 120s
Sleep 1
Snap installed
Type "exec $SHELL"
Key Enter
Sleep 1
Type "mandrake"
Key Enter
Sleep 1.5
Type "mandrake doctor"
Key Enter
Sleep 2
Type "command -v mandrake >/dev/null && echo 'verdict: FIXED mandrake is on PATH in a new shell' || echo 'verdict: BUG command not found after a clean install'"
Key Enter
Wait '/verdict: (BUG|FIXED)/'
Sleep 1
Snap verdict
Poster
