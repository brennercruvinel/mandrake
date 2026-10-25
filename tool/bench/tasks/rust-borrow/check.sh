# shellcheck shell=bash disable=SC2154,SC2319 # $task, $w, $orig, $answer come from tasks.py; $? after a test is the rubric code
(cd "$w" && CARGO_TARGET_DIR="$w/target" cargo test --quiet) > /dev/null 2>&1
rubric "cargo test passes" $?
grep -q 'assert_eq!(v, \["ab", "abc", "AB", "ABC"\]);' "$w/src/lib.rs"
rubric "the test still asserts the same" $?
rubric "only src/lib.rs changed" "$(only_changed src/lib.rs Cargo.lock)"
