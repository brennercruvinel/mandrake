# Issue #15: the gate fails on a 640-line file and a tracked CLAUDE.md, then passes without them.
# gate.sh prints what it runs and ends with "verdict: GATE WORKS" or "verdict: GATE BROKEN".
Set Cols 120
Set Rows 48
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "#15 · the gate fails on a 640-line file and a CLAUDE.md, then passes"
Type "./gate.sh"
Key Enter
Wait '/FAIL  agent files/' 120s
Sleep 1.5
Snap failing
Wait '/verdict: GATE (WORKS|BROKEN)/' 900s
Sleep 1.5
Snap green
Poster
Type "exit"
Key Enter
Sleep 0.3
