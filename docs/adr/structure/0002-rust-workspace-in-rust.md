---
id: ADR-0002
title: The Rust workspace lives in rust/
kind: decision
status: accepted
category: STRUCTURE
date: 2026-10-25
last-updated: 2026-10-25
tags: [packaging]
supersedes: []
superseded-by: null
related: ["#7", "#8", Cargo.toml, install.sh]
---

# ADR-0002: The Rust workspace lives in rust/

## Context

The v0 put the crates (`cli`, `home`, `layout`, `meter`, `proxy`, `tui`) under `pkgs/`. In anina and urna, `rust/` holds the crates and `pkgs/` holds distribution packages (choco, conda, nixos, wheel). The same name meaning two things across our repositories costs a reader a second look every time.

## Considered options

### Keep `pkgs/`

No move, but `pkgs/` stays taken by source code when Mandrake gets its first package.

### Move the crates to `rust/`

One rename: the workspace members, five path dependencies, `install.sh`, an e2e script and the docs.

## Decision

The crates live in `rust/`; `pkgs/` is left for distribution packages, as in anina and urna.

## Consequences

- The tree reads the same across our repositories, and a package has an obvious home.
- An existing checkout rebuilds `target/` once.
- None.

## References

- #7, #8
