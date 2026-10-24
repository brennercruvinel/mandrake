# Setup

## First run

```sh
git clone git@github.com:brennercruvinel/mandrake.git && cd mandrake
./install.sh            # builds, installs ~/.local/bin/mandrake, opens the setup wizard
exec $SHELL             # loads the shell block
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

## What setup writes outside `~/.claude`

| Where | What |
| - | - |
| `~/.zshrc` (or `.bashrc`, `.bash_profile`) | one block between `# >>> mandrake >>>` and `# <<< mandrake <<<`, replaced on every run |
| `settings.json` (through its link) | `env.ANTHROPIC_BASE_URL` and `env.CLAUDE_CODE_PLUGIN_DIRS`, nothing else; the old file goes to the backup |
| `~/Library/LaunchAgents/dev.mandrake.proxy.plist` or `~/.config/systemd/user/mandrake-proxy.service` | the proxy service |
| `~/.local/share/mandrake/` | the installed copy of `data/` and `cfgs/`, and the shell files |

## Souls

`claude invoke <soul>` opens a soul: the core prompt and the soul's prompt become the whole system
prompt, its `settings.json` is added, and `~/.config/gh-<soul>` becomes its gh login when that folder
exists. An `agents.json` in the souls folder gives extra names (`{"rhizo": ["Rhizo", "Root"]}`).
Any other `claude` call is plain Claude Code.
