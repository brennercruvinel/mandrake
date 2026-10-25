# Terms

The abbreviations used in the repository's names, one meaning each. Whole words (`proxy`, `meter`, `layout`, `relink`, `hex`) and names of tools and products (`anina`, `abtop`, `gitlogue`, `towncrier`) are not abbreviations and are not listed.

1. One abbreviation, one meaning.
2. Reuse an abbreviation already here before inventing another.
3. A new abbreviation enters this file in the same pull request that creates the name.

| Abbreviation | Meaning | Names |
| --- | --- | --- |
| `ab` | A/B comparison, without and with Mandrake | `rust/cli/src/ab.rs` |
| `adr` | architecture decision record | `docs/adr/` |
| `cfgs` | configuration files the installer writes | `cfgs/` |
| `cli` | command-line interface | `rust/cli/` |
| `cmd` | command | `rust/cli/src/cmd.rs`, `data/plugins/fancy/tint/cmd/` |
| `docs` | documentation | `docs/` |
| `e2e` | end to end: a test of the installed behavior, in a throwaway HOME | `tool/tests/e2e/`, `docs/img/e2e/` |
| `fmt` | formatting of numbers and text for display | `rust/tui/src/fmt.rs` |
| `img` | images | `docs/img/` |
| `pkgs` | distribution packages (none yet; ADR-0002) | `pkgs/` |
| `snap` | snapshot: a screen rendered to text | `rust/tui/src/snap.rs` |
| `term` | terminal | `rust/tui/src/term.rs` |
| `testutil` | test utilities | `rust/tui/src/testutil.rs` |
| `tui` | terminal user interface | `rust/tui/`, `tui-embed` |
