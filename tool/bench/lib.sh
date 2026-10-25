# shellcheck shell=bash disable=SC2154 # $orig and $w come from tool/bench/tasks.py
# Sourced before a task's check.sh. A check prints one line per rubric item; the first item is the
# task's success, the others its quality. Variables: $task (the task folder), $w (the work folder
# the session changed), $orig (an untouched copy of the task's repo), $answer (the session's last
# reply).
#   rubric <name> <code>   0 passes, anything else fails
#   same <file>            0 when the file is as in the task's repo
#   only_changed <files>   0 when no other file was added, removed or changed (dot folders apart)
rubric() { if [ "$2" = 0 ]; then echo "pass $1"; else echo "fail $1"; fi; }
same() { cmp -s "$orig/$1" "$w/$1"; echo $?; }
only_changed() {
  local allowed=" $* " f bad=0
  while IFS= read -r f; do
    case "$allowed" in *" $f "*) ;; *) bad=1 ;; esac
  done < <(diff -rq --exclude='.*' --exclude=__pycache__ --exclude=target "$orig" "$w" 2> /dev/null |
    sed -E "s#^Files $orig/([^ ]+) and .*#\1#; s#^Only in $w/?(.*): (.*)#\1/\2#; s#^Only in $orig/?(.*): (.*)#\1/\2#; s#^/##")
  echo "$bad"
}
