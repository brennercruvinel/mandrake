---
id: ADR-0005
title: Skills reach every CLI as thin files, and sessions are read from the store, not the service
kind: decision
status: accepted
category: INSTALLER
date: 2026-11-02
last-updated: 2026-11-02
tags: [shell, souls, opencode, codex]
supersedes: []
superseded-by: null
related: [cfgs/shell/soul, "#63", "#64"]
---

# ADR-0005: Skills reach every CLI as thin files, and sessions are read from the store, not the service

## Context

Souls open in Claude Code, OpenCode and Codex, but `/invoke` and the session behind it only existed in Claude Code. Each CLI has its own commands, skills folder and session id variable, and a skill written for one uses syntax the others leave unexpanded. Checked on OpenCode 2.2.5 and Codex 0.166:

- OpenCode expands `` !`command` `` in a command with no session: `OPENCODE_SESSION_ID` is empty there and set in the shell tool. A subagent's session has an id of its own.
- `opencode api` and `opencode session export` go through the background service, starting one when they find none. With the port taken by a service they cannot see, each call took 20 s to fail. One `/sleep` made five of them, 103 s, for an answer the database gives in under a millisecond.

## Considered options

### One transpiled copy of each skill per CLI

What rulesync does. Each CLI gets native syntax, at the price of a generator and copies that go stale when the skill changes.

### One thin file per CLI that calls `soul skill`

The file says: run this with the shell tool and follow what it prints. `soul skill` renders the skill at run time, so nothing goes stale, and the shell tool is where every CLI has the session id. It costs one tool call before the skill's text arrives.

### Parent sessions through the API

The public contract, and the reason to prefer it. It starts a service as a side effect of a read, and its failure is slow.

### Parent sessions from the database, read-only

Instant, and no process left behind. It depends on a table OpenCode can rename, so a failed read must be visible.

## Decision

`soul install` writes the thin files. A file that does not carry Mandrake's stamp stays as it is. The first line of what `soul skill` prints marks it as rendered, so whoever reads a transcript does not take it as something the person typed.

The parent of an OpenCode session comes from its database, read-only, and from the API only when the database cannot be read. A lookup that fails stops with an error: treating a child as the root would act on the wrong session.

## Consequences

- A CLI gets `/invoke` on the next `mandrake setup` after it is installed; `mandrake doctor` says when one is missing.
- A skill's Claude Code tool restrictions (`allowed-tools`) do not reach the other CLIs: there, the command follows that CLI's own permissions.
- A new OpenCode schema falls back to the API, with its cost, and then to an error. It does not fall back to a wrong answer.
- Hermes and Gemini CLI are not covered: Hermes is detected by its variable but gets no files, and Gemini CLI exposes its session id only to hooks.
