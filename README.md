# Mandrake

A middle guy to cut the bullshit. Mandrake makes a Claude Code setup reproducible on any Mac or
Linux machine: it trims what the CLI wraps around your prompt, measures what that saves, and
rebuilds your `~/.claude` arrangement from one profile. Every demo below was recorded with
anina in a throwaway HOME, and none of them spent a request.

## Install

```sh
git clone git@github.com:brennercruvinel/mandrake.git && cd mandrake
./install.sh --cli     # builds, installs, then asks item by item what to do
exec $SHELL            # loads Mandrake in this terminal
mandrake doctor        # checks everything
```

Needs Rust ([rustup.rs](https://rustup.rs)) and Claude Code. The shell block setup writes also puts
`~/.local/bin` on `PATH`, so `mandrake` is found once `exec $SHELL` loads it. Then use `claude` as always.

### `mandrake setup`

One screen per step: your development folder, each entry of `~/.claude` (keep, link or copy),
plugins, the proxy, the prompts. Whatever is in the way goes to a backup first.

![mandrake setup](docs/img/demo/setup.svg)

## See the cut

### `mandrake compare`

One prompt through your own Claude Code, without and with Mandrake, part by part: system prompt,
tools, messages. Both requests are answered locally, so nothing is spent and no login is needed.

![mandrake compare](docs/img/demo/compare.svg)

### `mandrake watch`

Every request live as it goes through the proxy: without Mandrake on the left, with it on the right.

![mandrake watch](docs/img/demo/watch-wide.svg)

### `mandrake hex`

One request before and after the cut, byte by byte: the newest one, or `mandrake hex <id>` with an id
from `mandrake audit`.

![mandrake hex](docs/img/demo/hex-wide.svg)

### `mandrake bench --view`

The saving over fixed prompts. `mandrake bench` measures it with real requests (two short ones per
prompt); `--view` draws the last report.

![mandrake bench --view](docs/img/demo/bench-view-wide.svg)

## Everything else

| Command | Does |
| - | - |
| `mandrake doctor` | checks links, settings, plugins, shell and proxy |
| `mandrake capture` | writes the current `~/.claude` as a profile, to rebuild another machine |
| `mandrake link plan\|apply\|rollback` | the links, with backup and journal; nothing is deleted |
| `mandrake proxy run\|install\|uninstall\|status` | the local proxy that trims and measures |
| `mandrake audit` | requests whose bodies were kept |
| `claude invoke <soul>` | a session with a soul: the core prompt plus its own, its settings and gh login |

**If something breaks:** `mandrake link rollback` undoes the last apply, the originals wait in
`~/.local/state/mandrake/backups/`, and `mandrake doctor` says what is off.

## Layout

```
rust/    the Rust workspace: cli (the binary), home, layout, meter, proxy, tui
cfgs/    what the installer writes: the ~/.claude catalog, trim rules, services, the shell launcher
data/    what gets installed: plugins (prompt-trim, fancy, tui-embed), the core prompt, souls, skills
docs/    guide/ (how it works, setup, measure), adr/ (decisions), audit/ and plan/ (records),
         img/ (demos and evidence), ATLAS.toml, TERMS.md, CHANGELOG
tool/    tests outside cargo (e2e scripts, anina tapes, fixtures) and demo/ (these recordings)
```

Start with [docs/guide/how-it-works.md](docs/guide/how-it-works.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md): the flow on one screen, and what the gate checks.
