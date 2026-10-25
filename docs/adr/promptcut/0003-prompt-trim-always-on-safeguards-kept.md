---
id: ADR-0003
title: prompt-trim is always on; the default profile keeps safeguards and strips only metadata
kind: decision
status: accepted
category: PROMPTCUT
date: 2026-10-25
last-updated: 2026-10-25
tags: [proxy, installer]
supersedes: []
superseded-by: null
related: [data/plugins/prompt-trim, cfgs/proxy, docs/guide/how-it-works.md, docs/audit/audit-2026-10-24.md]
---

# ADR-0003: prompt-trim is always on; the default profile keeps safeguards and strips only metadata

## Context

Mandrake cuts in two layers. The `prompt-trim` plugin acts inside Claude Code, before the request exists; the local proxy acts on the request body and measures. Measured on 2026-10-24, the plugin carries almost all of the saving (about 32% of the input tokens); the proxy alone removed 17 tokens. The proxy also only sees sessions opened from a shell with the Mandrake launcher, while the plugin reaches every session.

The request's top-level `safeguards` field feeds the API's safety classifier. Removing it saves a little and changes how the API judges the session.

## Considered options

### prompt-trim optional, like fancy and tui-embed

More freedom, but a profile without it keeps almost none of what Mandrake is for.

### Strip `safeguards` by default

A few tokens more per request, decided for the user without telling them.

## Decision

`prompt-trim` is always installed. The proxy's default `strip_fields` is `["metadata"]`; `safeguards` goes only when the person says so in setup, which explains what it feeds.

## Consequences

- Every installation carries the saving that matters, including sessions from the app and IDEs.
- Removing `safeguards` is a conscious choice, never a default.
- The plugin depends on the plugin API of the Claude Code version it was checked on (2.1.310); another version may not load it.
- Related, measured separately: `cut_identity` stays off, because in a long subscription session it made the request cost more (269,338 → 291,637 tokens), cause not investigated.

## References

- `docs/guide/how-it-works.md`, `docs/audit/audit-2026-10-24.md`, `docs/plan/plan-v0.md` (P2)
