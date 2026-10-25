# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
(cd "$w" && python3 -m unittest -q test_fizz) > /dev/null 2>&1
rubric "the new tests pass" $?
# Each wrong version must make the tests fail: a test that cannot fail checks nothing.
caught=0
[ -f "$w/test_fizz.py" ] || caught=-4
for mutant in 's/n % 15 == 0/n % 45 == 0/' 's/"Fizz"$/"Fiz"/' 's/n % 5 == 0:/n % 7 == 0:/' 's/return str(n)/return n/'; do
  m="$(mktemp -d)"; cp "$w/test_fizz.py" "$m/" 2> /dev/null; sed "$mutant" "$orig/fizz.py" > "$m/fizz.py"
  (cd "$m" && python3 -m unittest -q test_fizz) > /dev/null 2>&1 || caught=$((caught + 1)); rm -rf "$m"
done
[ "$caught" = 4 ]
rubric "the tests catch every broken version" $?
rubric "fizz.py untouched" "$(same fizz.py)"
