# Issue #6, case 1: a copy that fails midway, then rollback.
# cases.sh prints what it runs and ends with "verdict: BUG" (the issue) or "verdict: FIXED".
Set Cols 120
Set Rows 40
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
# macOS bash 3.2 opens with a notice about zsh; it is noise in the recording.
Set Env BASH_SILENCE_DEPRECATION_WARNING=1
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "#6 · 1 · a copy that fails midway, then rollback"
Type "./cases.sh copy-fail"
Key Enter
Wait '/verdict: (BUG|FIXED)/' 90s
Sleep 1.5
Snap copy-fail
Poster
Type "exit"
Key Enter
Sleep 0.3
