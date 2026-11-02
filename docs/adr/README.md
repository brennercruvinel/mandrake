# Decision records

An ADR records a decision with trade-offs someone will ask about, or a lesson that must stay as reference, with its context. How Mandrake is built today lives in [`docs/ATLAS.toml`](../ATLAS.toml); what changed and when lives in [`docs/CHANGELOG`](../CHANGELOG).

## Writing one

1. Copy [TEMPLATE.md](TEMPLATE.md) to `docs/adr/<category>/NNNN-short-title.md`. `NNNN` is the next free number across all of `docs/adr/`, so ids stay unique; the title is kebab-case English.
2. Fill the YAML header: `id` matches the file number, `category` is the folder in capitals (`STRUCTURE` for `structure/`), `kind` is `decision` or `lesson`.
3. Keep it short: the context, the options weighed, and what the decision costs.
4. An accepted ADR is never deleted, and its body stays as written. When a new ADR replaces it, the new one lists it in `supersedes`, and only the old one's header changes: `status: superseded`, `superseded-by` and `last-updated`.

## Categories

| Folder | Scope |
| --- | --- |
| [structure/](structure/) | Crates, folders and the boundaries between them |
| `interface/` | The CLI, the screens, the plugins' commands and what a person sees |
| [promptcut/](promptcut/) | What is cut from a request, what is kept, and how the saving is measured |
| [installer/](installer/) | Setup, profiles, links, backups, rollback and the shell launcher |
| [workflows/](workflows/) | Tests, the quality gate, CI, records and how a change gets in |

A folder is created with its first ADR, and a new category enters with the ADR that needs it, together with its row here. A decision that spans categories goes in the folder of its main consequence, with the others as `tags`.

## Finding one

```sh
rg -l '^status: accepted' docs/adr     # decisions in force
rg -l '^category: PROMPTCUT' docs/adr  # one category
rg '^title:' docs/adr                  # one line per ADR
```
