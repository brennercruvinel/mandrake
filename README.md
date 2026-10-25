# Mandrake

A middle guy to cut the bullshit. Mandrake makes a Claude Code setup reproducible on any Mac or
Linux machine: it trims what the CLI wraps around your prompt, measures what that saves, and
rebuilds your `~/.claude` arrangement from one profile.

```sh
./install.sh        # build, install, interactive setup
exec $SHELL
mandrake doctor
```

| Command | Does |
| - | - |
| `mandrake setup` | wizard over every entry of `~/.claude`, plugins, proxy, souls; `--profile f --yes` applies a saved one |
| `mandrake capture` | the current `~/.claude` as a profile |
| `mandrake link plan\|apply\|rollback` | the links, with backup and journal; nothing is deleted |
| `mandrake proxy run\|install\|uninstall\|status` | the local proxy that trims and measures |
| `mandrake compare` | what Claude Code sends without and with Mandrake, side by side, part by part; answered locally, spends nothing |
| `mandrake watch` | live split view: without and with Mandrake, tokens and savings |
| `mandrake hex <id>` | one request before and after the cut, side by side in hex |
| `mandrake bench` | token savings over fixed requests; `--view` draws the report |
| `mandrake audit` | requests with bodies kept |
| `mandrake doctor` | checks links, settings, plugins, shell, proxy |
| `claude invoke <soul>` | opens a soul: core prompt + the soul's own, its settings and gh login |

## Layout

```
rust/    Rust workspace: cli (the binary), home (dirs, profile), layout (links), meter (events),
         proxy (trim, measure, service), tui (watch, hex, bench, wizard)
cfgs/    what the installer writes: setup/items.toml (the catalog), proxy/ (trim rules, service
         templates), shell/mandrake.sh (the launcher)
data/    what gets installed: plugins/ (prompt-trim, fancy, tui-embed), prompt/cores, prompt/souls,
         skills/invoke
docs/    how it works, setup, measure and audit; ATLAS.toml (architecture), adr/ (decisions),
         TERMS.md (abbreviations), CHANGELOG and news/ (its fragments)
tool/    tests outside cargo: e2e scripts and anina tapes, request fixtures
```

Read [docs/guide/how-it-works.md](docs/guide/how-it-works.md) first.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
