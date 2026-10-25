# Every command and screen, recorded with anina

One tape per command or screen, for the visual audit asked in #3. Each tape types into
`env.sh`: a throwaway HOME, `MANDRAKE_HOME` and `CLAUDE_CONFIG_DIR`, a stub `claude` that prints
what it would run, and, for the screens that need traffic, a fake API on a local port
(`fake-upstream.py`). Nothing reaches the real home, the real Claude Code or the network, and no
tape runs the real `mandrake bench`.

| Tape | Seed | What it shows |
| --- | --- | --- |
| `status` | fresh | `mandrake` before and after `setup --yes` |
| `help` | fresh | `--help`, `link --help`, `proxy --help`, `bench --help` |
| `setup-wizard`, `setup-wizard-wide` | profile | the full-screen setup at 80x24 and 140x40 |
| `setup-cli` | profile | `setup --cli`, linking `agents`, defaults for the rest |
| `setup-yes` | profile | `setup --profile f --yes`, twice |
| `capture` | installed | `capture` and `capture --out` |
| `link` | installed | a lost link, then `link plan`, `apply`, `rollback` |
| `proxy` | api | `proxy status`, `proxy run`, one request, stop |
| `watch`, `watch-wide` | traffic | live requests at 80x24 and 140x40 |
| `hex`, `hex-wide` | traffic | `audit`, then `hex <id>` at 80x24 and 160x40 |
| `audit` | traffic | the kept requests, then with their bodies gone |
| `bench-view`, `bench-view-wide` | bench | `bench --view` over `bench --fixtures` against the fake API |
| `doctor` | installed | after setup, then with a lost link and no shell block |
| `invoke` | installed | `claude invoke <soul>` up to the command it would run |

```sh
cargo build --workspace --locked
tool/tests/e2e/anina/cli/record.sh linux            # or macos; all tapes
tool/tests/e2e/anina/cli/record.sh linux target/debug/mandrake watch hex   # some
```

`record.sh <platform>` writes `docs/img/e2e/cli/<platform>/<tape>.svg` and, in
`../logs/cli/<platform>/`, the timeline text (`<tape>.txt`), the text of every Snap
(`<tape>.snaps.txt`) and anina's log. Needs `anina` and `python3` on PATH.

To try a command by hand in the same isolated shell:

```sh
MANDRAKE_BIN=$PWD/target/debug/mandrake MANDRAKE_REPO=$PWD CLI_SEED=traffic \
  bash --rcfile tool/tests/e2e/anina/cli/env.sh -i
```

The findings of the Linux pass are in `docs/audit/audit-cli-2026-10-25-linux.md`.
