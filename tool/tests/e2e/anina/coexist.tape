# Mandrake next to other plugins and proxies (tool/tests/e2e/coexist.sh), on a throwaway machine.
# Uses target/debug/mandrake.
Set Cols 120
Set Rows 40
Set FPS 10
Set EndHold 4
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "coexist: another plugin folder, another proxy, another marketplace, another program on the port"
Type "../coexist.sh"
Key Enter
Wait '/coexist: (every check passed|[0-9]+ failed)/' 300s
Sleep 1.5
Snap verdict
Poster
Type "exit"
Key Enter
Sleep 0.3
