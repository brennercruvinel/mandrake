# Install, rollback, reinstall and migration over real folders (tool/tests/e2e/integrity.sh): one
# throwaway HOME per case, each line says what was checked. Uses target/debug/mandrake.
Set Cols 120
Set Rows 48
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "integrity: a folder, a file, a link elsewhere, a dangling link or nothing, against a missing, empty or full target"
Type "../integrity.sh"
Key Enter
Wait '/integrity: (every case passed|[0-9]+ failed)/' 300s
Sleep 1.5
Snap verdict
Poster
Type "exit"
Key Enter
Sleep 0.3
