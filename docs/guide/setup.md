# Setup

## First run

```sh
git clone git@github.com:brennercruvinel/mandrake.git && cd mandrake
./install.sh            # builds, installs ~/.local/bin/mandrake, opens the setup wizard
exec $SHELL             # loads the shell block, which also puts ~/.local/bin on PATH
mandrake doctor
```

You need Rust (rustup.rs), Claude Code, and `jq` for soul aliases. Go is optional (fancy's tab tint).

The wizard asks, step by step:

1. Your development folder, the root relative link targets hang from.
2. Each entry of `~/.claude` (the catalog, `cfgs/setup/items.toml`): keep, link to a folder of
   yours, or copy. Session entries that hold conversations are flagged before you link them.
3. Plugins: prompt-trim always, fancy and tui-embed if you want them.
4. The proxy: port, whether `safeguards` goes too (off by default, since that field feeds the
   API's safety classifier), audit bodies and how long to keep them.
5. Souls folder, core prompt and the `invoke` skill with its memory command.
6. A summary of what changes, to confirm.

It saves `~/.config/mandrake/mandrake.toml` and applies it.

## What tui-embed needs

`/abtop` and `/gitlogue` open the apps in a pane through their `--embed` mode, which the upstream
releases do not have. With tui-embed chosen, setup installs both builds from the forks, with
cargo, into the prefix Mandrake goes to (`MANDRAKE_PREFIX`, else `~/.local`):

| App | Fork, branch `embed` |
| - | - |
| abtop | https://github.com/tiamatio/abtop |
| gitlogue | https://github.com/tiamatio/gitlogue |

A build already on `PATH` that answers `--embed` is kept. Without cargo, or when a build fails,
setup says which one and why, and goes on with the rest. By hand:

```sh
cargo install --git https://github.com/tiamatio/abtop --branch embed --locked --root ~/.local
cargo install --git https://github.com/tiamatio/gitlogue --branch embed --locked --root ~/.local
```

The pane spawns each app by name, so its folder must be on the `PATH` Claude Code starts with.
The shell block puts `~/.local/bin` (or `$MANDRAKE_PREFIX/bin`) there, also on macOS, which leaves
it out; otherwise set `abtop_path` and `gitlogue_path` in `/config`. `mandrake doctor` checks both: found
on `PATH`, and answering `--embed`.

## Rebuilding a machine from a profile

```sh
mandrake capture --out my.toml          # on the old machine: the current ~/.claude as a profile
./install.sh --profile my.toml          # on the new one: same links, same keys, no questions
```

`capture` records every link in `~/.claude` that points outside it, and the deepest folder their
targets share as `dev_root`, so targets stay relative and the profile survives a different mount
point. Add what a link cannot tell by hand: the souls folder, the core, the skills.

## The profile

```toml
dev_root = "~/dev/harness"

[links.skills]
action = "link"          # keep | link | copy
target = "agent/skills"  # relative to dev_root, absolute, or ~/...

[links."agents/reviewer.md"]   # any path inside ~/.claude, not only the catalog's
action = "link"
target = "agents/reviewer.md"

[proxy]
port = 8787
upstream = "https://api.anthropic.com"
strip_fields = ["metadata"]   # add "safeguards" only knowing what it feeds
audit = false                 # keep request bodies before and after, mode 600, secrets redacted
audit_keep_days = 7
count_tokens = true           # two count_tokens calls per request, off the response path

[plugins]
fancy = true
tui_embed = true

[souls]
dir = "souls"                 # each <name>/ with <NAME>.md or SOUL.md, optional settings.json
core = "none"                 # or a path; unset means data/prompt/cores/coding.md

[skills]
invoke = true
memory_cmd = "bin/memory"     # called as `<cmd> invoke <session-id>`; prints the resume block
```

## Links

```sh
mandrake link plan        # what apply would do, touching nothing
mandrake link apply       # do it; real files in the way go to the backup
mandrake link rollback    # undo the last apply from its journal
```

When a link's target does not exist yet, or is an empty folder, `apply` copies the entry there
first, so `~/.claude/agents` still shows its files through the link; the backup keeps the original.
`rollback` puts the original back and moves the filled target to `backups/<stamp>/.seeded/`, so
whatever was written through the link is kept.

## What setup writes outside `~/.claude`

| Where | What |
| - | - |
| `~/.zshrc` (or `.bashrc`, `.bash_profile`) | one block between `# >>> mandrake >>>` and `# <<< mandrake <<<`, replaced on every run |
| `settings.json` (through its link) | prompt-trim's folder joins `env.CLAUDE_CODE_PLUGIN_DIRS`, and an `env.ANTHROPIC_BASE_URL` that an older Mandrake wrote goes away; nothing else, and the old file goes to the backup |
| `~/Library/LaunchAgents/dev.mandrake.proxy.plist` or `~/.config/systemd/user/mandrake-proxy.service` | the proxy service |
| `~/.local/share/mandrake/` | the installed copy of `data/` and `cfgs/`, and the shell files |
| `~/.local/bin/abtop`, `~/.local/bin/gitlogue` | the `--embed` builds, with tui-embed only, unless ones with `--embed` are already on `PATH` |

## Next to other plugins and proxies

- `CLAUDE_CODE_PLUGIN_DIRS` is a list (Claude Code splits it on `:`). Setup puts prompt-trim's folder
  first and keeps the others in order; a prompt-trim from an older install is replaced.
- An `ANTHROPIC_BASE_URL` in `settings.json` that is not Mandrake's own port (LiteLLM, a router, a
  gateway) stays. doctor notes that those sessions skip Mandrake's proxy, and prompt-trim still runs.
  To go through both, set `upstream` in `[proxy]` to that address.
- The launcher sends a session to `127.0.0.1:<port>` only when what answers there is Mandrake's proxy
  (`GET /_mandrake` says `mandrake`); another program on the port gets nothing, and the session goes
  straight to the API with a line that says why. An `ANTHROPIC_BASE_URL` already in the shell is left
  alone.
- Plugins already on from another marketplace (`fancy@someone`) are not installed twice.

## Souls

`claude invoke <soul>` opens a soul: the core prompt and the soul's prompt become the whole system
prompt, its `settings.json` is added, and `~/.config/gh-<soul>` becomes its gh login when that folder
exists. An `agents.json` in the souls folder gives extra names (`{"rhizo": ["Rhizo", "Root"]}`).
Any other `claude` call is plain Claude Code.

## Claude Code versions

prompt-trim hooks into Claude Code's plugin API, which can change between releases. CI runs the whole
gate on each of these:

| Version | Where |
| - | - |
| 2.1.287 | Linux |
| 2.1.310 | Linux and macOS |
| the newest on npm | Linux; a failure there is a warning, the sign to check a new release |

To add a version, add it to the `claude` matrix in `.github/workflows/ci.yml` and to this table.
