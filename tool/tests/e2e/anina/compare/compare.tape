# mandrake compare: the request Claude Code sends without Mandrake and with it, part by part.
Set Cols 110
Set Rows 34
Set FPS 10
Set EndHold 3
Set Cwd $TAPE_DIR
Set Env PS1='$ '
Set Env BASH_SILENCE_DEPRECATION_WARNING=1
Spawn bash --norc --noprofile
Wait '/^\$/'
Caption "mandrake compare · sem e com o Mandrake, lado a lado, sem gastar nada"
Type "./run.sh"
Key Enter
Wait '/q sair/' 180s
Sleep 1
Snap system
Key Right
Sleep 1
Snap tools
Key Right
Sleep 1
Snap messages
Poster
Key q
Wait '/^\$/'
Type "./run.sh '' --print"
Key Enter
Wait '/tokens \(bytes/' 180s
Sleep 1.5
Snap table
Type "exit"
Key Enter
Sleep 0.3
