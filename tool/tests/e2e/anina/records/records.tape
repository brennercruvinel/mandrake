# Issue #14: the four seed ADRs, the changelog draft from the fragments, and towncrier check both ways.
Set Cols 120
Set Rows 44
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "#14 · records: ADRs, changelog fragments, towncrier check"
Type "./check.sh"
Key Enter
Wait '/verdict: (PASS|FAIL)/' 120s
Sleep 1.5
Snap records
Poster
Type "exit"
Key Enter
Sleep 0.3
