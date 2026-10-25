# The four cases of #6, recorded with anina

`cases.sh` runs each case of issue #6 against a real `mandrake` binary, in a throwaway HOME with a
stub `claude`, and ends with one line: `verdict: BUG …` (exit 1, what the issue describes) or
`verdict: FIXED …` (exit 0, what it asks for). The same script runs before and after the fix; only
the verdict changes.

| Case | What it does | Fixed when |
| --- | --- | --- |
| `copy-fail` | `link apply` with a copy whose source is missing, then `link rollback` | `original.txt` is back in `~/.claude/rules` |
| `same-second` | two `link apply` in the same second (rules → a, then → b), then two rollbacks | two journals, and the original folder comes back |
| `doctor-copy` | `setup --profile --yes` with a copy, `doctor`, then setup again | doctor passes and the second setup backs nothing up |
| `eof` | `setup --cli < /dev/null` | non-zero exit and nothing written |

Each `<case>.tape` types `./cases.sh <case>` in a bash prompt and waits for the verdict, so the
recording shows the commands and what they found.

```sh
cargo build --workspace --locked
tool/tests/e2e/anina/cases.sh all                 # the four, plain text
tool/tests/e2e/anina/record.sh after              # with anina on PATH: SVGs and logs
```

`record.sh <label>` writes `docs/img/e2e/<label>/<case>.svg` and `logs/<label>/<case>.log`. The
`before` set was recorded on Linux (x86_64, Ubuntu) against `main` at `b815ea6`, before the fix.
