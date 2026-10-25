# prompt-trim's invariants on the real request (tool/tests/e2e/invariants.sh), through mandrake compare.
# Uses target/debug/mandrake and the Claude Code on PATH; nothing reaches the API.
Set Cols 120
Set Rows 34
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "invariants: the same prompt through Claude Code without and with Mandrake, against a local API; nothing is spent"
Type "../invariants.sh"
Key Enter
Wait '/invariants: (every check passed|[0-9]+ failed)/' 300s
Sleep 1.5
Snap verdict
Poster
Type "exit"
Key Enter
Sleep 0.3
