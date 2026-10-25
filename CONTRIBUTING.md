# Contributing

Keep it simple. Mandrake does three things (trim, measure, rebuild a `~/.claude`), and a change earns its place by making one of them better for someone using it.

## The flow

```mermaid
flowchart LR
  I[issue] --> B[branch from main] --> C[change] --> G[tool/fullcheck.sh] --> N[fragment in docs/news] --> P[pull request with POCS] --> R[review] --> M[maintainer merges]
```

1. Open or pick an issue; an epic groups several.
2. `git switch -c <you>/<short-name> origin/main`, change what the issue asks, nothing else.
3. `tool/fullcheck.sh` until it says `everything passed`.
4. One line in `docs/news/<issue>.<type>.md`, the way it should read in the changelog.
5. Open the pull request from the template: `Closes #<issue>`, a `[POCS]` line, the checks you ran, and its labels.

## The rules

- **Start from an issue.** One change is one issue; several independent items are an epic with one sub-issue each. The pull request starts with `Closes #<sub-issue>. Part of #<epic>.` and follows the template; with more than one issue, one keyword per number (`Closes #25, closes #30`), since GitHub closes only the first of `Closes #25 #30`.
- **No agent instruction files.** No `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.cursorrules` or the like ([ADR-0004](docs/adr/workflows/0004-no-agent-instruction-files.md)). The code, `docs/`, the ADRs and the changelog are the context, for people and agents alike.
- **Code files stay under 639 lines.** Data formats (JSON, JSONL, TOML catalogs, lockfiles, SVG, logs) are exempt.
- **Run the gate before pushing:** `tool/fullcheck.sh`. There is no CI: this script is the gate, run on Linux and macOS and on the Claude Code versions in [setup](docs/guide/setup.md#claude-code-versions).
- **Add a news fragment** in `docs/news/<issue>.<type>.md`: one line, written as the changelog entry, with a type of `added`, `changed`, `removed`, `fixed` or `security`; `+<name>.<type>.md` when there is no issue. Never edit `docs/CHANGELOG` by hand. A change nobody using Mandrake would notice takes the `skip-news` label instead.
- **Write an ADR** in `docs/adr/` when a choice has trade-offs someone will ask about ([how](docs/adr/README.md)). Architecture lives in [`docs/ATLAS.toml`](docs/ATLAS.toml); the names and abbreviations in [`docs/TERMS.md`](docs/TERMS.md).
- **Show it working.** A `[POCS]` line in the pull request says where the change can be seen; test evidence from a terminal goes as anina recordings, embedded one per line. Record with Mandrake's theme, `cfgs/anina/mandrake.json` (the recording scripts pass it; by hand, `anina record <tape> --theme cfgs/anina/mandrake.json`), and type no `$VARIABLE` in a tape: anina expands it, and the recording would show a path of your machine.
- **Sign your commits**, in plain English. The maintainer merges, after review.

## Labels and the board

Each issue and pull request takes **one type**, the word its commit prefix maps to, and **every area** it touches:

| Commit prefix | Type label |
| - | - |
| `fix` | `bug` |
| `feat` | `enhancement` |
| `docs` | `documentation` |
| `test` | `test` |
| `refactor` | `refactor` |
| `chore` | `ci` |

Areas, by path: `cli`, `tui`, `proxy`, `meter`, `layout`, `plugins`, `shell`; each label's description says which folders it covers. An epic also takes `epic`, and a report from testing `feedback` (their templates add it). A label that does not exist yet is created by the maintainer, in the same pattern. Every issue and pull request is on the [board](https://github.com/users/brennercruvinel/projects/8), and a sub-issue is attached to its epic.

## The records, checked

The ADRs in force, the changelog drafted from the fragments, and `towncrier check` passing with a fragment and failing without one (`tool/tests/e2e/anina/records/check.sh`):

![records check](docs/img/contributing/records.svg)

## Setup

Rust (rustup.rs), then `cargo build --workspace` and `cargo test --workspace`. How Mandrake works for someone using it is in [`docs/guide/how-it-works.md`](docs/guide/how-it-works.md).
