# Issue #9: after a build, an e2e run and an editor, git status shows nothing new.
# hygiene.sh prints what it runs and ends with "verdict: CLEAN" or "verdict: DIRTY".
Set Cols 120
Set Rows 40
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "#9 · build, e2e and editor files leave git status clean"
Type "./hygiene.sh"
Key Enter
Wait '/verdict: (CLEAN|DIRTY)/' 300s
Sleep 1.5
Snap hygiene
Poster
Type "exit"
Key Enter
Sleep 0.3
