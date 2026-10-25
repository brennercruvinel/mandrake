# Contributing

Keep it simple. Mandrake does three things (trim, measure, rebuild a `~/.claude`), and a change earns its place by making one of them better for someone using it.

- **Start from an issue.** One change is one issue; several independent items are an epic with one sub-issue each. The pull request starts with `Closes #<sub-issue>. Part of #<epic>.` and follows the template.
- **No agent instruction files.** No `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.cursorrules` or the like ([ADR-0004](docs/adr/workflows/0004-no-agent-instruction-files.md)). The code, `docs/`, the ADRs and the changelog are the context, for people and agents alike.
- **Code files stay under 639 lines.** Data formats (JSON, JSONL, TOML catalogs, lockfiles, SVG, logs) are exempt.
- **Run the gate before pushing:** `tool/fullcheck.sh`. CI runs the same script.
- **Add a news fragment** in `docs/news/<issue>.<type>.md`: one line, written as the changelog entry, with a type of `added`, `changed`, `removed`, `fixed` or `security`; `+<name>.<type>.md` when there is no issue. Never edit `docs/CHANGELOG` by hand. A change nobody using Mandrake would notice takes the `skip-news` label instead.
- **Write an ADR** in `docs/adr/` when a choice has trade-offs someone will ask about ([how](docs/adr/README.md)). Architecture lives in [`docs/ATLAS.toml`](docs/ATLAS.toml); the names and abbreviations in [`docs/TERMS.md`](docs/TERMS.md).
- **Show it working.** A `[POCS]` line in the pull request says where the change can be seen; test evidence from a terminal goes as anina recordings, embedded one per line.
- **Sign your commits**, in plain English. The maintainer merges, after review.

Setup: Rust (rustup.rs), then `cargo build --workspace` and `cargo test --workspace`. How Mandrake works for someone using it is in [`docs/guide/how-it-works.md`](docs/guide/how-it-works.md).
