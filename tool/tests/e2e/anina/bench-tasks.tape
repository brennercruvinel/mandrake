# The task bench without spending (#48): every check sound, then the pipeline against a stand-in API.
# Uses the Claude Code on PATH; nothing reaches the API.
Set Cols 120
Set Rows 44
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR/../../../..
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "tasks.py verify: each check fails on the untouched repo and passes on the reference solution"
Type "tool/bench/tasks.py verify"
Key Enter
Wait '/verify: (every task is sound|[0-9]+ task)/' 300s
Sleep 1.5
Snap verify
Type "clear; tool/bench/smoke.sh"
Key Enter
Caption "smoke.sh: three tasks, without and with Mandrake, against a local API; the tasks fail by design"
Wait '/relatório:/' 300s
Sleep 2
Snap smoke
Poster
Type "exit"
Key Enter
Sleep 0.3
